use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use d7s_db::{TableData, connection::Connection};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::*,
    widgets::{Block, Borders},
};

use crate::widgets::{
    hotkey::Hotkey,
    modal::{ConfirmationModal, Modal, ModalManager, Mode},
    table::DataTable,
    top_bar_view::{CONNECTION_HOTKEYS, TopBarView},
};

pub const APP_NAME: &str = r"
_________________        
\______ \______  \______ 
 |    |  \  /    /  ___/ 
 |    `   \/    /\___ \  
/_______  /____//____  > 
        \/           \/  
";

/// The main application which holds the state and logic of the application.
pub struct App<'a> {
    /// Is the application running?
    running: bool,
    show_popup: bool,
    modal_manager: ModalManager,
    hotkeys: Vec<Hotkey<'a>>,
    table_widget: DataTable<Connection>,
}

impl Default for App<'_> {
    fn default() -> Self {
        d7s_db::sqlite::init_db().unwrap();

        let items = d7s_db::sqlite::get_connections().unwrap_or_default();

        let table_widget = DataTable::new(items);

        Self {
            running: false,
            show_popup: false,
            modal_manager: ModalManager::new(),
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
            table_widget,
        }
    }
}

impl App<'_> {
    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(13),
                Constraint::Percentage(87),
            ])
            .split(frame.area());

        let current_connection = Connection::default();
        frame.render_widget(TopBarView { current_connection }, layout[0]);

        // Create the inner block
        let block = Block::new()
            .borders(Borders::ALL)
            .title(" Connections ")
            .title_alignment(Alignment::Center);

        // Get the inner area of the block (content area)
        let inner_area = block.inner(layout[1]);

        // Render the block itself (borders and title)
        frame.render_widget(block, layout[1]);

        // Render content inside the block
        // Use the data table directly
        frame.render_stateful_widget(
            self.table_widget.clone(),
            inner_area,
            &mut self.table_widget.table_state,
        );

        // Render modals using the modal manager
        if let Some(modal) = self.modal_manager.get_connection_modal() {
            frame.render_widget(modal.clone(), frame.area());
        }

        if let Some(confirmation_modal) =
            self.modal_manager.get_confirmation_modal()
        {
            frame.render_widget(confirmation_modal.clone(), frame.area());
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle modal events first
        if self.modal_manager.is_any_modal_open() {
            self.modal_manager.handle_key_events(key).await?;

            // Handle confirmation modal results
            if let Some(Some(connection)) =
                self.modal_manager.was_confirmation_modal_confirmed()
            {
                // Delete the keyring credential using the user
                let entry = keyring::Entry::new("d7s", &connection.user)?;
                entry.delete_credential()?;

                // Delete from database
                if let Err(e) =
                    d7s_db::sqlite::delete_connection(&connection.name)
                {
                    eprintln!("Failed to delete connection: {}", e);
                } else {
                    self.refresh_connections();
                }
            }

            // Handle connection modal closure
            if self.modal_manager.was_connection_modal_closed() {
                self.refresh_connections();
            }

            // Clean up closed modals
            self.modal_manager.cleanup_closed_modals();

            // If escape was pressed, don't handle it again in the main key matching
            if key.code == KeyCode::Esc {
                return Ok(());
            }
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('n')) => {
                if !self.modal_manager.is_any_modal_open() {
                    self.modal_manager.open_new_connection_modal();
                }
            }
            (_, KeyCode::Char('d')) => {
                // Only handle 'd' key if no modals are open
                if !self.modal_manager.is_any_modal_open()
                    && let Some(selected_index) =
                        self.table_widget.table_state.selected()
                {
                    // Account for header row - selected_index 0 is header, 1+ are data rows
                    let data_index = selected_index.saturating_sub(1);
                    if data_index < self.table_widget.items.len()
                        && let Some(connection) =
                            self.table_widget.items.get(data_index)
                    {
                        let message = format!(
                            "Are you sure you want to delete\nthe connection '{}'?\n\nThis action cannot be undone.",
                            connection.name
                        );
                        self.modal_manager.open_confirmation_modal(
                            message,
                            connection.clone(),
                        );
                        return Ok(()); // Return early to prevent key propagation
                    }
                }
            }
            (_, KeyCode::Char('e')) => {
                // Only handle 'e' key if no modal is open
                if !self.modal_manager.is_any_modal_open()
                    && let Some(selected_index) =
                        self.table_widget.table_state.selected()
                {
                    // Account for header row - selected_index 0 is header, 1+ are data rows
                    let data_index = selected_index.saturating_sub(1);
                    if data_index < self.table_widget.items.len()
                        && let Some(connection) =
                            self.table_widget.items.get(data_index)
                    {
                        if let Err(e) = self
                            .modal_manager
                            .open_edit_connection_modal(connection)
                        {
                            eprintln!("Failed to open edit modal: {}", e);
                        }
                        return Ok(()); // Return early to prevent key propagation
                    }
                }
            }
            (_, KeyCode::Char('p')) => self.toggle_popup(),
            (_, KeyCode::Esc) => {
                if self.show_popup {
                    self.toggle_popup();
                } else if self.modal_manager.is_any_modal_open() {
                    self.modal_manager.close_active_modal();
                }
                return Ok(());
            }
            // Vim keybindings for table navigation
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                self.table_widget.table_state.select_next();
            }
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.table_widget.table_state.select_previous();
            }
            (_, KeyCode::Char('h'))
            | (_, KeyCode::Left)
            | (_, KeyCode::Char('b')) => {
                self.table_widget.table_state.select_previous_column();
            }
            (_, KeyCode::Char('l'))
            | (_, KeyCode::Right)
            | (_, KeyCode::Char('w')) => {
                self.table_widget.table_state.select_next_column();
            }
            // Jump to edges
            (_, KeyCode::Char('0')) => {
                self.table_widget.table_state.select_column(Some(0));
            }
            (_, KeyCode::Char('$')) => {
                if let Some(num_cols) = self
                    .table_widget
                    .items
                    .first()
                    .map(d7s_db::TableData::num_columns)
                {
                    self.table_widget
                        .table_state
                        .select_column(Some(num_cols.saturating_sub(1)));
                }
            }
            (_, KeyCode::Char('g')) => {
                self.table_widget.table_state.select(Some(1)); // First data row
            }
            (_, KeyCode::Char('G')) => {
                if !self.table_widget.items.is_empty() {
                    self.table_widget
                        .table_state
                        .select(Some(self.table_widget.items.len()));
                }
            }
            // Add other key handlers here.
            _ => {}
        }

        Ok(())
    }

    const fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    /// Refresh the table data from the database
    fn refresh_connections(&mut self) {
        if let Ok(connections) = d7s_db::sqlite::get_connections() {
            self.table_widget.items = connections;
        }
    }

    /// Set running to false to quit the application.
    const fn quit(&mut self) {
        self.running = false;
    }
}
