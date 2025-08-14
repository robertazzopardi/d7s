use color_eyre::Result;
use rusqlite::{Connection as SqliteConnection, params};

use crate::{Database, connection::Connection, get_db_path};

pub struct Sqlite {
    pub name: String,
    pub path: String,
}

impl Database for Sqlite {
    async fn test(&self) -> bool {
        true
    }
}

/// Initialize the database
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if the query fails.
pub fn init_db() -> Result<()> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            host TEXT,
            port TEXT,
            database TEXT,
            user TEXT
        );
        ",
    )?;

    Ok(())
}

/// Save a connection to the database
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if the query fails.
pub fn save_connection(
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    conn.execute(
        "INSERT INTO connections (name, host, port, database, user) VALUES (?, ?, ?, ?, ?)",
        params![connection.name, connection.host, connection.port, connection.database, connection.user],
    )?;

    Ok(())
}

/// Get all connections from the database
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if the query fails.
pub fn get_connections() -> Result<Vec<Connection>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    let mut stmt = conn.prepare("SELECT * FROM connections")?;
    let connections = stmt
        .query_map([], |row| {
            let user: String = row.get(5)?;
            let password = {
                keyring::Entry::new("d7s", &user)
                    .map_or(None, |entry| entry.get_password().ok())
            };

            Ok(Connection {
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                database: row.get(4)?,
                user,
                schema: None,
                table: None,
                password,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(connections)
}

/// Update a connection in the database
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if the query fails.
pub fn update_connection(
    old_name: &str,
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    conn.execute(
        "UPDATE connections SET name = ?, host = ?, port = ?, database = ?, user = ? WHERE name = ?",
        params![connection.name, connection.host, connection.port, connection.database, connection.user, old_name],
    )?;

    Ok(())
}

/// Delete a connection from the database
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if the query fails.
pub fn delete_connection(
    connection_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    conn.execute(
        "DELETE FROM connections WHERE name = ?",
        params![connection_name],
    )?;

    Ok(())
}
