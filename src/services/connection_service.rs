use color_eyre::{Result, eyre::eyre};
use d7s_db::{
    Database,
    connection::{Connection, ConnectionType},
    sqlite::{
        delete_connection, get_connections, save_connection, update_connection,
    },
};

/// Service for managing database connections (CRUD operations)
pub struct ConnectionService;

impl ConnectionService {
    /// Get all connections from the database
    pub fn get_all() -> Result<Vec<Connection>> {
        get_connections()
    }

    /// Create a new connection
    pub fn create(connection: &Connection) -> Result<()> {
        save_connection(connection).map_err(|e| eyre!("{}", e))?;
        Ok(())
    }

    /// Update an existing connection (handles renames)
    pub fn update(old_name: &str, connection: &Connection) -> Result<()> {
        // update_connection handles renaming automatically via WHERE clause
        update_connection(old_name, connection).map_err(|e| eyre!("{}", e))?;
        Ok(())
    }

    /// Delete a connection by name
    pub fn delete(name: &str) -> Result<()> {
        delete_connection(name).map_err(|e| eyre!("{}", e))?;
        Ok(())
    }

    /// Validate a connection (check required fields are present)
    pub fn validate(connection: &Connection) -> Result<(), String> {
        if connection.name.trim().is_empty() {
            return Err("Connection name is required".to_string());
        }
        if connection.url.trim().is_empty() {
            return Err("Connection url is required".to_string());
        }
        Ok(())
    }

    /// Test a connection by attempting to connect (postgres or sqlite)
    pub async fn test(connection: &Connection) -> bool {
        match connection.r#type {
            ConnectionType::Postgres => connection.to_postgres().test().await,
            ConnectionType::Sqlite => connection.to_sqlite().test().await,
        }
    }
}
