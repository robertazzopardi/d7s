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

// For d7s storage
pub fn init_db() -> Result<()> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    // Example: create a table for storing connection info for d7s
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            host TEXT,
            port TEXT,
            database TEXT
        );
        ",
    )?;

    Ok(())
}

pub fn save_connection(
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    conn.execute(
        "INSERT INTO connections (name, host, port, database) VALUES (?, ?, ?, ?)",
        params![connection.name, connection.host, connection.port, connection.database],
    )?;

    Ok(())
}

pub fn get_connections() -> Result<Vec<Connection>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    let mut stmt = conn.prepare("SELECT * FROM connections")?;
    let connections = stmt
        .query_map([], |row| {
            Ok(Connection {
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                database: row.get(4)?,
                user: "".to_string(),
                schema: None,
                table: None,
                password: "".to_string(),
            })
        })?
        .map(|r| r.unwrap())
        .collect();

    Ok(connections)
}
