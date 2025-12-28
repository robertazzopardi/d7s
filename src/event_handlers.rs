use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use d7s_ui::{
    handlers::{handle_connection_list_navigation, handle_search_filter_input, handle_sql_executor_input},
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
            Event::Key(_) => {} // Ignore non-press key events
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {} // Terminal resize is handled automatically by ratatui
        }

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    #[allow(clippy::too_many_lines)]
    pub async fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle search filter input first
        if self.search_filter.is_active {
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
                return Ok(());
            }
        }

        // Handle SQL executor input
        let is_sql_executor = self
            .database_explorer
            .as_ref()
            .map(|e| matches!(e.state, DatabaseExplorerState::SqlExecutor))
            .unwrap_or(false);

        if is_sql_executor && handle_sql_executor_input(key, &mut self.sql_executor) {
            return Ok(());
        }

        // Handle modal events first
        if self.modal_manager.is_any_modal_open() {
            return self.handle_modal_events(key).await;
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('n')) => {
                if self.state == AppState::ConnectionList
                    && !self.modal_manager.is_any_modal_open()
                {
                    // Don't initialize keyring yet - it will be initialized when the user fills in the form
                    self.modal_manager.open_new_connection_modal();
                }
            }
            (_, KeyCode::Char('d')) => {
                // Only handle 'd' key if no modals are open and in connection list
                if self.state == AppState::ConnectionList
                    && !self.modal_manager.is_any_modal_open()
                    && let Some(connection) = self.get_selected_connection()
                {
                    let message = format!(
                        "Are you sure you want to delete\nthe connection '{}'?\n\nThis action cannot be undone.",
                        connection.name
                    );
                    self.modal_manager
                        .open_confirmation_modal(message, connection.clone());
                    return Ok(()); // Return early to prevent key propagation
                }
            }
            (_, KeyCode::Char('e')) => {
                // Only handle 'e' key if no modal is open and in connection list
                if self.state == AppState::ConnectionList
                    && !self.modal_manager.is_any_modal_open()
                    && let Some(connection) = self.get_selected_connection()
                {
                    let password = self.get_connection_password(connection);
                    let connection = connection.clone();
                    self.modal_manager
                        .open_edit_connection_modal(&connection, password);
                    return Ok(()); // Return early to prevent key propagation
                }
            }
            (_, KeyCode::Char('p')) => self.toggle_popup(),
            (_, KeyCode::Char('t')) => {
                if self.state == AppState::DatabaseConnected {
                    if let Some(explorer) = &self.database_explorer {
                        if let DatabaseExplorerState::TableData(schema_name, table_name) = &explorer.state {
                            // Toggle to columns view
                            let schema_name = schema_name.clone();
                            let table_name = table_name.clone();
                            if let Err(e) = self.load_columns(&schema_name, &table_name).await {
                                self.set_status(format!("Failed to load columns: {e}"));
                            }
                        } else if let DatabaseExplorerState::Columns(schema_name, table_name) = &explorer.state {
                            // Toggle to data view
                            let schema_name = schema_name.clone();
                            let table_name = table_name.clone();
                            if let Err(e) = self.load_table_data(&schema_name, &table_name).await {
                                self.set_status(format!("Failed to load table data: {e}"));
                            }
                        }
                    }
                }
            }
            (_, KeyCode::Char('s')) => {
                if self.state == AppState::DatabaseConnected {
                    // Enter SQL execution mode
                    if let Some(explorer) = &mut self.database_explorer {
                        explorer.state = DatabaseExplorerState::SqlExecutor;
                    }
                    self.sql_executor.activate();
                }
            }
            (_, KeyCode::Esc) => {
                if self.show_popup {
                    self.toggle_popup();
                } else if self.modal_manager.is_any_modal_open() {
                    self.modal_manager.close_active_modal();
                } else if self.state == AppState::DatabaseConnected {
                    let is_sql_executor = self
                        .database_explorer
                        .as_ref()
                        .map(|e| matches!(e.state, DatabaseExplorerState::SqlExecutor))
                        .unwrap_or(false);

                    if is_sql_executor {
                        // Deactivate SQL executor and go back to schemas
                        self.sql_executor.deactivate();
                    }

                    self.go_back_in_database();
                }
                return Ok(());
            }
            (_, KeyCode::Enter) => {
                if self.state == AppState::ConnectionList {
                    self.connect_to_database().await?;
                } else if self.state == AppState::DatabaseConnected {
                    // Handle database navigation
                    self.handle_database_navigation().await?;
                }
                return Ok(());
            }
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
            // Add other key handlers here.
            _ => {}
        }

        Ok(())
    }

    /// Handle modal events
    #[allow(clippy::too_many_lines)]
    pub async fn handle_modal_events(&mut self, key: KeyEvent) -> Result<()> {
        // Handle modal events (UI only)
        let action = self.modal_manager.handle_key_events_ui(key);

        // Handle business logic based on modal actions
        match action {
            ModalAction::Save => {
                // Handle password modal save (only used for "ask every time" connections)
                if let Some(password_modal) = self.modal_manager.get_password_modal_mut() {
                    let Some(connection) = &password_modal.connection.clone() else {
                        return Ok(());
                    };

                    let password = password_modal.password.clone();
                    password_modal.close();

                    // Store in session memory via PasswordService
                    self.password_service
                        .store_session_password(connection, password.clone());

                    // Connect with the provided password
                    if let Err(e) = self
                        .connect_with_password(connection.clone(), password)
                        .await
                    {
                        self.set_status(format!("Failed to connect: {e}"));
                    }
                    return Ok(());
                }

                // Handle connection modal save
                let Some(modal) = self.modal_manager.get_connection_modal_mut() else {
                    return Ok(());
                };
                let Some(connection) = modal.get_connection() else {
                    return Ok(());
                };

                let original_name = modal.original_name.clone();

                // Validate the connection
                if let Err(error_msg) = ConnectionService::validate(&connection) {
                    modal.test_result = TestResult::Failed(error_msg);
                    return Ok(());
                }

                // Save password to keyring if using keyring storage and password is provided
                if connection.uses_keyring() {
                    if let Some(ref password) = connection.password {
                        if let Err(e) = PasswordService::save_to_keyring(&connection.name, password) {
                            modal.test_result = TestResult::Failed(format!("Failed to save password: {e}"));
                            return Ok(());
                        }
                    }
                }

                // Save the connection using ConnectionService
                let save_result = if let Some(ref orig_name) = original_name {
                    ConnectionService::update(orig_name, &connection)
                } else {
                    ConnectionService::create(&connection)
                };

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
            ModalAction::Test => {
                let Some(modal) = self.modal_manager.get_connection_modal_mut() else {
                    return Ok(());
                };

                let Some(connection) = modal.get_connection() else {
                    modal.test_result = TestResult::Failed(
                        "Please fill in all fields".to_string(),
                    );
                    return Ok(());
                };

                modal.test_result = TestResult::Testing;
                // Use ConnectionService to test the connection
                let success = ConnectionService::test(&connection).await;
                modal.test_result = if success {
                    TestResult::Success
                } else {
                    TestResult::Failed("Connection failed".to_string())
                };
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
            if let Err(e) = ConnectionService::delete(&connection.name)
            {
                self.set_status(format!("Failed to delete connection: {e}"));
            } else {
                self.refresh_connections();
            }
        }

        // Clean up closed modals
        self.modal_manager.cleanup_closed_modals();

        Ok(())
    }
}
