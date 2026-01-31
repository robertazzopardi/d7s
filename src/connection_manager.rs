use color_eyre::Result;
use d7s_db::{Database, connection::Connection};
use d7s_ui::widgets::top_bar_view::{CONNECTION_HOTKEYS, DATABASE_HOTKEYS};

use crate::{
    app::App,
    app_state::{AppState, DatabaseExplorerState},
    database_explorer_state::DatabaseExplorer,
};

impl App<'_> {
    /// Get the currently selected connection from the connection list
    pub fn get_selected_connection(&self) -> Option<&Connection> {
        self.connections
            .table
            .view
            .state
            .selected()
            .filter(|&idx| idx < self.connections.table.model.items.len())
            .and_then(|idx| self.connections.table.model.items.get(idx))
    }

    /// Connect to the selected database
    pub async fn connect_to_database(&mut self) -> Result<()> {
        let Some(connection) = self.get_selected_connection() else {
            return Ok(());
        };

        // Try to get password from service (checks session first, then keyring)
        if let Some(password) = self.password_service.get_password(connection) {
            self.connect_with_password(connection.clone(), password)
                .await?;
        } else {
            // Need to prompt for password
            let prompt = if connection.should_ask_every_time() {
                format!("Enter password for user '{}':", connection.user_display())
            } else {
                format!(
                    "Password not found for user '{}'.\nPlease enter password:",
                    connection.user_display()
                )
            };
            self.modal_manager
                .open_password_modal(connection.clone(), prompt);
        }
        Ok(())
    }

    /// Connect to database with the provided password
    pub async fn connect_with_password(
        &mut self,
        connection: Connection,
        password: String,
    ) -> Result<()> {
        // Create connection with password
        let mut connection_with_password = connection.clone();
        connection_with_password.password = Some(password);

        // For PostgreSQL, connect to a default database first to list databases
        let default_db = "postgres".to_string();

        // Create a temporary connection to the default database
        let mut temp_connection = connection_with_password.clone();
        temp_connection.selected_database = Some(default_db.clone());
        let postgres = temp_connection.to_postgres();

        if postgres.test().await {
            // Connection successful; keep selected_database so explorer is on "postgres"
            connection_with_password.selected_database = Some(default_db);
            let boxed_db: Box<dyn Database> = Box::new(postgres);
            self.database_explorer =
                DatabaseExplorer::new(connection_with_password, Some(boxed_db));
            self.state = AppState::DatabaseConnected;

            // Update hotkeys for database mode
            self.hotkeys = DATABASE_HOTKEYS.to_vec();

            // Load databases after successful connection
            self.load_databases().await?;
        } else {
            self.set_status(format!(
                "Failed to connect to database: {}",
                connection.name
            ));
        }
        Ok(())
    }

    /// Disconnect from the current database
    pub fn disconnect_from_database(&mut self) {
        self.database_explorer.state = DatabaseExplorerState::Connections;
        self.state = AppState::ConnectionList;

        // Update hotkeys for connection mode
        self.hotkeys = CONNECTION_HOTKEYS.to_vec();
    }
}
