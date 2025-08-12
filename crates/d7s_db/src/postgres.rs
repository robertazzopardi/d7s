use tokio_postgres::NoTls;

use crate::{Column, Database, Schema, Table, TableData};

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

        let Ok(_) = tokio_postgres::connect(&config, NoTls).await else {
            return false;
        };

        true
    }
}

impl Postgres {
    /// Get a connection to the database
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
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(client)
    }

    /// Get all schemas in the database
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

        dbg!(&query, &schema_name);
        let rows = client.query(query, &[&schema_name]).await?;
        let mut tables = Vec::new();

        dbg!(&rows);
        for row in rows {
            let table = Table {
                name: row.get(0),
                schema: row.get(1),
                size: row.get(3),
            };
            tables.push(table);
        }

        Ok(tables)
    }

    /// Get all columns in a table
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
    pub async fn get_sample_data(
        &self,
        schema_name: &str,
        table_name: &str,
        limit: i64,
    ) -> Result<Vec<Vec<String>>, tokio_postgres::Error> {
        let client = self.get_connection().await?;

        let query =
            format!("SELECT * FROM {}.{} LIMIT $1", schema_name, table_name);

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
