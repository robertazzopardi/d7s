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

        if !rows.is_empty() {
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
        } else {
            // For queries that don't return rows (INSERT, UPDATE, DELETE, etc.)
            // Return a single row with the affected row count
            let affected_rows = client.execute(sql, &[]).await?;
            result.push(TableRow {
                values: vec![format!("Affected rows: {}", affected_rows)],
                column_names: vec!["Result".to_string()],
            });
        }

        Ok(result)
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

    /// Get all schemas in the database
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_schemas(
        &self,
    ) -> Result<Vec<Schema>, tokio_postgres::Error> {
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

    /// Get all tables in a schema
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_tables(
        &self,
        schema_name: &str,
    ) -> Result<Vec<Table>, tokio_postgres::Error> {
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

    /// Get all columns in a table
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<Column>, tokio_postgres::Error> {
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
                let value: Option<String> = row.get(i);
                values.push(value.unwrap_or_else(|| "NULL".to_string()));
            }
            data.push(values);
        }

        Ok(data)
    }

    /// Get table data as `TableRow` objects
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_table_data(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<TableRow>, tokio_postgres::Error> {
        let client = self.get_connection().await?;

        let query =
            format!("SELECT * FROM {schema_name}.{table_name} LIMIT 100");

        let rows = client.query(&query, &[]).await?;
        let mut table_rows = Vec::new();
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
                let value: Option<String> = row.get(i);
                values.push(value.unwrap_or_else(|| "NULL".to_string()));
            }
            table_rows.push(TableRow {
                values,
                column_names: column_names.clone(),
            });
        }

        Ok(table_rows)
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

    /// Get table data with column names
    ///
    /// # Errors
    ///
    /// This function will return an error if the query fails.
    pub async fn get_table_data_with_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<(Vec<Vec<String>>, Vec<String>), tokio_postgres::Error> {
        self.get_table_data_with_columns_simple(schema_name, table_name)
            .await
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
        "json" | "jsonb" => get_json(row, index),
        _ if type_name.ends_with("[]") => get_vec_string(row, index, type_name),
        "text" | "varchar" | "char" | "character varying" | "character"
        | "float4" | "real" | "float8" | "double precision" => {
            get_string::<String>(row, index)
        }
        "uuid" => get_string::<Uuid>(row, index),
        "int2" | "smallint" | "int4" | "integer" | "int8" | "bigint"
        | "numeric" | "decimal" => get_string::<String>(row, index),
        "bool" | "boolean" => get_bool(row, index),
        "timestamp" | "timestamptz" | "date" | "time" => {
            get_string::<DateTime<Utc>>(row, index)
        }
        "bytea" => get_bytes(row, index),
        _ => get_custom(row, index, type_name),
    }
}

fn get_string<'a, T: ToString + FromSql<'a>>(
    row: &'a tokio_postgres::Row,
    index: usize,
) -> String {
    row.try_get::<_, Option<T>>(index).map_or_else(
        |_| "NULL".to_string(),
        |v| v.map_or_else(|| "NULL".to_string(), |x| x.to_string()),
    )
}

fn get_vec_string(
    row: &tokio_postgres::Row,
    index: usize,
    type_name: &str,
) -> String {
    row.try_get::<_, Option<Vec<String>>>(index).map_or_else(
        |_| {
            row.try_get::<_, Option<String>>(index).map_or_else(
                |_| format!("<{type_name}>"),
                |value| value.unwrap_or_else(|| "NULL".to_string()),
            )
        },
        |value| {
            value.map_or_else(
                || "NULL".to_string(),
                |arr| format!("[{}]", arr.join(", ")),
            )
        },
    )
}

fn get_bool(row: &tokio_postgres::Row, index: usize) -> String {
    row.try_get::<_, Option<bool>>(index).map_or_else(
        |_| "NULL".to_string(),
        |value| value.map_or_else(|| "NULL".to_string(), |b| b.to_string()),
    )
}

fn get_bytes(row: &tokio_postgres::Row, index: usize) -> String {
    row.try_get::<_, Option<Vec<u8>>>(index).map_or_else(
        |_| "NULL".to_string(),
        |value| {
            value.map_or_else(
                || "NULL".to_string(),
                |bytes| format!("<{} bytes>", bytes.len()),
            )
        },
    )
}

fn get_json(row: &tokio_postgres::Row, index: usize) -> String {
    row.try_get::<_, Option<serde_json::Value>>(index)
        .map_or_else(
            |_| "NULL".to_string(),
            |value| value.map_or_else(|| "NULL".to_string(), |v| v.to_string()),
        )
}

fn get_custom(
    row: &tokio_postgres::Row,
    index: usize,
    type_name: &str,
) -> String {
    row.try_get::<_, Option<String>>(index).map_or_else(
        |_| {
            row.try_get::<_, Option<serde_json::Value>>(index)
                .map_or_else(
                    |_| format!("<{type_name}>"),
                    |value| {
                        value.map_or_else(
                            || "NULL".to_string(),
                            |v| v.to_string(),
                        )
                    },
                )
        },
        |value| value.unwrap_or_else(|| "NULL".to_string()),
    )
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
