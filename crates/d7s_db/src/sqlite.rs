use color_eyre::Result;
use rusqlite::{Connection as SqliteConnection, params};
use rusqlite_migration::{M, Migrations};

use crate::{Database, TableRow, connection::Connection, get_db_path};

pub struct Sqlite {
    pub name: String,
    pub path: String,
}

impl Database for Sqlite {
    async fn test(&self) -> bool {
        true
    }

    async fn execute_sql(
        &self,
        sql: &str,
    ) -> Result<Vec<TableRow>, Box<dyn std::error::Error>> {
        // rusqlite is synchronous, so we just run it in the async context
        let client = self.get_connection()?;

        // Try to prepare the statement
        let mut stmt = client.prepare(sql)?;

        // Try to get column names
        let column_names: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        let mut result = Vec::new();

        // Try to query for rows
        let mut rows_iter = stmt.query([])?;

        let mut found_row = false;
        while let Some(row) = rows_iter.next()? {
            found_row = true;
            let mut values = Vec::new();
            for i in 0..column_names.len() {
                let value = convert_sqlite_value_to_string(row, i);
                values.push(value);
            }
            result.push(TableRow {
                values,
                column_names: column_names.clone(),
            });
        }

        // If no rows, treat as an execute (e.g. INSERT/UPDATE/DELETE)
        if !found_row {
            let affected_rows = client.execute(sql, [])?;
            result.push(TableRow {
                values: vec![format!("Affected rows: {}", affected_rows)],
                column_names: vec!["Result".to_string()],
            });
        }

        Ok(result)
    }
}

impl Sqlite {
    fn get_connection(
        &self,
    ) -> Result<SqliteConnection, Box<dyn std::error::Error>> {
        Ok(SqliteConnection::open(&self.path)?)
    }
}

/// Initialize the database with migrations
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if migrations fail.
pub fn init_db() -> Result<()> {
    let db_path = get_db_path()?;
    let mut conn = SqliteConnection::open(db_path)?;

    // Define migrations
    let migrations = Migrations::new(vec![
        M::up(
            "CREATE TABLE IF NOT EXISTS connections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                host TEXT,
                port TEXT,
                database TEXT,
                user TEXT
            );",
        ),
        M::up("ALTER TABLE connections ADD COLUMN password_storage TEXT;")
            .down("ALTER TABLE connections DROP COLUMN password_storage;"),
    ]);

    // Apply migrations
    migrations.to_latest(&mut conn)?;

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
        "INSERT INTO connections (name, host, port, database, user, password_storage) VALUES (?, ?, ?, ?, ?, ?)",
        params![
            connection.name,
            connection.host,
            connection.port,
            connection.database,
            connection.user,
            connection.password_storage
        ],
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
            Ok(Connection {
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                database: row.get(4)?,
                user: row.get(5)?,
                schema: None,
                table: None,
                password: None,
                password_storage: row.get(6).ok(), // May be NULL for old connections
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
        "UPDATE connections SET name = ?, host = ?, port = ?, database = ?, user = ?, password_storage = ? WHERE name = ?",
        params![
            connection.name,
            connection.host,
            connection.port,
            connection.database,
            connection.user,
            connection.password_storage,
            old_name
        ],
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

/// Convert a `SQLite` value to a string representation
fn convert_sqlite_value_to_string(row: &rusqlite::Row, index: usize) -> String {
    // Try to get as different types and convert to string
    if let Ok(value) = row.get::<_, Option<String>>(index) {
        return value.unwrap_or_else(|| "NULL".to_string());
    }

    if let Ok(value) = row.get::<_, Option<i64>>(index) {
        return value.map_or_else(|| "NULL".to_string(), |v| v.to_string());
    }

    if let Ok(value) = row.get::<_, Option<f64>>(index) {
        return value.map_or_else(|| "NULL".to_string(), |v| v.to_string());
    }

    if let Ok(value) = row.get::<_, Option<Vec<u8>>>(index) {
        return value.map_or_else(
            || "NULL".to_string(),
            |v| format!("<{} bytes>", v.len()),
        );
    }

    // Fallback for unknown types
    "<unprintable>".to_string()
}
