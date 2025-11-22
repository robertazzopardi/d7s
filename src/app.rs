use std::{collections::HashMap, sync::Arc};

use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use d7s_auth::Keyring;
use d7s_db::{
    Column, Database, Schema, Table, TableData,
    connection::Connection,
    postgres::Postgres,
    sqlite::{get_connections, init_db},
};
use d7s_ui::{
    handlers::{
        TableNavigationHandler, handle_connection_list_navigation,
        handle_save_connection, handle_search_filter_input,
        handle_sql_executor_input, test_connection,
    },
    widgets::{
        constraint_len_calculator,
        hotkey::Hotkey,
        modal::{ModalAction, ModalManager, TestResult},
        search_filter::SearchFilter,
        sql_executor::SqlExecutor,
        status_line::StatusLine,
        table::{DataTable, RawTableRow},
        top_bar_view::{CONNECTION_HOTKEYS, DATABASE_HOTKEYS, TopBarView},
    },
};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::*,
    widgets::{Block, Borders},
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    ConnectionList,
    DatabaseConnected,
}

/// Database explorer state to track what object type is being viewed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseExplorerState {
    Schemas,
    Tables(String),            // schema name
    Columns(String, String),   // schema name, table name
    TableData(String, String), // schema name, table name
    SqlExecutor,               // SQL execution mode
}

/// The main application which holds the state and logic of the application.
pub struct App<'a> {
    /// Is the application running?
    running: bool,
    show_popup: bool,
    modal_manager: ModalManager,
    #[allow(dead_code)]
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
    table_data: Option<DataTable<RawTableRow>>,
    /// SQL executor widget
    sql_executor: SqlExecutor,
    /// Search filter widget
    search_filter: SearchFilter,
    /// Status line widget
    status_line: StatusLine,
    /// Original unfiltered data for restoration
    original_connections: Vec<Connection>,
    original_schemas: Vec<Schema>,
    original_tables: Vec<Table>,
    original_columns: Vec<Column>,
    original_table_data: Vec<Vec<String>>,
    /// Session password storage (in-memory only, cleared when app exits)
    /// Key format: "{user}@{host}:{port}/{database}"
    session_passwords: HashMap<String, String>,
    /// Whether to automatically store passwords in session memory when "ask every time" is enabled
    /// Default: true (auto-store for better UX)
    auto_store_session_password: bool,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            running: false,
            show_popup: false,
            modal_manager: ModalManager::new(),
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
            table_widget: DataTable::default(),
            state: AppState::ConnectionList,
            active_connection: None,
            active_database: None,
            explorer_state: None,
            schema_table: None,
            table_table: None,
            column_table: None,
            table_data: None,
            sql_executor: SqlExecutor::new(),
            search_filter: SearchFilter::new(),
            status_line: StatusLine::new(),
            original_connections: Vec::new(),
            original_schemas: Vec::new(),
            original_tables: Vec::new(),
            original_columns: Vec::new(),
            original_table_data: Vec::new(),
            session_passwords: HashMap::new(),
            auto_store_session_password: true, // Default to auto-storing for better UX
        }
    }
}

impl App<'_> {
    pub fn initialise(mut self) -> Result<Self> {
        init_db()?;

        let items = get_connections().unwrap_or_default();

        let table_widget = DataTable::new(items.clone());

        self.table_widget = table_widget;
        self.original_connections = items;

        Ok(self)
    }

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
        // Split layout: top bar, main content, and status line
        // Status line gets fixed 1 row, main content takes the rest
        let mut main_layout =
            vec![Constraint::Percentage(13), Constraint::Min(0)];

        if !self.status_line.message().is_empty() {
            main_layout.push(Constraint::Length(1));
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(main_layout)
            .split(frame.area());
        let first_layout = *layout.first().unwrap_or(&Rect::default());

        let current_connection =
            self.active_connection.clone().unwrap_or_default();
        frame.render_widget(
            TopBarView {
                current_connection,
                hotkeys: &self.hotkeys,
                app_name: APP_NAME,
            },
            first_layout,
        );

        // Create the main content area (layout[1] is the middle section)
        let layout_rect = *layout.get(1).unwrap_or(&frame.area());
        let main_area = if self.search_filter.is_active {
            // If search filter is active, create a layout with search filter at top
            let search_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Search filter height
                    Constraint::Min(0),    // Remaining space for table
                ])
                .split(layout_rect);

            let search_layout_rect =
                *search_layout.first().unwrap_or(&Rect::default());

            // Render search filter
            frame.render_stateful_widget(
                self.search_filter.clone(),
                search_layout_rect,
                &mut (),
            );

            *search_layout.get(1).unwrap_or(&Rect::default())
        } else {
            layout_rect
        };

        match self.state {
            AppState::ConnectionList => {
                // Create the inner block
                let block = Block::new()
                    .borders(Borders::ALL)
                    .title(" Connections ")
                    .title_alignment(Alignment::Center);

                // Get the inner area of the block (content area)
                let inner_area = block.inner(main_area);

                // Render the block itself (borders and title)
                frame.render_widget(block, main_area);

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
                let inner_area = block.inner(main_area);

                // Render the block itself (borders and title)
                frame.render_widget(block, main_area);

                // Render the appropriate table based on explorer state
                self.render_database_table(frame, inner_area);
            }
        }

        // Render status line at the bottom
        if !self.status_line.message().is_empty()
            && let Some(status_layout) = layout.get(2)
        {
            frame.render_widget(self.status_line.clone(), *status_layout);
        }

        // Render modals using the modal manager
        self.render_modals(frame);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key).await?;
            }
            Event::Key(_) => todo!(),
            Event::FocusGained => todo!(),
            Event::FocusLost => todo!(),
            Event::Mouse(_) => todo!(),
            Event::Paste(_) => todo!(),
            Event::Resize(_, _) => todo!(),
        }

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    #[allow(clippy::too_many_lines)]
    async fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
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
        if matches!(
            &self.explorer_state,
            Some(DatabaseExplorerState::SqlExecutor)
        ) && handle_sql_executor_input(key, &mut self.sql_executor)
        {
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
                            eprintln!("Failed to load columns: {e}");
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
                            eprintln!("Failed to load table data: {e}");
                        }
                    }
                }
            }
            (_, KeyCode::Char('s')) => {
                if self.state == AppState::DatabaseConnected {
                    // Enter SQL execution mode
                    self.explorer_state =
                        Some(DatabaseExplorerState::SqlExecutor);
                    self.sql_executor.activate();
                }
            }
            (_, KeyCode::Esc) => {
                if self.show_popup {
                    self.toggle_popup();
                } else if self.modal_manager.is_any_modal_open() {
                    self.modal_manager.close_active_modal();
                } else if self.state == AppState::DatabaseConnected {
                    if matches!(
                        &self.explorer_state,
                        Some(DatabaseExplorerState::SqlExecutor)
                    ) {
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
                        &mut self.table_widget,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Down);
                }
            }
            (_, KeyCode::Char('k') | KeyCode::Up) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Up,
                        &mut self.table_widget,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Up);
                }
            }
            (_, KeyCode::Char('h' | 'b') | KeyCode::Left) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Left,
                        &mut self.table_widget,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Left);
                }
            }
            (_, KeyCode::Char('l' | 'w') | KeyCode::Right) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Right,
                        &mut self.table_widget,
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
                        &mut self.table_widget,
                    );
                }
            }
            (_, KeyCode::Char('$')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('$'),
                        &mut self.table_widget,
                    );
                }
            }
            (_, KeyCode::Char('g')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('g'),
                        &mut self.table_widget,
                    );
                } else if self.state == AppState::DatabaseConnected {
                    self.handle_database_table_navigation(KeyCode::Char('g'));
                }
            }
            (_, KeyCode::Char('G')) => {
                if self.state == AppState::ConnectionList {
                    handle_connection_list_navigation(
                        KeyCode::Char('G'),
                        &mut self.table_widget,
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

    const fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    /// Refresh the table data from the database
    fn refresh_connections(&mut self) {
        if let Ok(connections) = get_connections() {
            self.original_connections.clone_from(&connections);
            // Reapply filter if one is active, otherwise show all connections
            if self.search_filter.get_filter_query().is_empty() {
                self.table_widget.items = connections;
            } else {
                self.apply_filter();
            }
        }
    }

    /// Set running to false to quit the application.
    const fn quit(&mut self) {
        self.running = false;
    }

    /// Generate a unique key for a connection to use in session password storage
    fn connection_key(connection: &Connection) -> String {
        format!(
            "{}@{}:{}/{}",
            connection.user,
            connection.host,
            connection.port,
            connection.database
        )
    }

    /// Get the currently selected connection from the connection list
    fn get_selected_connection(&self) -> Option<&Connection> {
        self.table_widget
            .table_state
            .selected()
            .filter(|&idx| idx < self.table_widget.items.len())
            .and_then(|idx| self.table_widget.items.get(idx))
    }

    /// Get the password for a connection from keyring (if not "ask every time")
    fn get_connection_password(&self, connection: &Connection) -> String {
        if connection.should_ask_every_time() {
            String::new()
        } else {
            Keyring::new(&connection.name)
                .ok()
                .and_then(|keyring| keyring.get_password().ok())
                .unwrap_or_default()
        }
    }

    /// Connect to the selected database
    async fn connect_to_database(&mut self) -> Result<()> {
        let Some(connection) = self.get_selected_connection() else {
            return Ok(());
        };

        if connection.should_ask_every_time() {
            // Check session storage first
            let connection_key = Self::connection_key(connection);
            if let Some(session_password) =
                self.session_passwords.get(&connection_key)
            {
                self.connect_with_password(
                    connection.clone(),
                    session_password.clone(),
                )
                .await?;
            } else {
                let prompt =
                    format!("Enter password for user '{}':", connection.user);
                self.modal_manager
                    .open_password_modal(connection.clone(), prompt);
            }
        } else {
            // Try to get password from keyring
            if let Ok(entry) = Keyring::new(&connection.name) {
                if let Ok(password) = entry.get_password() {
                    self.connect_with_password(connection.clone(), password)
                        .await?;
                } else {
                    let prompt = format!(
                        "Password not found for user '{}'.\nPlease enter password:",
                        connection.user
                    );
                    self.modal_manager
                        .open_password_modal(connection.clone(), prompt);
                }
            } else {
                let prompt = format!(
                    "Unable to access keyring for user '{}'.\nPlease enter password:",
                    connection.user
                );
                self.modal_manager
                    .open_password_modal(connection.clone(), prompt);
            }
        }
        Ok(())
    }

    /// Connect to database with the provided password
    async fn connect_with_password(
        &mut self,
        connection: Connection,
        password: String,
    ) -> Result<()> {
        // Create connection with password
        let mut connection_with_password = connection.clone();
        connection_with_password.password = Some(password);

        // Test the connection first
        let postgres = connection_with_password.to_postgres();
        if postgres.test().await {
            // Connection successful, update state
            self.active_connection = Some(connection_with_password.clone());
            self.active_database = Some(postgres);
            self.state = AppState::DatabaseConnected;

            // Update hotkeys for database mode
            self.hotkeys = DATABASE_HOTKEYS.to_vec();

            // Load schemas after successful connection
            self.load_schemas().await?;
        } else {
            self.set_status(format!(
                "Failed to connect to database: {}",
                connection.name
            ));
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

        // Update hotkeys for connection mode
        self.hotkeys = CONNECTION_HOTKEYS.to_vec();
    }

    // TODO use an impl for this
    /// Get the title for the database view based on current state
    fn get_database_title(&self) -> String {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) => " Schemas ".to_string(),
            Some(DatabaseExplorerState::Tables(schema)) => {
                format!(" {schema} ")
            }
            Some(
                DatabaseExplorerState::Columns(schema, table)
                | DatabaseExplorerState::TableData(schema, table),
            ) => {
                format!(" {schema}.{table} ")
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                " SQL Executor ".to_string()
            }
            None => " Database Explorer ".to_string(),
        }
    }

    /// Render all active modals
    fn render_modals(&self, frame: &mut Frame) {
        let area = frame.area();

        if let Some(modal) = self.modal_manager.get_connection_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_confirmation_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_cell_value_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_password_modal() {
            frame.render_widget(modal.clone(), area);
        }
    }

    /// Render the appropriate database table based on explorer state
    fn render_database_table(&self, frame: &mut Frame, area: Rect) {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) | None => {
                render_data_table(frame, &self.schema_table, area);
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                render_data_table(frame, &self.table_table, area);
            }
            Some(DatabaseExplorerState::Columns(_, _)) => {
                render_data_table(frame, &self.column_table, area);
            }
            Some(DatabaseExplorerState::TableData(_, _)) => {
                render_data_table(frame, &self.table_data, area);
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                frame.render_widget(self.sql_executor.clone(), area);
            }
        }
    }

    /// Load schemas from the database
    async fn load_schemas(&mut self) -> Result<()> {
        if let Some(database) = &self.active_database {
            match database.get_schemas().await {
                Ok(schemas) => {
                    // Store original data for filtering
                    self.original_schemas.clone_from(&schemas);
                    self.schema_table = Some(DataTable::new(schemas));
                    self.explorer_state = Some(DatabaseExplorerState::Schemas);
                }
                Err(e) => {
                    eprintln!("Failed to load schemas: {e}");
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
                    // Store original data for filtering
                    self.original_tables.clone_from(&tables);
                    self.table_table = Some(DataTable::new(tables));
                    self.explorer_state = Some(DatabaseExplorerState::Tables(
                        schema_name.to_string(),
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to load tables: {e}");
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
                    // Store original data for filtering
                    self.original_columns.clone_from(&columns);
                    self.column_table = Some(DataTable::new(columns));
                    self.explorer_state = Some(DatabaseExplorerState::Columns(
                        schema_name.to_string(),
                        table_name.to_string(),
                    ));
                }
                Err(e) => {
                    eprintln!("Failed to load columns: {e}");
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
                if let Some(schema_table) = &self.schema_table
                    && let Some(selected_index) =
                        schema_table.table_state.selected()
                    && selected_index < schema_table.items.len()
                    && let Some(schema) = schema_table.items.get(selected_index)
                {
                    if let Some(connection) = &mut self.active_connection {
                        connection.schema = Some(schema.name.clone());
                    }

                    self.load_tables(&schema.name.clone()).await?;
                }
            }
            Some(DatabaseExplorerState::Tables(schema_name)) => {
                // Navigate to table data in selected table (show data first, not columns)
                if let Some(table_table) = &self.table_table
                    && let Some(selected_index) =
                        table_table.table_state.selected()
                    && selected_index < table_table.items.len()
                    && let Some(table) = table_table.items.get(selected_index)
                {
                    let schema_name = schema_name.clone();
                    let table_name = table.name.clone();

                    if let Some(connection) = &mut self.active_connection {
                        connection.table = Some(table_name.clone());
                    }

                    self.load_table_data(&schema_name, &table_name).await?;
                }
            }
            Some(DatabaseExplorerState::Columns(schema_name, table_name)) => {
                // Toggle to data view
                let schema_name = schema_name.clone();
                let table_name = table_name.clone();
                self.load_table_data(&schema_name, &table_name).await?;
            }
            Some(DatabaseExplorerState::TableData(schema_name, table_name)) => {
                // Show cell value in dialog if a cell is selected
                if let Some(table_data) = &self.table_data
                    && let Some(selected_row) =
                        table_data.table_state.selected()
                    && selected_row < table_data.items.len()
                {
                    let selected_col =
                        table_data.table_state.selected_column().unwrap_or(0);

                    let selected_index = table_data
                        .items
                        .get(selected_row)
                        .map(|row| row.values.len())
                        .unwrap_or_default();
                    if let Some(ref column_names) =
                        table_data.dynamic_column_names
                        && selected_col < column_names.len()
                        && selected_col < selected_index
                    {
                        let no_column_name_error_message =
                            &"Could not get column name.".to_string();
                        let column_name = column_names
                            .get(selected_col)
                            .unwrap_or(no_column_name_error_message)
                            .clone();

                        let no_cell_value_error_message =
                            &"Could not get cell value.".to_string();
                        let cell_value = table_data
                            .items
                            .get(selected_row)
                            .map(|item| &item.values)
                            .and_then(|values| values.get(selected_col))
                            .unwrap_or(no_cell_value_error_message)
                            .clone();

                        self.modal_manager
                            .open_cell_value_modal(column_name, cell_value);
                        return Ok(());
                    }
                }

                // If no cell selected or invalid, toggle back to columns view
                self.explorer_state = Some(DatabaseExplorerState::Columns(
                    schema_name.clone(),
                    table_name.clone(),
                ));
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // Execute SQL query
                if !self.sql_executor.sql_input.trim().is_empty()
                    && let Some(database) = &self.active_database
                {
                    match database
                        .execute_sql(&self.sql_executor.sql_input)
                        .await
                    {
                        Ok(results) => {
                            let data: Vec<Vec<String>> = results
                                .iter()
                                .map(|row| row.values.clone())
                                .collect();

                            let column_names = results
                                .first()
                                .map(|result| result.column_names.clone());

                            if let Some(names) = column_names {
                                self.sql_executor.set_results(data, &names);
                            }
                        }
                        Err(e) => {
                            self.sql_executor.set_error(e.to_string());
                        }
                    }
                }
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
            Some(
                DatabaseExplorerState::TableData(schema_name, _)
                | DatabaseExplorerState::Columns(schema_name, _),
            ) => {
                // Go back to tables in the same schema
                if self.table_table.is_some() {
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
                if self.schema_table.is_some() {
                    self.explorer_state = Some(DatabaseExplorerState::Schemas);

                    if let Some(connection) = &mut self.active_connection {
                        connection.schema = None;
                    }
                }
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // Go back to schemas
                if self.schema_table.is_some() {
                    self.explorer_state = Some(DatabaseExplorerState::Schemas);
                }
            }
            Some(DatabaseExplorerState::Schemas) | None => {
                // Go back to connection list (disconnect)
                self.disconnect_from_database();
            }
        }
    }

    /// Handle table navigation for the current database table
    fn handle_database_table_navigation(&mut self, key: KeyCode) {
        match &self.explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                TableNavigationHandler::navigate(&mut self.schema_table, key);
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                TableNavigationHandler::navigate(&mut self.table_table, key);
            }
            Some(DatabaseExplorerState::Columns(_, _)) => {
                TableNavigationHandler::navigate(&mut self.column_table, key);
            }
            Some(DatabaseExplorerState::TableData(_, _)) => {
                TableNavigationHandler::navigate(&mut self.table_data, key);
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // If we have results, handle table navigation
                if self.sql_executor.table_widget.is_some() {
                    TableNavigationHandler::handle_sql_results_navigation(
                        &mut self.sql_executor,
                        key,
                    );
                }
            }
            None => {}
        }
    }

    /// Handle modal events
    #[allow(clippy::too_many_lines)]
    async fn handle_modal_events(&mut self, key: KeyEvent) -> Result<()> {
        // Handle modal events (UI only)
        let action = self.modal_manager.handle_key_events_ui(key);

        // Handle business logic based on modal actions
        match action {
            ModalAction::Save => {
                // Handle password modal save (only used for "ask every time" connections)
                if let Some(password_modal) =
                    self.modal_manager.get_password_modal_mut()
                    && let Some(connection) = &password_modal.connection.clone()
                {
                    let password = password_modal.password.clone();
                    password_modal.close();

                    // For "ask every time" connections, never access keyring
                    // Only store in session memory if auto-store is enabled
                    if self.auto_store_session_password
                        && connection.should_ask_every_time()
                    {
                        let connection_key = Self::connection_key(connection);
                        self.session_passwords
                            .insert(connection_key, password.clone());
                    }

                    // Connect with the provided password
                    // Don't create keyring for "ask every time" connections
                    if let Err(e) = self
                        .connect_with_password(connection.clone(), password)
                        .await
                    {
                        eprintln!("Failed to connect: {e}");
                    }
                }
                // Handle connection modal save
                else if let Some(modal) =
                    self.modal_manager.get_connection_modal_mut()
                    && let Some(connection) = modal.get_connection()
                {
                    let original_name = modal.original_name.clone();
                    let mode = modal.mode;

                    // Always save password to keyring if using keyring storage and password is provided
                    let should_save_to_keyring = connection.uses_keyring()
                        && connection.password.is_some();

                    // Prepare keyring for saving (only if we should save)
                    let keyring_for_save = if should_save_to_keyring {
                        &mut Keyring::new(&connection.name).ok()
                    } else {
                        &mut None
                    };

                    match handle_save_connection(
                        keyring_for_save,
                        &connection,
                        mode,
                        original_name,
                    ) {
                        Ok(()) => {
                            modal.close();
                            self.refresh_connections();
                        }
                        Err(error_msg) => {
                            modal.test_result = TestResult::Failed(error_msg);
                        }
                    }
                }
            }
            ModalAction::Test => {
                if let Some(modal) =
                    self.modal_manager.get_connection_modal_mut()
                {
                    if let Some(connection) = modal.get_connection() {
                        modal.test_result = TestResult::Testing;
                        modal.test_result = test_connection(&connection).await;
                    } else {
                        modal.test_result = TestResult::Failed(
                            "Please fill in all fields".to_string(),
                        );
                    }
                }
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
            // Only try to delete from keyring if not using "ask every time"
            if !connection.should_ask_every_time()
                && let Some(keyring) = &Keyring::new(&connection.name).ok()
            {
                let _ = keyring.delete_password();
            }

            if let Err(e) = d7s_db::sqlite::delete_connection(&connection.name)
            {
                eprintln!("Failed to delete connection: {e}");
            } else {
                self.refresh_connections();
            }
        }

        // Clean up closed modals
        self.modal_manager.cleanup_closed_modals();

        Ok(())
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
                    // Store original data for filtering
                    self.original_table_data.clone_from(&data);
                    self.table_data =
                        Some(DataTable::from_raw_data(data, &column_names));
                    self.explorer_state =
                        Some(DatabaseExplorerState::TableData(
                            schema_name.to_string(),
                            table_name.to_string(),
                        ));
                }
                Err(e) => {
                    eprintln!("Failed to load table data: {e}");
                }
            }
        }
        Ok(())
    }

    /// Apply the current search filter to the active table
    fn apply_filter(&mut self) {
        let query = self.search_filter.get_filter_query();

        match self.state {
            AppState::ConnectionList => {
                // Filter from original_connections, not from already-filtered items
                let temp_table =
                    DataTable::new(self.original_connections.clone());
                let filtered_items = temp_table.filter(query);
                self.table_widget.items = filtered_items;
                TableNavigationHandler::clamp_selection(&mut self.table_widget);
            }
            AppState::DatabaseConnected => {
                match self.explorer_state {
                    Some(DatabaseExplorerState::Schemas) => {
                        filter_table_data(query, &mut self.schema_table);
                    }
                    Some(DatabaseExplorerState::Tables(_)) => {
                        filter_table_data(query, &mut self.table_table);
                    }
                    Some(DatabaseExplorerState::Columns(_, _)) => {
                        filter_table_data(query, &mut self.column_table);
                    }
                    Some(DatabaseExplorerState::TableData(_, _)) => {
                        filter_table_data(query, &mut self.table_data);
                    }
                    Some(DatabaseExplorerState::SqlExecutor) | None => {
                        // No filtering for SQL executor
                    }
                }
            }
        }
    }

    /// Clear the current filter and restore original data
    fn clear_filter(&mut self) {
        match self.state {
            AppState::ConnectionList => {
                self.table_widget.items = self.original_connections.clone();
                TableNavigationHandler::clamp_selection(&mut self.table_widget);
            }
            AppState::DatabaseConnected => {
                match self.explorer_state {
                    Some(DatabaseExplorerState::Schemas) => {
                        restore_table_data(
                            &mut self.schema_table,
                            &self.original_schemas,
                        );
                    }
                    Some(DatabaseExplorerState::Tables(_)) => {
                        restore_table_data(
                            &mut self.table_table,
                            &self.original_tables,
                        );
                    }
                    Some(DatabaseExplorerState::Columns(_, _)) => {
                        restore_table_data(
                            &mut self.column_table,
                            &self.original_columns,
                        );
                    }
                    Some(DatabaseExplorerState::TableData(_, _)) => {
                        if let Some(table_data) = &mut self.table_data {
                            if let Some(ref column_names) =
                                table_data.dynamic_column_names
                            {
                                // Recreate items from original data
                                let column_names_arc = Arc::clone(column_names);
                                table_data.items = self
                                    .original_table_data
                                    .iter()
                                    .map(|values| RawTableRow {
                                        values: values.clone(),
                                        column_names: Arc::clone(
                                            &column_names_arc,
                                        ),
                                    })
                                    .collect();
                                // Recalculate longest_item_lens
                                table_data.longest_item_lens =
                                    constraint_len_calculator(
                                        &table_data.items,
                                    );
                            }
                            TableNavigationHandler::clamp_selection(table_data);
                        }
                    }
                    Some(DatabaseExplorerState::SqlExecutor) | None => {
                        // No filtering for SQL executor
                    }
                }
            }
        }
    }

    /// Set the status line message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_line.set_message(message);
    }

    /// Clear the status line
    pub fn clear_status(&mut self) {
        self.status_line.clear();
    }
}

fn filter_table_data<T: TableData + Clone>(
    query: &str,
    table: &mut Option<DataTable<T>>,
) {
    if let Some(table) = table {
        table.items = table.filter(query);
        TableNavigationHandler::clamp_selection(table);
    }
}

fn restore_table_data<T: TableData + Clone>(
    table: &mut Option<DataTable<T>>,
    original_data: &[T],
) {
    if let Some(table) = table {
        table.items.clone_from(&original_data.to_vec());
        TableNavigationHandler::clamp_selection(table);
    }
}

fn render_data_table<T: TableData + Clone + std::fmt::Debug>(
    frame: &mut Frame,
    table: &Option<DataTable<T>>,
    area: Rect,
) {
    if let Some(table) = table {
        frame.render_stateful_widget(
            table.clone(),
            area,
            &mut table.table_state.clone(),
        );
    }
}
