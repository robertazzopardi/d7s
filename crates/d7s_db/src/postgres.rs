use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use tokio_postgres::{
    NoTls, Row,
    types::{FromSql, Type},
};
use uuid::Uuid;

use crate::{
    Column, Database, DatabaseInfo, Schema, Table, TableData, TableRow,
};

#[derive(Debug, Clone, Default)]
pub struct Postgres {
    pub name: String,
    pub host: Option<String>,
    pub port: Option<String>,
    pub user: String,
    pub database: String,
    pub password: String,
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
            let affected_rows = client.execute(sql, &[]).await?;
            result.push(TableRow {
                values: vec![format!("Affected rows: {}", affected_rows)],
                column_names: vec!["Result".to_string()],
            });
        } else {
            let column_names: Vec<String> = rows[0]
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();

            for row in &rows {
                let values = row
                    .columns()
                    .iter()
                    .enumerate()
                    .map(|(i, col)| column_to_string(row, i, col.type_()))
                    .collect();
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

    async fn get_databases(
        &self,
    ) -> Result<Vec<DatabaseInfo>, Box<dyn std::error::Error>> {
        let client = self.get_connection().await?;

        let query = "
            SELECT datname
            FROM pg_database
            WHERE datistemplate = false
            ORDER BY datname
        ";

        let rows = client.query(query, &[]).await?;

        Ok(rows
            .iter()
            .map(|row| DatabaseInfo { name: row.get(0) })
            .collect())
    }
}

impl Postgres {
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

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {e}");
            }
        });

        Ok(client)
    }

    /// Retrieves sample data from a table.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or the query cannot be executed.
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

        for row in &rows {
            let values = row
                .columns()
                .iter()
                .enumerate()
                .map(|(i, col)| column_to_string(row, i, col.type_()))
                .collect();
            data.push(values);
        }

        Ok(data)
    }

    /// Retrieves table data along with column names.
    ///
    /// # Errors
    ///
    /// Returns an error if the database connection fails or the query cannot be executed.
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

        if let Some(first_row) = rows.first() {
            for i in 0..first_row.len() {
                column_names.push(first_row.columns()[i].name().to_string());
            }
        }

        for row in &rows {
            let values = row
                .columns()
                .iter()
                .enumerate()
                .map(|(i, col)| column_to_string(row, i, col.type_()))
                .collect();

            data.push(values);
        }

        Ok((data, column_names))
    }
}

/// Try and convert the value of the row to a string based on the type of the
/// column.
///
/// The types used to check against are gathered from:
/// <https://docs.rs/tokio-postgres/latest/tokio_postgres/types/struct.Type.html>
#[allow(clippy::too_many_lines)]
fn column_to_string(row: &Row, index: usize, ty: &Type) -> String {
    if row.is_empty() {
        return "NULL".to_string();
    }

    match *ty {
        // Boolean
        Type::BOOL => try_get::<bool>(row, index),

        // Binary data
        Type::BYTEA => try_get_bytes(row, index),

        // Integer types
        Type::CHAR => try_get::<i8>(row, index),
        Type::INT2 => try_get::<i16>(row, index),
        Type::INT4 => try_get::<i32>(row, index),
        Type::INT8 => try_get::<i64>(row, index),

        // Floating point types
        Type::FLOAT4 => try_get::<f32>(row, index),
        Type::FLOAT8 => try_get::<f64>(row, index),

        // Object identifier types
        Type::OID => try_get::<u32>(row, index),

        // UUID
        Type::UUID => try_get::<Uuid>(row, index),

        // JSON types
        Type::JSON | Type::JSONB => try_get::<serde_json::Value>(row, index),

        // Date/Time types with special handling
        Type::TIMESTAMP => try_get_timestamp(row, index),
        Type::TIMESTAMPTZ => try_get_timestamptz(row, index),
        Type::DATE => try_get_date(row, index),
        Type::TIME | Type::TIMETZ => try_get_time(row, index),

        // Typed array types
        Type::BOOL_ARRAY => try_get_array::<bool>(row, index),
        Type::CHAR_ARRAY => try_get_array::<i8>(row, index),
        Type::INT2_ARRAY => try_get_array::<i16>(row, index),
        Type::INT4_ARRAY => try_get_array::<i32>(row, index),
        Type::INT8_ARRAY => try_get_array::<i64>(row, index),
        Type::FLOAT4_ARRAY => try_get_array::<f32>(row, index),
        Type::FLOAT8_ARRAY => try_get_array::<f64>(row, index),
        Type::OID_ARRAY => try_get_array::<u32>(row, index),
        Type::UUID_ARRAY => try_get_array::<Uuid>(row, index),
        Type::TEXT_ARRAY
        | Type::VARCHAR_ARRAY
        | Type::BPCHAR_ARRAY
        | Type::NAME_ARRAY => try_get_array::<String>(row, index),

        // Numeric types with rust_decimal
        Type::NUMERIC => try_get_numeric(row, index),
        Type::NUMERIC_ARRAY => try_get_numeric_array(row, index),

        // All other types as strings - consolidated for clippy::match_same_arms
        Type::INTERVAL
        | Type::TEXT
        | Type::VARCHAR
        | Type::BPCHAR
        | Type::NAME
        | Type::UNKNOWN
        | Type::INET
        | Type::CIDR
        | Type::MACADDR
        | Type::MACADDR8
        | Type::XML
        | Type::BIT
        | Type::VARBIT
        | Type::MONEY
        | Type::TID
        | Type::XID
        | Type::CID
        | Type::REGPROC
        | Type::REGPROCEDURE
        | Type::REGOPER
        | Type::REGOPERATOR
        | Type::REGCLASS
        | Type::REGTYPE
        | Type::REGNAMESPACE
        | Type::REGROLE
        | Type::REGCONFIG
        | Type::REGDICTIONARY
        | Type::REGCOLLATION
        | Type::POINT
        | Type::LSEG
        | Type::PATH
        | Type::BOX
        | Type::POLYGON
        | Type::LINE
        | Type::CIRCLE
        | Type::TS_VECTOR
        | Type::TSQUERY
        | Type::GTS_VECTOR
        | Type::TID_ARRAY
        | Type::XID_ARRAY
        | Type::CID_ARRAY
        | Type::INT4_RANGE
        | Type::INT8_RANGE
        | Type::NUM_RANGE
        | Type::TS_RANGE
        | Type::TSTZ_RANGE
        | Type::DATE_RANGE
        | Type::INT4MULTI_RANGE
        | Type::INT8MULTI_RANGE
        | Type::NUMMULTI_RANGE
        | Type::TSMULTI_RANGE
        | Type::TSTZMULTI_RANGE
        | Type::DATEMULTI_RANGE
        | Type::PG_LSN
        | Type::PG_SNAPSHOT
        | Type::TXID_SNAPSHOT
        | Type::PG_NDISTINCT
        | Type::PG_DEPENDENCIES
        | Type::PG_MCV_LIST
        | Type::PG_BRIN_BLOOM_SUMMARY
        | Type::PG_BRIN_MINMAX_MULTI_SUMMARY
        | Type::JSONPATH
        | Type::XID8
        | Type::ACLITEM
        | Type::REFCURSOR
        | Type::BYTEA_ARRAY
        | Type::TIMESTAMP_ARRAY
        | Type::TIMESTAMPTZ_ARRAY
        | Type::DATE_ARRAY
        | Type::TIME_ARRAY
        | Type::TIMETZ_ARRAY
        | Type::INTERVAL_ARRAY
        | Type::INET_ARRAY
        | Type::CIDR_ARRAY
        | Type::MACADDR_ARRAY
        | Type::MACADDR8_ARRAY
        | Type::JSON_ARRAY
        | Type::JSONB_ARRAY
        | Type::REGPROC_ARRAY
        | Type::REGPROCEDURE_ARRAY
        | Type::REGOPER_ARRAY
        | Type::REGOPERATOR_ARRAY
        | Type::REGCLASS_ARRAY
        | Type::REGTYPE_ARRAY
        | Type::REGNAMESPACE_ARRAY
        | Type::REGROLE_ARRAY
        | Type::REGCONFIG_ARRAY
        | Type::REGDICTIONARY_ARRAY
        | Type::REGCOLLATION_ARRAY
        | Type::POINT_ARRAY
        | Type::LSEG_ARRAY
        | Type::PATH_ARRAY
        | Type::BOX_ARRAY
        | Type::POLYGON_ARRAY
        | Type::LINE_ARRAY
        | Type::CIRCLE_ARRAY
        | Type::BIT_ARRAY
        | Type::VARBIT_ARRAY
        | Type::MONEY_ARRAY
        | Type::TS_VECTOR_ARRAY
        | Type::TSQUERY_ARRAY
        | Type::GTS_VECTOR_ARRAY
        | Type::INT4_RANGE_ARRAY
        | Type::INT8_RANGE_ARRAY
        | Type::NUM_RANGE_ARRAY
        | Type::TS_RANGE_ARRAY
        | Type::TSTZ_RANGE_ARRAY
        | Type::DATE_RANGE_ARRAY
        | Type::INT4MULTI_RANGE_ARRAY
        | Type::INT8MULTI_RANGE_ARRAY
        | Type::NUMMULTI_RANGE_ARRAY
        | Type::TSMULTI_RANGE_ARRAY
        | Type::TSTZMULTI_RANGE_ARRAY
        | Type::DATEMULTI_RANGE_ARRAY
        | Type::XML_ARRAY
        | Type::JSONPATH_ARRAY
        | Type::XID8_ARRAY
        | Type::PG_LSN_ARRAY
        | Type::PG_SNAPSHOT_ARRAY
        | Type::TXID_SNAPSHOT_ARRAY
        | Type::ACLITEM_ARRAY
        | Type::REFCURSOR_ARRAY
        | Type::CSTRING_ARRAY
        | Type::INT2_VECTOR
        | Type::OID_VECTOR
        | Type::INT2_VECTOR_ARRAY
        | Type::OID_VECTOR_ARRAY
        | Type::PG_DDL_COMMAND
        | Type::PG_NODE_TREE
        | Type::TABLE_AM_HANDLER
        | Type::INDEX_AM_HANDLER
        | Type::TSM_HANDLER
        | Type::FDW_HANDLER
        | Type::LANGUAGE_HANDLER
        | Type::INTERNAL
        | Type::EVENT_TRIGGER
        | Type::TRIGGER
        | Type::VOID
        | Type::RECORD
        | Type::CSTRING
        | Type::ANY
        | Type::ANYARRAY
        | Type::ANYELEMENT
        | Type::ANYNONARRAY
        | Type::ANYENUM
        | Type::ANY_RANGE
        | Type::ANYMULTI_RANGE
        | Type::ANYCOMPATIBLE
        | Type::ANYCOMPATIBLEARRAY
        | Type::ANYCOMPATIBLENONARRAY
        | Type::ANYCOMPATIBLE_RANGE
        | Type::ANYCOMPATIBLEMULTI_RANGE
        | Type::RECORD_ARRAY => try_get::<String>(row, index),

        // Fallback for unknown types
        _ => row
            .columns()
            .get(index)
            .map(|c| c.type_().name())
            .map_or_else(
                || try_get::<String>(row, index),
                |type_name| try_get_or_label::<String>(row, index, type_name),
            ),
    }
}

/// Generic helper to get a value and convert it to a string, handling NULL values
fn try_get<'a, T: ToString + FromSql<'a>>(
    row: &'a Row,
    index: usize,
) -> String {
    row.try_get::<_, Option<T>>(index)
        .ok()
        .flatten()
        .map_or_else(|| "NULL".to_string(), |v| v.to_string())
}

/// Helper to get array values and format them as a comma-separated list
fn try_get_array<'a, T: ToString + FromSql<'a>>(
    row: &'a Row,
    index: usize,
) -> String {
    row.try_get::<_, Option<Vec<T>>>(index)
        .ok()
        .flatten()
        .map(|arr| {
            arr.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .map_or_else(|| "NULL".to_string(), |s| format!("[{s}]"))
}

/// Helper to get binary data and display the byte count
fn try_get_bytes(row: &Row, index: usize) -> String {
    row.try_get::<_, Option<Vec<u8>>>(index)
        .ok()
        .flatten()
        .map_or_else(
            || "NULL".to_string(),
            |bytes| format!("<{} bytes>", bytes.len()),
        )
}

/// Fallback helper for unknown types - tries to get as string or shows type label
fn try_get_or_label<'a, T: ToString + FromSql<'a>>(
    row: &'a Row,
    index: usize,
    type_name: &str,
) -> String {
    if let Ok(Some(v)) = row.try_get::<_, Option<T>>(index) {
        v.to_string()
    } else {
        format!("<{type_name}>")
    }
}

/// Helper to get NUMERIC values using `rust_decimal`
fn try_get_numeric(row: &Row, index: usize) -> String {
    row.try_get::<_, Option<Decimal>>(index)
        .ok()
        .flatten()
        .map_or_else(|| "NULL".to_string(), |v| v.to_string())
}

/// Helper to get NUMERIC array values
fn try_get_numeric_array(row: &Row, index: usize) -> String {
    row.try_get::<_, Option<Vec<Decimal>>>(index)
        .ok()
        .flatten()
        .map(|arr| {
            arr.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .map_or_else(|| "NULL".to_string(), |s| format!("[{s}]"))
}

/// Helper to get TIMESTAMP values with better compatibility
/// Tries multiple datetime representations for robustness
fn try_get_timestamp(row: &Row, index: usize) -> String {
    // Try NaiveDateTime first (TIMESTAMP WITHOUT TIME ZONE)
    if let Ok(Some(v)) = row.try_get::<_, Option<NaiveDateTime>>(index) {
        return v.to_string();
    }

    // Fallback to DateTime<Utc> in case it's stored with timezone info
    if let Ok(Some(v)) = row.try_get::<_, Option<DateTime<Utc>>>(index) {
        return v.naive_utc().to_string();
    }

    // Final fallback to string
    try_get::<String>(row, index)
}

/// Helper to get TIMESTAMPTZ values with better compatibility
/// Handles timezone-aware timestamps
fn try_get_timestamptz(row: &Row, index: usize) -> String {
    // Try DateTime<Utc> first (TIMESTAMP WITH TIME ZONE)
    if let Ok(Some(v)) = row.try_get::<_, Option<DateTime<Utc>>>(index) {
        return v.to_rfc3339();
    }

    // Fallback to NaiveDateTime if stored without timezone
    if let Ok(Some(v)) = row.try_get::<_, Option<NaiveDateTime>>(index) {
        return v.to_string();
    }

    // Final fallback to string
    try_get::<String>(row, index)
}

/// Helper to get DATE values
fn try_get_date(row: &Row, index: usize) -> String {
    if let Ok(Some(v)) = row.try_get::<_, Option<NaiveDate>>(index) {
        return v.format("%Y-%m-%d").to_string();
    }

    // Fallback to string
    try_get::<String>(row, index)
}

/// Helper to get TIME values
fn try_get_time(row: &Row, index: usize) -> String {
    if let Ok(Some(v)) = row.try_get::<_, Option<NaiveTime>>(index) {
        return v.format("%H:%M:%S%.f").to_string();
    }

    // Fallback to string
    try_get::<String>(row, index)
}
