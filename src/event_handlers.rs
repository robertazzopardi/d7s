use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use d7s_ui::{
    handlers::{
        handle_connection_list_navigation, handle_search_filter_input,
        handle_sql_executor_input,
    },
    widgets::modal::{ModalAction, TestResult},
};

use crate::{
    app::App,
    app_state::{AppState, DatabaseExplorerState},
    services::{ConnectionService, PasswordService},
};

impl App<'_> {
    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    pub async fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key).await?;
            }
            Event::Key(_)
            | Event::FocusGained
            | Event::FocusLost
            | Event::Mouse(_)
            | Event::Paste(_)
            | Event::Resize(_, _) => {} // Ignore non-press key events
                                        // Terminal resize is handled automatically by ratatui
        }

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub async fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle search filter input first
        if self.handle_search_filter(key) {
            return Ok(());
        }

        // Handle SQL executor input
        if self.handle_sql_executor_input(key) {
            return Ok(());
        }

        // Handle modal events
        if self.modal_manager.is_any_modal_open() {
            return self.handle_modal_events(key).await;
        }

        // Handle application shortcuts (q, n, d, e, t, s, Esc, Enter)
        if self.handle_application_shortcuts(key).await? {
            return Ok(());
        }

        // Handle navigation keys (j/k/h/l, 0/$, g/G, /)
        self.handle_navigation_keys(key);

        Ok(())
    }

    /// Handle application shortcuts (q, n, d, e, t, s, Esc, Enter)
    /// Returns true if the key was handled and should stop processing
    async fn handle_application_shortcuts(
        &mut self,
        key: KeyEvent,
    ) -> Result<bool> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => {
                self.quit();
                Ok(true)
            }
            (_, KeyCode::Char('n')) => {
                if self.state == AppState::ConnectionList {
                    self.modal_manager.open_new_connection_modal();
                }
                Ok(true)
            }
            (_, KeyCode::Char('d')) => {
                if self.state == AppState::ConnectionList {
                    self.handle_delete_connection();
                    return Ok(true);
                }
                Ok(false)
            }
            (_, KeyCode::Char('e')) => {
                if self.state == AppState::ConnectionList {
                    self.handle_edit_connection();
                    return Ok(true);
                }
                Ok(false)
            }
            (_, KeyCode::Char('t')) => {
                if self.state == AppState::DatabaseConnected {
                    self.handle_toggle_table_view().await?;
                }
                Ok(true)
            }
            (_, KeyCode::Char('s')) => {
                if self.state == AppState::DatabaseConnected {
                    self.enter_sql_executor_mode();
                }
                Ok(true)
            }
            (_, KeyCode::Esc) => {
                if self.modal_manager.is_any_modal_open() {
                    self.modal_manager.close_active_modal();
                } else if self.state == AppState::DatabaseConnected {
                    let is_sql_executor =
                        self.database_explorer.as_ref().is_some_and(|e| {
                            matches!(
                                e.state,
                                DatabaseExplorerState::SqlExecutor
                            )
                        });

                    if is_sql_executor {
                        // Restore the previous state before SQL executor was opened
                        self.sql_executor.deactivate();
                        if let Some(explorer) = &mut self.database_explorer
                            && let Some(previous_state) =
                                explorer.previous_state.take()
                        {
                            explorer.state = previous_state;
                        }
                    } else if self.has_active_filter() {
                        self.clear_filter();
                    } else {
                        self.go_back_in_database();
                    }
                }
                Ok(true)
            }
            (_, KeyCode::Enter) => {
                if self.state == AppState::ConnectionList {
                    self.connect_to_database().await?;
                } else if self.state == AppState::DatabaseConnected {
                    // Handle database navigation
                    self.handle_database_navigation().await?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Handle navigation keys (j/k/h/l, 0/$, g/G, /)
    fn handle_navigation_keys(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            // Vim keybindings for table navigation
            (_, KeyCode::Char('j') | KeyCode::Down) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Down,
                        &mut self.connections.table,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Down);
                }
            }
            (_, KeyCode::Char('k') | KeyCode::Up) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Up,
                        &mut self.connections.table,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Up);
                }
            }
            (_, KeyCode::Char('h' | 'b') | KeyCode::Left) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Left,
                        &mut self.connections.table,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Left);
                }
            }
            (_, KeyCode::Char('l' | 'w') | KeyCode::Right) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Right,
                        &mut self.connections.table,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Right);
                }
            }
            // Jump to edges
            (_, KeyCode::Char('0')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('0'),
                        &mut self.connections.table,
                    );
                }
            }
            (_, KeyCode::Char('$')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('$'),
                        &mut self.connections.table,
                    );
                }
            }
            (_, KeyCode::Char('g')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('g'),
                        &mut self.connections.table,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Char('g'));
                }
            }
            (_, KeyCode::Char('G')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('G'),
                        &mut self.connections.table,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Char('G'));
                }
            }
            (_, KeyCode::Char('/')) => {
                if !self.modal_manager.is_any_modal_open() {
                    self.search_filter.activate();
                }
            }
            _ => {}
        }
    }

    /// Handle search filter input
    fn handle_search_filter(&mut self, key: KeyEvent) -> bool {
        if !self.search_filter.is_active {
            return false;
        }

        let mut should_clear = false;
        let mut should_apply = false;
        let filter_handled = handle_search_filter_input(
            key,
            &mut self.search_filter,
            &mut || {
                if key.code == KeyCode::Esc {
                    should_clear = true;
                } else {
                    should_apply = true;
                }
            },
        );

        if filter_handled {
            if should_clear {
                self.clear_filter();
            } else if should_apply {
                self.apply_filter();
            }
            true
        } else {
            false
        }
    }

    /// Handle SQL executor input
    fn handle_sql_executor_input(&mut self, key: KeyEvent) -> bool {
        let is_sql_executor =
            self.database_explorer.as_ref().is_some_and(|e| {
                matches!(e.state, DatabaseExplorerState::SqlExecutor)
            });

        is_sql_executor
            && handle_sql_executor_input(key, &mut self.sql_executor)
    }

    /// Handle delete connection action
    fn handle_delete_connection(&mut self) {
        let Some(connection) = self.get_selected_connection() else {
            return;
        };
        let message = format!(
            "Are you sure you want to delete\nthe connection '{}'?\n\nThis action cannot be undone.",
            connection.name
        );
        self.modal_manager
            .open_confirmation_modal(message, connection.clone());
    }

    /// Handle edit connection action
    fn handle_edit_connection(&mut self) {
        let Some(connection) = self.get_selected_connection() else {
            return;
        };
        let password =
            crate::services::PasswordService::get_connection_password(
                connection,
            );
        let connection = connection.clone();
        self.modal_manager
            .open_edit_connection_modal(&connection, password);
    }

    /// Handle toggle between table data and columns view
    async fn handle_toggle_table_view(&mut self) -> Result<()> {
        let state = self.database_explorer.as_ref().map(|e| e.state.clone());

        match state {
            Some(DatabaseExplorerState::TableData(schema_name, table_name)) => {
                if let Err(e) =
                    self.load_columns(&schema_name, &table_name).await
                {
                    self.set_status(format!("Failed to load columns: {e}"));
                }
            }
            Some(DatabaseExplorerState::Columns(schema_name, table_name)) => {
                if let Err(e) =
                    self.load_table_data(&schema_name, &table_name).await
                {
                    self.set_status(format!("Failed to load table data: {e}"));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Enter SQL executor mode
    fn enter_sql_executor_mode(&mut self) {
        if let Some(explorer) = &mut self.database_explorer {
            // Save the current state before entering SQL executor
            explorer.previous_state = Some(explorer.state.clone());
            explorer.state = DatabaseExplorerState::SqlExecutor;
        }
        self.sql_executor.activate();
    }

    /// Handle modal events
    pub async fn handle_modal_events(&mut self, key: KeyEvent) -> Result<()> {
        let action = self.modal_manager.handle_key_events_ui(key);

        match action {
            ModalAction::Save => {
                if self.handle_password_modal_save().await? {
                    return Ok(());
                }
                self.handle_connection_modal_save();
            }
            ModalAction::Test => {
                self.handle_connection_modal_test().await;
            }
            ModalAction::Cancel => {
                if self.modal_manager.was_connection_modal_closed() {
                    self.refresh_connections();
                }
            }
            ModalAction::None => {}
        }

        // Handle confirmation modal results
        if let Some(connection) =
            self.modal_manager.was_confirmation_modal_confirmed()
        {
            // Delete from keyring if not using "ask every time"
            if !connection.should_ask_every_time() {
                let _ = PasswordService::delete_from_keyring(&connection.name);
            }

            // Delete connection using ConnectionService
            if let Err(e) = ConnectionService::delete(&connection.name) {
                self.set_status(format!("Failed to delete connection: {e}"));
            } else {
                self.refresh_connections();
            }
        }

        // Clean up closed modals
        self.modal_manager.cleanup_closed_modals();

        Ok(())
    }

    /// Handle password modal save action
    async fn handle_password_modal_save(&mut self) -> Result<bool> {
        // Extract data from modal before attempting connection
        // This releases the mutable borrow so we can call connect_with_password
        let (connection, password) = {
            let Some(password_modal) =
                self.modal_manager.get_password_modal_mut()
            else {
                return Ok(false);
            };

            let Some(connection) = password_modal.connection.clone() else {
                return Ok(false);
            };

            (connection, password_modal.password.clone())
        };

        // Store the state before attempting connection to check if it changed
        let state_before = self.state.clone();

        // Try to connect with the password (don't store in session yet)
        let password_clone = password.clone();
        let _ = self
            .connect_with_password(connection.clone(), password_clone)
            .await;

        // Check if connection succeeded by checking if state changed to DatabaseConnected
        if self.state == AppState::DatabaseConnected
            && state_before != AppState::DatabaseConnected
        {
            // Connection succeeded, store password in session and close the modal
            self.password_service
                .store_session_password(&connection, password);
            if let Some(password_modal) =
                self.modal_manager.get_password_modal_mut()
            {
                password_modal.close();
            }
        } else {
            // Connection failed, keep modal open so user can retry
            // Remove any password from session that might have been stored
            self.password_service.remove_session_password(&connection);
            // The error message is already set by connect_with_password
            // Clear the password field so user can retry
            if let Some(password_modal) =
                self.modal_manager.get_password_modal_mut()
            {
                password_modal.clear_password();
            }
        }
        Ok(true)
    }

    /// Handle connection modal save action
    fn handle_connection_modal_save(&mut self) {
        let Some(modal) = self.modal_manager.get_connection_modal_mut() else {
            return;
        };

        let Some(connection) = modal.get_connection() else {
            return;
        };

        let original_name = modal.original_name.clone();

        if let Err(error_msg) = ConnectionService::validate(&connection) {
            modal.test_result = TestResult::Failed(error_msg);
            return;
        }

        if connection.uses_keyring()
            && let Some(ref password) = connection.password
            && let Err(e) =
                PasswordService::save_to_keyring(&connection.name, password)
        {
            modal.test_result =
                TestResult::Failed(format!("Failed to save password: {e}"));
            return;
        }

        let save_result = original_name.as_ref().map_or_else(
            || ConnectionService::create(&connection),
            |orig_name| ConnectionService::update(orig_name, &connection),
        );

        match save_result {
            Ok(()) => {
                modal.close();
                self.refresh_connections();
            }
            Err(e) => {
                modal.test_result = TestResult::Failed(e.to_string());
            }
        }
    }

    /// Handle connection modal test action
    async fn handle_connection_modal_test(&mut self) {
        let Some(modal) = self.modal_manager.get_connection_modal_mut() else {
            return;
        };

        let Some(connection) = modal.get_connection() else {
            modal.test_result =
                TestResult::Failed("Please fill in all fields".to_string());
            return;
        };

        modal.test_result = TestResult::Testing;
        let success = ConnectionService::test(&connection).await;
        modal.test_result = if success {
            TestResult::Success
        } else {
            TestResult::Failed("Connection failed".to_string())
        };
    }
}
