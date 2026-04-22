use std::{collections::HashMap, fmt::Write};

use color_eyre::Result;
use rusqlite::{Connection as SqliteConnection, params};
use rusqlite_migration::{M, Migrations};

use crate::db::{
    Column, Database, DatabaseInfo, DbRowId, Schema, Table, TableData,
    TableDataPage, TableRow,
    connection::{Connection, ConnectionType, Environment},
    get_db_path,
};

fn sqlite_quote_ident(ident: &str) -> String {
    format!(r#""{}""#, ident.replace('"', "\"\""))
}

fn sqlite_table_decltypes(
    conn: &SqliteConnection,
    table_name: &str,
) -> Result<HashMap<String, String>, rusqlite::Error> {
    let mut stmt =
        conn.prepare(&format!("PRAGMA table_info('{table_name}')"))?;
    let mut m = HashMap::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let decl: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
        m.insert(name, decl);
    }
    Ok(m)
}

fn sqlite_resolve_decl<'a>(
    decls: &'a HashMap<String, String>,
    col: &str,
) -> &'a str {
    decls
        .get(col)
        .map(String::as_str)
        .or_else(|| {
            decls
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(col))
                .map(|(_, v)| v.as_str())
        })
        .unwrap_or("")
}

/// Target for `SQLite` [`CAST`](https://www.sqlite.org/lang_expr.html#castexpr): `INTEGER`, `REAL`, `TEXT`, or `BLOB`.
fn sqlite_cast_keyword(decl: &str) -> &'static str {
    let d = decl.trim();
    if d.is_empty() {
        return "TEXT";
    }
    let u = d.to_ascii_uppercase();
    if u.contains("INT") || u == "BOOLEAN" || u == "BOOL" {
        return "INTEGER";
    }
    if u.contains("BLOB") {
        return "BLOB";
    }
    if u.contains("REAL")
        || u.contains("FLOA")
        || u.contains("DOUB")
        || u.contains("NUM")
        || u.contains("DEC")
    {
        return "REAL";
    }
    if u.contains("CHAR") || u.contains("CLOB") || u.contains("TEXT") {
        return "TEXT";
    }
    "TEXT"
}

pub struct Sqlite {
    pub name: String,
    pub path: String,
}

impl TableData for Sqlite {
    fn title() -> &'static str {
        "Sqlite"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![self.name.clone(), self.path.clone()]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Path"]
    }
}

#[async_trait::async_trait]
impl Database for Sqlite {
    async fn test(&self) -> bool {
        SqliteConnection::open(&self.path).is_ok()
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

    async fn get_schemas(
        &self,
    ) -> Result<Vec<Schema>, Box<dyn std::error::Error>> {
        Ok(vec![Schema {
            name: "sqlite_schema".to_string(),
            owner: String::new(),
        }])
    }

    async fn get_tables(
        &self,
        schema_name: &str,
    ) -> Result<Vec<Table>, Box<dyn std::error::Error>> {
        let conn = SqliteConnection::open(&self.path)?;

        let mut stmt = conn.prepare(&format!(
            "SELECT name FROM {schema_name} WHERE type='table';"
        ))?;
        let tables = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;

                // TODO can probably query the whole db, store it in a hashmap and then key access it here
                let mut size_stmt = conn.prepare(&format!(
                    r#"SELECT SUM("pgsize") FROM "dbstat" WHERE name='{name}';"#
                ))?;
                let size: u32 = size_stmt.query_one([], |row| row.get(0))?;

                Ok(Table {
                    name,
                    schema: schema_name.to_string(),
                    size: Some(size.to_string()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tables)
    }

    async fn get_columns(
        &self,
        _schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<Column>, Box<dyn std::error::Error>> {
        let conn = SqliteConnection::open(&self.path)?;

        let mut stmt =
            conn.prepare(&format!("PRAGMA table_info('{table_name}')"))?;
        let columns = stmt
            .query_map([], |row| {
                let name = row.get(1)?;
                let data_type = row.get(2)?;
                let is_nullable = row.get(3)?;
                Ok(Column {
                    name,
                    data_type,
                    is_nullable,
                    default_value: None,
                    description: None,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(columns)
    }

    async fn get_table_data_page(
        &self,
        schema_name: &str,
        table_name: &str,
        offset: u64,
        limit: u32,
    ) -> Result<TableDataPage, Box<dyn std::error::Error>> {
        let columns: Vec<String> = self
            .get_columns(schema_name, table_name)
            .await?
            .into_iter()
            .map(|col| col.name)
            .collect();

        let conn = SqliteConnection::open(&self.path)?;

        let column_count = columns.len();
        let limit_i = i64::from(limit);
        let offset_i = i64::try_from(offset).unwrap_or(i64::MAX);
        let tq = sqlite_quote_ident(table_name);
        let col_list = columns
            .iter()
            .map(|c| sqlite_quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        let query_rowid =
            format!("SELECT rowid, {col_list} FROM {tq} LIMIT ?1 OFFSET ?2");
        let (data, row_ids) = if let Ok(mut stmt) = conn.prepare(&query_rowid) {
            let mut row_ids = Vec::new();
            let data = stmt
                .query_map(params![limit_i, offset_i], |row| {
                    let rid = row.get::<_, i64>(0)?;
                    row_ids.push(Some(DbRowId::Sqlite(rid)));
                    let values = (0..column_count)
                        .map(|i| convert_sqlite_value_to_string(row, i + 1))
                        .collect();
                    Ok(values)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            (data, row_ids)
        } else {
            let mut stmt = conn.prepare(&format!(
                "SELECT {col_list} FROM {tq} LIMIT ?1 OFFSET ?2"
            ))?;
            let data = stmt
                .query_map(params![limit_i, offset_i], |row| {
                    let values = (0..column_count)
                        .map(|i| convert_sqlite_value_to_string(row, i))
                        .collect();
                    Ok(values)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let row_ids = vec![None; data.len()];
            (data, row_ids)
        };

        Ok(TableDataPage {
            rows: data,
            column_names: columns,
            row_ids,
        })
    }

    async fn get_primary_key_columns(
        &self,
        _schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = SqliteConnection::open(&self.path)?;
        let mut stmt =
            conn.prepare(&format!("PRAGMA table_info('{table_name}')"))?;
        let mut pk_cols: Vec<(i64, String)> = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let pk: i64 = row.get(5)?;
                Ok((pk, name))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        pk_cols.retain(|(pk, _)| *pk > 0);
        pk_cols.sort_by_key(|(pk, _)| *pk);
        Ok(pk_cols.into_iter().map(|(_, n)| n).collect())
    }

    async fn update_table_cell(
        &self,
        _schema_name: &str,
        table_name: &str,
        set_column: &str,
        new_value: &str,
        primary_key: &[(String, String)],
        row_id_fallback: Option<DbRowId>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = SqliteConnection::open(&self.path)?;
        let decls = sqlite_table_decltypes(&conn, table_name)?;
        let set_kw =
            sqlite_cast_keyword(sqlite_resolve_decl(&decls, set_column));
        let tq = sqlite_quote_ident(table_name);
        let cq = sqlite_quote_ident(set_column);

        if !primary_key.is_empty() {
            let mut sql =
                format!("UPDATE {tq} SET {cq} = CAST(?1 AS {set_kw}) WHERE ");
            for (i, (k, _)) in primary_key.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                let param_num = i + 2;
                let pk_kw = sqlite_cast_keyword(sqlite_resolve_decl(&decls, k));
                let _ = write!(
                    sql,
                    "{} = CAST(?{param_num} AS {pk_kw})",
                    sqlite_quote_ident(k)
                );
            }
            let mut refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
            refs.push(&new_value);
            for (_, v) in primary_key {
                refs.push(v);
            }
            let n = u64::try_from(conn.execute(&sql, refs.as_slice())?)
                .unwrap_or(0);
            return Ok(n);
        }

        if let Some(DbRowId::Sqlite(rid)) = row_id_fallback {
            let sql = format!(
                "UPDATE {tq} SET {cq} = CAST(?1 AS {set_kw}) WHERE rowid = ?2"
            );
            let n = u64::try_from(conn.execute(&sql, params![new_value, rid])?)
                .unwrap_or(0);
            return Ok(n);
        }

        Err(
            "Cannot update row: table has no usable primary key and rowid is not available"
                .into(),
        )
    }

    async fn insert_table_row(
        &self,
        _schema_name: &str,
        table_name: &str,
        values: &[String],
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let columns = self.get_columns("sqlite_schema", table_name).await?;
        if values.len() != columns.len() {
            return Err("INSERT column count does not match table.".into());
        }
        let conn = SqliteConnection::open(&self.path)?;
        let decls = sqlite_table_decltypes(&conn, table_name)?;
        let tq = sqlite_quote_ident(table_name);
        let mut col_list = String::new();
        let mut val_parts: Vec<String> = Vec::with_capacity(columns.len());
        let mut refs: Vec<rusqlite::types::Value> = Vec::new();
        for (i, c) in columns.iter().enumerate() {
            if i > 0 {
                col_list.push_str(", ");
            }
            col_list.push_str(&sqlite_quote_ident(&c.name));
        }
        for (i, c) in columns.iter().enumerate() {
            let raw = values.get(i).map(String::as_str).unwrap_or("");
            let as_null = c.is_nullable
                && (raw.is_empty() || raw.eq_ignore_ascii_case("null"));
            if as_null {
                val_parts.push("NULL".to_string());
            } else {
                let kw =
                    sqlite_cast_keyword(sqlite_resolve_decl(&decls, &c.name));
                val_parts.push(format!("CAST(? AS {kw})"));
                refs.push(rusqlite::types::Value::Text(raw.to_string()));
            }
        }
        let sql = format!(
            "INSERT INTO {tq} ({col_list}) VALUES ({})",
            val_parts.join(", ")
        );
        let mut pvec: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for v in &refs {
            pvec.push(v);
        }
        let n = u64::try_from(conn.execute(&sql, pvec.as_slice())?).unwrap_or(0);
        Ok(n)
    }

    async fn delete_table_row(
        &self,
        _schema_name: &str,
        table_name: &str,
        primary_key: &[(String, String)],
        row_id_fallback: Option<DbRowId>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = SqliteConnection::open(&self.path)?;
        let decls = sqlite_table_decltypes(&conn, table_name)?;
        let tq = sqlite_quote_ident(table_name);
        if !primary_key.is_empty() {
            let mut sql = format!("DELETE FROM {tq} WHERE ");
            for (i, (k, _)) in primary_key.iter().enumerate() {
                if i > 0 {
                    sql.push_str(" AND ");
                }
                let param_num = i + 1;
                let pk_kw = sqlite_cast_keyword(sqlite_resolve_decl(&decls, k));
                let _ = write!(
                    sql,
                    "{} = CAST(?{param_num} AS {pk_kw})",
                    sqlite_quote_ident(k)
                );
            }
            let mut refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
            for (_, v) in primary_key {
                refs.push(v);
            }
            let n = u64::try_from(conn.execute(&sql, refs.as_slice())?)
                .unwrap_or(0);
            return Ok(n);
        }
        if let Some(DbRowId::Sqlite(rid)) = row_id_fallback {
            let sql = format!("DELETE FROM {tq} WHERE rowid = ?1");
            let n = u64::try_from(conn.execute(&sql, params![rid])?).unwrap_or(0);
            return Ok(n);
        }
        Err("Cannot delete row: no primary key and no rowid".into())
    }

    async fn get_table_row_count(
        &self,
        _schema_name: &str,
        table_name: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = SqliteConnection::open(&self.path)?;
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM {table_name}"),
            [],
            |row| row.get(0),
        )?;
        Ok(count.cast_unsigned())
    }

    async fn get_databases(
        &self,
    ) -> Result<Vec<DatabaseInfo>, Box<dyn std::error::Error>> {
        // SQLite doesn't have multiple databases per connection
        // Return a single database with the path as the name
        Ok(vec![DatabaseInfo {
            name: self.path.clone(),
        }])
    }
}

impl Sqlite {
    fn get_connection(
        &self,
    ) -> Result<SqliteConnection, Box<dyn std::error::Error>> {
        // TODO move to field in Sqlite
        Ok(SqliteConnection::open(&self.path)?)
    }
}

/// Initialize the database with migrations.
///
/// Base schema: Name, Type, Url, Environment, Metadata (JSONB stored as TEXT).
///
/// # Errors
///
/// This function will return an error if the database cannot be opened or if migrations fail.
pub fn init_db() -> Result<()> {
    let db_path = get_db_path()?;
    let mut conn = SqliteConnection::open(db_path)?;

    // Base schema: Name, Type, Url, Environment, Metadata (JSONB as TEXT).
    let migrations = Migrations::new(vec![
        M::up(
            "CREATE TABLE IF NOT EXISTS connections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                type TEXT NOT NULL CHECK( type IN ('postgres','sqlite') ),
                url TEXT NOT NULL,
                environment TEXT NOT NULL CHECK( environment IN ('local', 'dev','staging','prod') ),
                metadata TEXT
            );",
        )
        .down("DROP TABLE connections"),
    ]);

    migrations.to_latest(&mut conn)?;

    Ok(())
}

/// Build metadata JSON for storage (includes `password_storage` if set).
fn metadata_for_save(connection: &Connection) -> String {
    let mut obj = match &connection.metadata {
        serde_json::Value::Object(m) => m.clone(),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_)
        | serde_json::Value::Array(_) => serde_json::Map::new(),
    };
    if let Some(ref ps) = connection.password_storage {
        obj.insert(
            "password_storage".to_string(),
            serde_json::Value::String(ps.clone()),
        );
    }
    serde_json::Value::Object(obj).to_string()
}

/// Parse metadata from DB and extract `password_storage`.
fn metadata_from_row(
    metadata_json: Option<&String>,
) -> (serde_json::Value, Option<String>) {
    let mut password_storage = None;
    let value = metadata_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    if let Some(obj) = value.as_object()
        && let Some(ps) = obj.get("password_storage").and_then(|v| v.as_str())
    {
        password_storage = Some(ps.to_string());
    }
    (value, password_storage)
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

    let metadata = metadata_for_save(connection);

    conn.execute(
        "INSERT INTO connections (name, type, url, environment, metadata) VALUES (?, ?, ?, ?, ?)",
        params![
            connection.name,
            connection.r#type.to_string(),
            connection.url,
            connection.environment.to_string(),
            metadata,
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

    let mut stmt = conn.prepare(
        "SELECT name, type, url, environment, metadata FROM connections ORDER BY name",
    )?;
    let connections = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let type_str: String = row.get(1)?;
            let url: String = row.get(2)?;
            let env_str: String = row.get(3)?;
            let metadata_str: Option<String> = row.get(4)?;

            let r#type = type_str.parse().unwrap_or(ConnectionType::Postgres);
            let environment = env_str.parse().unwrap_or(Environment::Dev);
            let (metadata, password_storage) =
                metadata_from_row(metadata_str.as_ref());

            Ok(Connection {
                name,
                r#type,
                url,
                environment,
                metadata,
                selected_database: None,
                schema: None,
                table: None,
                password: None,
                password_storage,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(connections)
}

/// Update a connection in the database
///
/// # Errors
///
/// This function will return an error if the urlUrlUrlurlurlot be opened or if the query fails.
/// // TODO test that the path exists
pub fn update_connection(
    old_name: &str,
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_db_path()?;
    let conn = SqliteConnection::open(db_path)?;

    let metadata = metadata_for_save(connection);

    conn.execute(
        "UPDATE connections SET name = ?, type = ?, url = ?, environment = ?, metadata = ? WHERE name = ?",
        params![
            connection.name,
            connection.r#type.to_string(),
            connection.url,
            connection.environment.to_string(),
            metadata,
            old_name,
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
