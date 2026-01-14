use chrono::{DateTime, Utc};
use tokio_postgres::{NoTls, types::FromSql};
use uuid::Uuid;

use crate::{Column, Database, Schema, Table, TableData, TableRow};

#[derive(Debug, Clone, Default)]

pub struct Postgres {
    pub name: String,
    pub host: Option<String>,
    pub port: Option<String>,
    pub user: String,
    pub database: String,
    pub password: String,
}

#[async_trait::async_trait]
impl Database for Postgres {
    async fn test(&self) -> bool {
        let config = format!(
            "host={} port={} user={} password={} dbname={}",
            self.host.clone().unwrap_or_else(|| "localhost".to_string()),
            self.port.clone().unwrap_or_else(|| "5432".to_string()),
            self.user,
            self.password,
            self.database
        );

        tokio_postgres::connect(&config, NoTls).await.is_ok()
    }

    async fn execute_sql(
        &self,
        sql: &str,
    ) -> Result<Vec<TableRow>, Box<dyn std::error::Error>> {
        let client = self.get_connection().await?;

        let rows = client.query(sql, &[]).await?;
        let mut result = Vec::new();

        if rows.is_empty() {
            // For queries that don't return rows (INSERT, UPDATE, DELETE, etc.)
            // Return a single row with the affected row count
            let affected_rows = client.execute(sql, &[]).await?;
            result.push(TableRow {
                values: vec![format!("Affected rows: {}", affected_rows)],
                column_names: vec!["Result".to_string()],
            });
        } else {
            // Get column names from the first row
            let column_names: Vec<String> = rows[0]
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();

            for row in rows {
                let mut values = Vec::new();
                for i in 0..row.columns().len() {
                    let value =
                        convert_postgres_value_to_string_simple(&row, i);
                    values.push(value);
                }
                result.push(TableRow {
                    values,
                    column_names: column_names.clone(),
                });
            }
        }

        Ok(result)
    }

    async fn get_schemas(
        &self,
    ) -> Result<Vec<Schema>, Box<dyn std::error::Error>> {
        let client = self.get_connection().await?;

        let query = "
            SELECT schema_name, schema_owner
            FROM information_schema.schemata
            WHERE schema_name NOT IN ('information_schema', 'pg_catalog', 'pg_toast')
            ORDER BY schema_name
        ";

        let rows = client.query(query, &[]).await?;
        let mut schemas = Vec::new();

        for row in rows {
            let schema = Schema {
                name: row.get(0),
                owner: row.get(1),
            };
            schemas.push(schema);
        }

        Ok(schemas)
    }

    async fn get_tables(
        &self,
        schema_name: &str,
    ) -> Result<Vec<Table>, Box<dyn std::error::Error>> {
        let client = self.get_connection().await?;

        let query = "
            SELECT
                t.table_name,
                t.table_schema,
                pg_size_pretty(pg_total_relation_size(quote_ident(t.table_schema)||'.'||quote_ident(t.table_name))) as size
            FROM information_schema.tables t
            WHERE t.table_schema = $1
            AND t.table_type = 'BASE TABLE'
            ORDER BY t.table_name;
        ";

        let rows = client.query(query, &[&schema_name]).await?;
        let mut tables = Vec::new();

        for row in rows {
            let table = Table {
                name: row.get(0),
                schema: row.get(1),
                size: row.get(2),
            };
            tables.push(table);
        }

        Ok(tables)
    }

    async fn get_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<Column>, Box<dyn std::error::Error>> {
        let client = self.get_connection().await?;

        let query = "
            SELECT
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                pgd.description
            FROM information_schema.columns c
            LEFT JOIN pg_catalog.pg_statio_all_tables st ON (c.table_schema = st.schemaname AND c.table_name = st.relname)
            LEFT JOIN pg_catalog.pg_description pgd ON (pgd.objoid = st.relid AND pgd.objsubid = c.ordinal_position)
            WHERE c.table_schema = $1
            AND c.table_name = $2
            ORDER BY c.ordinal_position
        ";

        let rows = client.query(query, &[&schema_name, &table_name]).await?;
        let mut columns = Vec::new();

        for row in rows {
            let column = Column {
                name: row.get(0),
                data_type: row.get(1),
                is_nullable: row.get::<_, String>(2) == "YES",
                default_value: row.get(3),
                description: row.get(4),
            };
            columns.push(column);
        }

        Ok(columns)
    }

    async fn get_table_data_with_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<(Vec<Vec<String>>, Vec<String>), Box<dyn std::error::Error>>
    {
        self.get_table_data_with_columns_simple(schema_name, table_name)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

impl Postgres {
    /// Get a connection to the database
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    async fn get_connection(
        &self,
    ) -> Result<tokio_postgres::Client, tokio_postgres::Error> {
        let config = format!(
            "host={} port={} user={} password={} dbname={}",
            self.host.clone().unwrap_or_else(|| "localhost".to_string()),
            self.port.clone().unwrap_or_else(|| "5432".to_string()),
            self.user,
            self.password,
            self.database
        );

        let (client, connection) =
            tokio_postgres::connect(&config, NoTls).await?;

        // Spawn the connection to run in the background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {e}");
            }
        });

        Ok(client)
    }

    /// Get sample data from a table
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_sample_data(
        &self,
        schema_name: &str,
        table_name: &str,
        limit: i64,
    ) -> Result<Vec<Vec<String>>, tokio_postgres::Error> {
        let client = self.get_connection().await?;

        let query =
            format!("SELECT * FROM {schema_name}.{table_name} LIMIT $1");

        let rows = client.query(&query, &[&limit]).await?;
        let mut data = Vec::new();

        for row in rows {
            let mut values = Vec::new();
            for i in 0..row.len() {
                let value = convert_postgres_value_to_string_simple(&row, i);
                values.push(value);
            }
            data.push(values);
        }

        Ok(data)
    }

    /// Get table data with column names (simplified version without extra dependencies)
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_table_data_with_columns_simple(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<(Vec<Vec<String>>, Vec<String>), tokio_postgres::Error> {
        let client = self.get_connection().await?;

        let query =
            format!("SELECT * FROM {schema_name}.{table_name} LIMIT 100");

        let rows = client.query(&query, &[]).await?;
        let mut data = Vec::new();
        let mut column_names = Vec::new();

        // Get column names from the first row
        if let Some(first_row) = rows.first() {
            for i in 0..first_row.len() {
                column_names.push(first_row.columns()[i].name().to_string());
            }
        }

        for row in rows {
            let mut values = Vec::new();

            for i in 0..row.len() {
                let value = convert_postgres_value_to_string_simple(&row, i);
                values.push(value);
            }

            data.push(values);
        }

        Ok((data, column_names))
    }
}

/// Convert a `PostgreSQL` value to a string representation (simplified version)
fn convert_postgres_value_to_string_simple(
    row: &tokio_postgres::Row,
    index: usize,
) -> String {
    let col_type = row.columns()[index].type_();
    let type_name = col_type.name();

    match type_name {
        // Special types that need specific handling
        "json" | "jsonb" => try_get::<serde_json::Value>(row, index),
        "bool" | "boolean" => try_get::<bool>(row, index),
        "bytea" => try_get_bytes(row, index),
        _ if type_name.ends_with("[]") => get_vec_string(row, index, type_name),

        // Integer types
        "int2" | "smallint" => try_get::<i16>(row, index),
        "int4" | "integer" => try_get::<i32>(row, index),
        "int8" | "bigint" => try_get::<i64>(row, index),

        // Numeric/decimal - try multiple types
        "numeric" | "decimal" => {
            try_get_multiple::<String, i64, f64>(row, index)
        }

        // UUID
        "uuid" => try_get::<Uuid>(row, index),

        // Timestamps
        "timestamp" | "timestamptz" | "date" | "time" => {
            try_get::<DateTime<Utc>>(row, index)
        }

        // Text and floating point - can use String
        "text" | "varchar" | "char" | "character varying" | "character"
        | "float4" | "real" | "float8" | "double precision" => {
            try_get::<String>(row, index)
        }

        // Unknown types - try common conversions
        _ => {
            let result =
                try_get_multiple::<String, serde_json::Value, i64>(row, index);
            if result == "NULL" {
                format!("<{type_name}>")
            } else {
                result
            }
        }
    }
}

/// Generic helper to try getting a value as Option<T> and convert to string
fn try_get<'a, T: ToString + FromSql<'a>>(
    row: &'a tokio_postgres::Row,
    index: usize,
) -> String {
    row.try_get::<_, Option<T>>(index)
        .ok()
        .flatten()
        .map_or_else(|| "NULL".to_string(), |v| v.to_string())
}

/// Try multiple type conversions in order, return first successful one
fn try_get_multiple<'a, T1, T2, T3>(
    row: &'a tokio_postgres::Row,
    index: usize,
) -> String
where
    T1: ToString + FromSql<'a>,
    T2: ToString + FromSql<'a>,
    T3: ToString + FromSql<'a>,
{
    if let Ok(Some(v)) = row.try_get::<_, Option<T1>>(index) {
        return v.to_string();
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<T2>>(index) {
        return v.to_string();
    }
    if let Ok(Some(v)) = row.try_get::<_, Option<T3>>(index) {
        return v.to_string();
    }
    "NULL".to_string()
}

/// Special handling for bytea to show byte count
fn try_get_bytes(row: &tokio_postgres::Row, index: usize) -> String {
    row.try_get::<_, Option<Vec<u8>>>(index)
        .ok()
        .flatten()
        .map_or_else(
            || "NULL".to_string(),
            |bytes| format!("<{} bytes>", bytes.len()),
        )
}

fn get_vec_string(
    row: &tokio_postgres::Row,
    index: usize,
    type_name: &str,
) -> String {
    // Try to get as array of strings first
    if let Ok(Some(arr)) = row.try_get::<_, Option<Vec<String>>>(index) {
        return format!("[{}]", arr.join(", "));
    }
    // Fallback to single string
    if let Ok(Some(s)) = row.try_get::<_, Option<String>>(index) {
        return s;
    }
    format!("<{type_name}>")
}

impl TableData for Postgres {
    fn title() -> &'static str {
        "Postgres"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.host.clone().unwrap_or_default(),
            self.port.clone().unwrap_or_default(),
            self.user.clone(),
            self.password.clone(),
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Host", "Port", "User", "Password"]
    }
}
