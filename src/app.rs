use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use d7s_db::{
    Column, Database, DatabaseObjectType, Schema, Table, TableData,
    connection::Connection, postgres::Postgres,
};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::*,
    widgets::{Block, Borders},
};

use crate::widgets::{
    hotkey::Hotkey,
    modal::{ConfirmationModal, Modal, ModalManager, Mode},
    table::{DataTable, TableDataWidget},
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

/// Application state to track whether we're viewing connections or connected to a database
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    ConnectionList,
    DatabaseConnected,
}

/// Database explorer state to track what object type is being viewed
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseExplorerState {
    Schemas,
    Tables(String),            // schema name
    Columns(String, String),   // schema name, table name
    TableData(String, String), // schema name, table name
}

/// The main application which holds the state and logic of the application.
pub struct App<'a> {
    /// Is the application running?
    running: bool,
    show_popup: bool,
    modal_manager: ModalManager,
    hotkeys: Vec<Hotkey<'a>>,
    table_widget: DataTable<Connection>,
    /// Current application state
    state: AppState,
    /// Active database connection when connected
    active_connection: Option<Connection>,
    /// Active database client when connected
    active_database: Option<Postgres>,
    /// Current database explorer state
    explorer_state: Option<DatabaseExplorerState>,
    /// Current schema table data
    schema_table: Option<DataTable<Schema>>,
    /// Current table table data
    table_table: Option<DataTable<Table>>,
    /// Current column table data
    column_table: Option<DataTable<Column>>,
    /// Current table data (rows)
    table_data: Option<TableDataWidget>,
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
            state: AppState::ConnectionList,
            active_connection: None,
            active_database: None,
            explorer_state: None,
            schema_table: None,
            table_table: None,
            column_table: None,
            table_data: None,
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

        let current_connection =
            self.active_connection.clone().unwrap_or_default();
        frame.render_widget(TopBarView { current_connection }, layout[0]);

        match self.state {
            AppState::ConnectionList => {
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
            }
            AppState::DatabaseConnected => {
                // Create the inner block for database view
                let block = Block::new()
                    .borders(Borders::ALL)
                    .title(self.get_database_title())
                    .title_alignment(Alignment::Center);

                // Get the inner area of the block (content area)
                let inner_area = block.inner(layout[1]);

                // Render the block itself (borders and title)
                frame.render_widget(block, layout[1]);

                // Render the appropriate table based on explorer state
                self.render_database_table(frame, inner_area);
            }
        }

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
                let _ = entry.delete_credential();

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
            // if key.code == KeyCode::Esc {
            return Ok(());
            // }
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('n')) => {
                if self.state == AppState::ConnectionList
                    && !self.modal_manager.is_any_modal_open()
                {
                    self.modal_manager.open_new_connection_modal();
                }
            }
            (_, KeyCode::Char('d')) => {
                // Only handle 'd' key if no modals are open and in connection list
                if self.state == AppState::ConnectionList
                    && !self.modal_manager.is_any_modal_open()
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
                // Only handle 'e' key if no modal is open and in connection list
                if self.state == AppState::ConnectionList
                    && !self.modal_manager.is_any_modal_open()
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
            (_, KeyCode::Char('t')) => {
                if self.state == AppState::DatabaseConnected {
                    if let Some(DatabaseExplorerState::TableData(
                        schema_name,
                        table_name,
                    )) = &self.explorer_state
                    {
                        // Toggle to columns view
                        let schema_name = schema_name.clone();
                        let table_name = table_name.clone();
                        if let Err(e) =
                            self.load_columns(&schema_name, &table_name).await
                        {
                            eprintln!("Failed to load columns: {}", e);
                        }
                    } else if let Some(DatabaseExplorerState::Columns(
                        schema_name,
                        table_name,
                    )) = &self.explorer_state
                    {
                        // Toggle to data view
                        let schema_name = schema_name.clone();
                        let table_name = table_name.clone();
                        if let Err(e) = self
                            .load_table_data(&schema_name, &table_name)
                            .await
                        {
                            eprintln!("Failed to load table data: {}", e);
                        }
                    }
                }
            }
            (_, KeyCode::Esc) => {
                if self.show_popup {
                    self.toggle_popup();
                } else if self.modal_manager.is_any_modal_open() {
                    self.modal_manager.close_active_modal();
                } else if self.state == AppState::DatabaseConnected {
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
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                if self.state == AppState::ConnectionList {
                    self.table_widget.table_state.select_next();
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Down);
                }
            }
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                if self.state == AppState::ConnectionList {
                    self.table_widget.table_state.select_previous();
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Up);
                }
            }
            (_, KeyCode::Char('h'))
            | (_, KeyCode::Left)
            | (_, KeyCode::Char('b')) => {
                if self.state == AppState::ConnectionList {
                    self.table_widget.table_state.select_previous_column();
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Left);
                }
            }
            (_, KeyCode::Char('l'))
            | (_, KeyCode::Right)
            | (_, KeyCode::Char('w')) => {
                if self.state == AppState::ConnectionList {
                    self.table_widget.table_state.select_next_column();
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Right);
                }
            }
            // Jump to edges
            (_, KeyCode::Char('0')) => {
                if self.state == AppState::ConnectionList {
                    self.table_widget.table_state.select_column(Some(0));
                }
            }
            (_, KeyCode::Char('$')) => {
                if self.state == AppState::ConnectionList {
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
            }
            (_, KeyCode::Char('g')) => {
                if self.state == AppState::ConnectionList {
                    self.table_widget.table_state.select(Some(1)); // First data row
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Char('g'));
                }
            }
            (_, KeyCode::Char('G')) => {
                if self.state == AppState::ConnectionList
                    && !self.table_widget.items.is_empty()
                {
                    self.table_widget
                        .table_state
                        .select(Some(self.table_widget.items.len()));
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Char('G'));
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

    /// Connect to the selected database
    async fn connect_to_database(&mut self) -> Result<()> {
        if let Some(selected_index) = self.table_widget.table_state.selected() {
            // Account for header row - selected_index 0 is header, 1+ are data rows
            let data_index = selected_index.saturating_sub(1);
            if data_index < self.table_widget.items.len() {
                if let Some(connection) =
                    self.table_widget.items.get(data_index)
                {
                    // Get the password from keyring
                    let entry = keyring::Entry::new("d7s", &connection.user)?;
                    let password = entry.get_password()?;

                    // Create connection with password
                    let mut connection_with_password = connection.clone();
                    connection_with_password.password = Some(password);

                    // Test the connection first
                    let postgres = connection_with_password.to_postgres();
                    if postgres.test().await {
                        // Connection successful, update state
                        self.active_connection =
                            Some(connection_with_password.clone());
                        self.active_database = Some(postgres);
                        self.state = AppState::DatabaseConnected;

                        // Load schemas after successful connection
                        self.load_schemas().await?;
                    } else {
                        eprintln!(
                            "Failed to connect to database: {}",
                            connection.name
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Disconnect from the current database
    fn disconnect_from_database(&mut self) {
        self.active_connection = None;
        self.active_database = None;
        self.state = AppState::ConnectionList;
        self.explorer_state = None;
        self.schema_table = None;
        self.table_table = None;
        self.column_table = None;
        self.table_data = None;
    }

    /// Get the title for the database view based on current state
    fn get_database_title(&self) -> String {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) => " Schemas ".to_string(),
            Some(DatabaseExplorerState::Tables(schema)) => {
                format!(" Tables in {} ", schema)
            }
            Some(DatabaseExplorerState::Columns(schema, table)) => {
                format!(" Columns in {}.{} ", schema, table)
            }
            Some(DatabaseExplorerState::TableData(schema, table)) => {
                format!(" Data in {}.{} ", schema, table)
            }
            None => " Database Explorer ".to_string(),
        }
    }

    /// Render the appropriate database table based on explorer state
    fn render_database_table(&mut self, frame: &mut Frame, area: Rect) {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                if let Some(schema_table) = &self.schema_table {
                    frame.render_stateful_widget(
                        schema_table.clone(),
                        area,
                        &mut schema_table.table_state.clone(),
                    );
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                if let Some(table_table) = &self.table_table {
                    frame.render_stateful_widget(
                        table_table.clone(),
                        area,
                        &mut table_table.table_state.clone(),
                    );
                }
            }
            Some(DatabaseExplorerState::Columns(_, _)) => {
                if let Some(column_table) = &self.column_table {
                    frame.render_stateful_widget(
                        column_table.clone(),
                        area,
                        &mut column_table.table_state.clone(),
                    );
                }
            }
            Some(DatabaseExplorerState::TableData(_, _)) => {
                if let Some(table_data) = &self.table_data {
                    frame.render_stateful_widget(
                        table_data.clone(),
                        area,
                        &mut table_data.table_state.clone(),
                    );
                }
            }
            None => {
                // Show schemas by default
                if let Some(schema_table) = &self.schema_table {
                    frame.render_stateful_widget(
                        schema_table.clone(),
                        area,
                        &mut schema_table.table_state.clone(),
                    );
                }
            }
        }
    }

    /// Load schemas from the database
    async fn load_schemas(&mut self) -> Result<()> {
        if let Some(database) = &self.active_database {
            match database.get_schemas().await {
                Ok(schemas) => {
                    self.schema_table = Some(DataTable::new(schemas));
                    self.explorer_state = Some(DatabaseExplorerState::Schemas);
                }
                Err(e) => {
                    eprintln!("Failed to load schemas: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Load tables for a schema
    async fn load_tables(&mut self, schema_name: &str) -> Result<()> {
        if let Some(database) = &self.active_database {
            match database.get_tables(schema_name).await {
                Ok(tables) => {
                    self.table_table = Some(DataTable::new(tables));
                    self.explorer_state = Some(DatabaseExplorerState::Tables(
                        schema_name.to_string(),
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to load tables: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Load columns for a table
    async fn load_columns(
        &mut self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        if let Some(database) = &self.active_database {
            match database.get_columns(schema_name, table_name).await {
                Ok(columns) => {
                    self.column_table = Some(DataTable::new(columns));
                    self.explorer_state = Some(DatabaseExplorerState::Columns(
                        schema_name.to_string(),
                        table_name.to_string(),
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to load columns: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Handle database navigation when Enter is pressed
    async fn handle_database_navigation(&mut self) -> Result<()> {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                // Navigate to tables in selected schema
                if let Some(schema_table) = &self.schema_table {
                    if let Some(selected_index) =
                        schema_table.table_state.selected()
                    {
                        if selected_index < schema_table.items.len() {
                            if let Some(schema) =
                                schema_table.items.get(selected_index)
                            {
                                let schema_name = schema.name.clone();

                                if let Some(connection) =
                                    &mut self.active_connection
                                {
                                    connection.schema =
                                        Some(schema_name.clone());
                                }

                                self.load_tables(&schema_name).await?;
                            }
                        }
                    }
                }
            }
            Some(DatabaseExplorerState::Tables(schema_name)) => {
                // Navigate to table data in selected table (show data first, not columns)
                if let Some(table_table) = &self.table_table {
                    if let Some(selected_index) =
                        table_table.table_state.selected()
                    {
                        let data_index = selected_index;
                        if data_index < table_table.items.len() {
                            if let Some(table) =
                                table_table.items.get(data_index)
                            {
                                let schema_name = schema_name.clone();
                                let table_name = table.name.clone();

                                if let Some(connection) =
                                    &mut self.active_connection
                                {
                                    connection.table = Some(table_name.clone());
                                }

                                self.load_table_data(&schema_name, &table_name)
                                    .await?;
                            }
                        }
                    }
                }
            }
            Some(DatabaseExplorerState::Columns(schema_name, table_name)) => {
                // Toggle to data view
                let schema_name = schema_name.clone();
                let table_name = table_name.clone();
                self.load_table_data(&schema_name, &table_name).await?;
            }
            Some(DatabaseExplorerState::TableData(schema_name, table_name)) => {
                // Toggle back to columns view
                self.explorer_state = Some(DatabaseExplorerState::Columns(
                    schema_name.clone(),
                    table_name.clone(),
                ));
            }
            None => {
                // Load schemas if not loaded yet
                self.load_schemas().await?;
            }
        }
        Ok(())
    }

    /// Go back to previous level in database navigation
    fn go_back_in_database(&mut self) {
        match &self.explorer_state {
            Some(DatabaseExplorerState::TableData(schema_name, _)) => {
                // Go back to tables in the same schema
                if let Some(table_table) = &self.table_table {
                    self.explorer_state = Some(DatabaseExplorerState::Tables(
                        schema_name.clone(),
                    ));

                    if let Some(connection) = &mut self.active_connection {
                        connection.table = None;
                    }
                }
            }
            Some(DatabaseExplorerState::Columns(schema_name, _)) => {
                // Go back to tables in the same schema
                if let Some(table_table) = &self.table_table {
                    self.explorer_state = Some(DatabaseExplorerState::Tables(
                        schema_name.clone(),
                    ));

                    if let Some(connection) = &mut self.active_connection {
                        connection.table = None;
                    }
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                // Go back to schemas
                if let Some(schema_table) = &self.schema_table {
                    self.explorer_state = Some(DatabaseExplorerState::Schemas);

                    if let Some(connection) = &mut self.active_connection {
                        connection.schema = None;
                    }
                }
            }
            Some(DatabaseExplorerState::Schemas) => {
                // Go back to connection list (disconnect)
                self.disconnect_from_database();
            }
            None => {
                // Go back to connection list (disconnect)
                self.disconnect_from_database();
            }
        }
    }

    /// Handle table navigation for the current database table
    fn handle_database_table_navigation(&mut self, key: KeyCode) {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                if let Some(schema_table) = &mut self.schema_table {
                    match key {
                        KeyCode::Char('j') | KeyCode::Down => {
                            schema_table.table_state.select_next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            schema_table.table_state.select_previous();
                        }
                        KeyCode::Char('h')
                        | KeyCode::Left
                        | KeyCode::Char('b') => {
                            schema_table.table_state.select_previous_column();
                        }
                        KeyCode::Char('l')
                        | KeyCode::Right
                        | KeyCode::Char('w') => {
                            schema_table.table_state.select_next_column();
                        }
                        KeyCode::Char('g') => {
                            schema_table.table_state.select(Some(1));
                        }
                        KeyCode::Char('G') => {
                            if !schema_table.items.is_empty() {
                                schema_table
                                    .table_state
                                    .select(Some(schema_table.items.len()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                if let Some(table_table) = &mut self.table_table {
                    match key {
                        KeyCode::Char('j') | KeyCode::Down => {
                            table_table.table_state.select_next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            table_table.table_state.select_previous();
                        }
                        KeyCode::Char('h')
                        | KeyCode::Left
                        | KeyCode::Char('b') => {
                            table_table.table_state.select_previous_column();
                        }
                        KeyCode::Char('l')
                        | KeyCode::Right
                        | KeyCode::Char('w') => {
                            table_table.table_state.select_next_column();
                        }
                        KeyCode::Char('g') => {
                            table_table.table_state.select(Some(1));
                        }
                        KeyCode::Char('G') => {
                            if !table_table.items.is_empty() {
                                table_table
                                    .table_state
                                    .select(Some(table_table.items.len()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(DatabaseExplorerState::Columns(_, _)) => {
                if let Some(column_table) = &mut self.column_table {
                    match key {
                        KeyCode::Char('j') | KeyCode::Down => {
                            column_table.table_state.select_next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            column_table.table_state.select_previous();
                        }
                        KeyCode::Char('h')
                        | KeyCode::Left
                        | KeyCode::Char('b') => {
                            column_table.table_state.select_previous_column();
                        }
                        KeyCode::Char('l')
                        | KeyCode::Right
                        | KeyCode::Char('w') => {
                            column_table.table_state.select_next_column();
                        }
                        KeyCode::Char('g') => {
                            column_table.table_state.select(Some(1));
                        }
                        KeyCode::Char('G') => {
                            if !column_table.items.is_empty() {
                                column_table
                                    .table_state
                                    .select(Some(column_table.items.len()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(DatabaseExplorerState::TableData(_, _)) => {
                if let Some(table_data) = &mut self.table_data {
                    match key {
                        KeyCode::Char('j') | KeyCode::Down => {
                            table_data.table_state.select_next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            table_data.table_state.select_previous();
                        }
                        KeyCode::Char('h')
                        | KeyCode::Left
                        | KeyCode::Char('b') => {
                            table_data.table_state.select_previous_column();
                        }
                        KeyCode::Char('l')
                        | KeyCode::Right
                        | KeyCode::Char('w') => {
                            table_data.table_state.select_next_column();
                        }
                        KeyCode::Char('g') => {
                            table_data.table_state.select(Some(1));
                        }
                        KeyCode::Char('G') => {
                            if !table_data.items.is_empty() {
                                table_data
                                    .table_state
                                    .select(Some(table_data.items.len()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            None => {}
        }
    }

    /// Load table data for a table
    async fn load_table_data(
        &mut self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        if let Some(database) = &self.active_database {
            match database
                .get_table_data_with_columns(schema_name, table_name)
                .await
            {
                Ok((data, column_names)) => {
                    self.table_data =
                        Some(TableDataWidget::new(data, column_names));
                    self.explorer_state =
                        Some(DatabaseExplorerState::TableData(
                            schema_name.to_string(),
                            table_name.to_string(),
                        ));
                }
                Err(e) => {
                    eprintln!("Failed to load table data: {}", e);
                }
            }
        }
        Ok(())
    }
}
