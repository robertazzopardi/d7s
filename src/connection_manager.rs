use color_eyre::Result;
use d7s_db::{Database, connection::Connection};
use d7s_ui::widgets::top_bar_view::{CONNECTION_HOTKEYS, DATABASE_HOTKEYS};

use crate::{app::App, app_state::AppState, database_explorer_state::DatabaseExplorer};

impl App<'_> {
    /// Get the currently selected connection from the connection list
    pub fn get_selected_connection(&self) -> Option<&Connection> {
        self.connections
            .table
            .state
            .selected()
            .filter(|&idx| idx < self.connections.table.items.len())
            .and_then(|idx| self.connections.table.items.get(idx))
    }

    /// Get the password for a connection (delegates to PasswordService)
    pub fn get_connection_password(&self, connection: &Connection) -> String {
        self.password_service.get_connection_password(connection)
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
                format!("Enter password for user '{}':", connection.user)
            } else {
                format!(
                    "Password not found for user '{}'.\nPlease enter password:",
                    connection.user
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

        // Test the connection first
        let postgres = connection_with_password.to_postgres();
        if postgres.test().await {
            // Connection successful, create database explorer
            self.database_explorer = Some(DatabaseExplorer::new(
                connection_with_password,
                postgres,
            ));
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
    pub fn disconnect_from_database(&mut self) {
        self.database_explorer = None;
        self.state = AppState::ConnectionList;

        // Update hotkeys for connection mode
        self.hotkeys = CONNECTION_HOTKEYS.to_vec();
    }
}
