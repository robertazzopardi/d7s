use color_eyre::Result;
use d7s_db::{
    Database,
    connection::Connection,
    sqlite::{
        delete_connection as db_delete_connection,
        get_connections as db_get_connections,
        save_connection as db_save_connection,
        update_connection as db_update_connection,
    },
};

/// Service for managing database connections (CRUD operations)
pub struct ConnectionService;

impl ConnectionService {
    /// Get all connections from the database
    pub fn get_all() -> Result<Vec<Connection>> {
        db_get_connections()
    }

    /// Create a new connection
    pub fn create(connection: &Connection) -> Result<()> {
        db_save_connection(connection)
            .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        Ok(())
    }

    /// Update an existing connection (handles renames)
    pub fn update(old_name: &str, connection: &Connection) -> Result<()> {
        // update_connection handles renaming automatically via WHERE clause
        db_update_connection(old_name, connection)
            .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        Ok(())
    }

    /// Delete a connection by name
    pub fn delete(name: &str) -> Result<()> {
        db_delete_connection(name)
            .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
        Ok(())
    }

    /// Validate a connection (check required fields are present)
    pub fn validate(connection: &Connection) -> Result<(), String> {
        if connection.name.trim().is_empty() {
            return Err("Connection name is required".to_string());
        }
        if connection.host.trim().is_empty() {
            return Err("Host is required".to_string());
        }
        if connection.user.trim().is_empty() {
            return Err("User is required".to_string());
        }
        if connection.database.trim().is_empty() {
            return Err("Database is required".to_string());
        }
        Ok(())
    }

    /// Test a connection by attempting to connect
    pub async fn test(connection: &Connection) -> bool {
        let postgres = connection.to_postgres();
        postgres.test().await
    }
}
