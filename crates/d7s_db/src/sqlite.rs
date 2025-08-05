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
            user TEXT,
            password TEXT,
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
        "INSERT INTO connections (name, host, port, user, password, database) VALUES (?, ?, ?, ?, ?, ?)",
        params![connection.name, connection.host, connection.port, connection.user, connection.password, connection.database],
    )?;

    Ok(())
}
