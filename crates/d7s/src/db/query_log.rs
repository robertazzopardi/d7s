//! Session query log: every statement d7s itself sends to a database.
//!
//! Recorded at one choke point, the [`LoggedDatabase`] decorator around the
//! [`Database`] trait, so Postgres and `SQLite` (and any future backend) are
//! covered without per-caller changes. Only SQL text / labels are stored;
//! never passwords, connection strings or edited cell values.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use k9tui::widgets::table::TableData;
use ratatui::style::Style;

use super::{
    Column, Database, DatabaseInfo, DbRowId, Schema, Table, TableDataPage,
    TableRow,
};

/// Ring buffer capacity.
pub const QUERY_LOG_CAPACITY: usize = 500;
const SQL_DISPLAY_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOrigin {
    User,
    Watch,
    Activity,
    Metadata,
}

impl QueryOrigin {
    const fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Watch => "watch",
            Self::Activity => "activity",
            Self::Metadata => "metadata",
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryLogEntry {
    pub at: SystemTime,
    pub origin: QueryOrigin,
    pub duration: Duration,
    pub sql: String,
    /// `Ok(rows)` or `Err(message)`.
    pub outcome: Result<usize, String>,
}

impl TableData for QueryLogEntry {
    fn title() -> &'static str {
        "Query log"
    }

    fn ref_array(&self) -> Vec<String> {
        let secs = self
            .at
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let day = secs % 86_400;
        let outcome = match &self.outcome {
            Ok(n) => format!("{n} rows"),
            Err(e) => format!("ERR {}", one_line(e)),
        };
        vec![
            format!("{:02}:{:02}:{:02}", day / 3600, day % 3600 / 60, day % 60),
            self.origin.label().to_string(),
            format!("{}ms", self.duration.as_millis()),
            outcome,
            one_line(&self.sql)
                .chars()
                .take(SQL_DISPLAY_CHARS)
                .collect(),
        ]
    }

    fn num_columns(&self) -> usize {
        5
    }

    fn cols() -> Vec<&'static str> {
        vec!["Time (UTC)", "Origin", "Duration", "Rows/Error", "SQL"]
    }

    fn cell_style(&self, column: usize) -> Option<Style> {
        if column == 3 && self.outcome.is_err() {
            return Some(k9tui::theme::error());
        }
        None
    }
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Default)]
struct Inner {
    entries: VecDeque<QueryLogEntry>,
    /// Tag applied to the next `execute_sql` calls (set by the app).
    origin: Option<QueryOrigin>,
}

/// Shared, bounded, cheap-to-clone handle.
#[derive(Clone, Default)]
pub struct QueryLog(Arc<Mutex<Inner>>);

impl QueryLog {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn push(&self, entry: QueryLogEntry) {
        let mut g = self.lock();
        if g.entries.len() >= QUERY_LOG_CAPACITY {
            g.entries.pop_front();
        }
        g.entries.push_back(entry);
    }

    /// Origin used for subsequent `execute_sql` calls (default `User`).
    pub fn set_origin(&self, origin: QueryOrigin) {
        self.lock().origin = Some(origin);
    }

    fn sql_origin(&self) -> QueryOrigin {
        self.lock().origin.unwrap_or(QueryOrigin::User)
    }

    /// Newest first.
    #[must_use]
    pub fn snapshot(&self) -> Vec<QueryLogEntry> {
        self.lock().entries.iter().rev().cloned().collect()
    }

    /// Wrap a backend so everything it runs is recorded here.
    #[must_use]
    pub fn wrap(&self, inner: Box<dyn Database>) -> Box<dyn Database> {
        Box::new(LoggedDatabase {
            inner,
            log: self.clone(),
        })
    }
}

/// Decorator recording every [`Database`] call into a [`QueryLog`].
pub struct LoggedDatabase {
    inner: Box<dyn Database>,
    log: QueryLog,
}

impl LoggedDatabase {
    async fn record<T, F>(
        &self,
        origin: QueryOrigin,
        sql: String,
        count: fn(&T) -> usize,
        fut: F,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let (at, start) = (SystemTime::now(), Instant::now());
        let res = fut.await;
        self.log.push(QueryLogEntry {
            at,
            origin,
            duration: start.elapsed(),
            sql,
            outcome: match &res {
                Ok(v) => Ok(count(v)),
                Err(e) => Err(e.to_string()),
            },
        });
        res
    }
}

#[async_trait::async_trait]
impl Database for LoggedDatabase {
    // Connectivity probe, not a query: unlogged.
    async fn test(&self) -> bool {
        self.inner.test().await
    }

    async fn execute_sql(
        &self,
        sql: &str,
    ) -> Result<Vec<TableRow>, Box<dyn std::error::Error>> {
        self.record(
            self.log.sql_origin(),
            sql.to_string(),
            Vec::len,
            self.inner.execute_sql(sql),
        )
        .await
    }

    // The metadata methods below log a descriptive label rather than the
    // backend's SQL (which lives inside each impl); no values are included.
    async fn get_schemas(
        &self,
    ) -> Result<Vec<Schema>, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            "get_schemas".into(),
            Vec::len,
            self.inner.get_schemas(),
        )
        .await
    }

    async fn get_tables(
        &self,
        schema_name: &str,
    ) -> Result<Vec<Table>, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            format!("get_tables {schema_name}"),
            Vec::len,
            self.inner.get_tables(schema_name),
        )
        .await
    }

    async fn get_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<Column>, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            format!("get_columns {schema_name}.{table_name}"),
            Vec::len,
            self.inner.get_columns(schema_name, table_name),
        )
        .await
    }

    async fn get_table_data_page(
        &self,
        schema_name: &str,
        table_name: &str,
        offset: u64,
        limit: u32,
    ) -> Result<TableDataPage, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            format!(
                "get_table_data_page {schema_name}.{table_name} offset={offset} limit={limit}"
            ),
            |p: &TableDataPage| p.rows.len(),
            self.inner
                .get_table_data_page(schema_name, table_name, offset, limit),
        )
        .await
    }

    async fn get_primary_key_columns(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            format!("get_primary_key_columns {schema_name}.{table_name}"),
            Vec::len,
            self.inner.get_primary_key_columns(schema_name, table_name),
        )
        .await
    }

    async fn update_table_cell(
        &self,
        schema_name: &str,
        table_name: &str,
        set_column: &str,
        new_value: &str,
        primary_key: &[(String, String)],
        row_id_fallback: Option<DbRowId>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::User,
            format!(
                "update_table_cell {schema_name}.{table_name}.{set_column}"
            ),
            |n: &u64| usize::try_from(*n).unwrap_or(usize::MAX),
            self.inner.update_table_cell(
                schema_name,
                table_name,
                set_column,
                new_value,
                primary_key,
                row_id_fallback,
            ),
        )
        .await
    }

    async fn insert_table_row(
        &self,
        schema_name: &str,
        table_name: &str,
        values: &[String],
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::User,
            format!("insert_table_row {schema_name}.{table_name}"),
            |n: &u64| usize::try_from(*n).unwrap_or(usize::MAX),
            self.inner.insert_table_row(schema_name, table_name, values),
        )
        .await
    }

    async fn delete_table_row(
        &self,
        schema_name: &str,
        table_name: &str,
        primary_key: &[(String, String)],
        row_id_fallback: Option<DbRowId>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::User,
            format!("delete_table_row {schema_name}.{table_name}"),
            |n: &u64| usize::try_from(*n).unwrap_or(usize::MAX),
            self.inner.delete_table_row(
                schema_name,
                table_name,
                primary_key,
                row_id_fallback,
            ),
        )
        .await
    }

    async fn get_table_row_count(
        &self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            format!("get_table_row_count {schema_name}.{table_name}"),
            |_: &u64| 1,
            self.inner.get_table_row_count(schema_name, table_name),
        )
        .await
    }

    async fn get_databases(
        &self,
    ) -> Result<Vec<DatabaseInfo>, Box<dyn std::error::Error>> {
        self.record(
            QueryOrigin::Metadata,
            "get_databases".into(),
            Vec::len,
            self.inner.get_databases(),
        )
        .await
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db::sqlite::Sqlite;

    fn entry(sql: &str) -> QueryLogEntry {
        QueryLogEntry {
            at: UNIX_EPOCH,
            origin: QueryOrigin::User,
            duration: Duration::from_millis(3),
            sql: sql.to_string(),
            outcome: Ok(1),
        }
    }

    /// Unique temp `SQLite` file path (each `Sqlite` call opens a fresh
    /// connection, so `:memory:` would not persist between calls).
    pub fn temp_sqlite(tag: &str) -> Sqlite {
        let path = std::env::temp_dir().join(format!(
            "d7s-test-{tag}-{}-{:?}.db",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&path);
        Sqlite {
            name: tag.to_string(),
            path: path.to_string_lossy().into_owned(),
        }
    }

    #[test]
    fn ring_buffer_caps_and_orders_newest_first() {
        let log = QueryLog::default();
        for i in 0..QUERY_LOG_CAPACITY + 10 {
            log.push(entry(&format!("q{i}")));
        }
        let snap = log.snapshot();
        assert_eq!(snap.len(), QUERY_LOG_CAPACITY);
        assert_eq!(snap.first().map(|e| e.sql.as_str()), Some("q509"));
        // oldest 10 evicted
        assert_eq!(snap.last().map(|e| e.sql.as_str()), Some("q10"));
    }

    #[test]
    fn row_formatting_truncates_and_flattens_sql() {
        let long = format!("SELECT\n  {}", "x".repeat(500));
        let cells = entry(&long).ref_array();
        assert_eq!(cells.first().map(String::as_str), Some("00:00:00"));
        assert_eq!(cells.get(2).map(String::as_str), Some("3ms"));
        let sql = cells.get(4).cloned().unwrap_or_default();
        assert_eq!(sql.chars().count(), SQL_DISPLAY_CHARS);
        assert!(sql.starts_with("SELECT xxx"));
    }

    #[tokio::test]
    async fn decorator_records_user_watch_error_and_metadata() {
        let log = QueryLog::default();
        let sqlite = temp_sqlite("decorator");
        // Seed directly: Sqlite::execute_sql re-runs row-less statements.
        rusqlite::Connection::open(&sqlite.path)
            .unwrap()
            .execute_batch("CREATE TABLE t(a); INSERT INTO t VALUES (1),(2);")
            .unwrap();
        let db = log.wrap(Box::new(sqlite));

        db.execute_sql("SELECT 1").await.unwrap();
        log.set_origin(QueryOrigin::Watch);
        db.execute_sql("SELECT * FROM t").await.unwrap();
        assert!(db.execute_sql("SELEKT nope").await.is_err());
        db.get_tables("sqlite_schema").await.unwrap();
        assert!(db.test().await); // probe is not logged

        let snap = log.snapshot();
        let got: Vec<_> =
            snap.iter().map(|e| (e.origin, e.outcome.is_ok())).collect();
        assert_eq!(
            got,
            vec![
                (QueryOrigin::Metadata, true),
                (QueryOrigin::Watch, false),
                (QueryOrigin::Watch, true),
                (QueryOrigin::User, true),
            ]
        );
        assert_eq!(snap.get(2).map(|e| e.outcome.clone()), Some(Ok(2)));
        assert_eq!(snap.get(1).map(|e| e.sql.as_str()), Some("SELEKT nope"));
    }

    /// Needs `just docker-up` (Postgres on localhost:5432):
    /// `cargo test -p d7s -- --ignored pg_activity`
    #[tokio::test]
    #[ignore = "requires docker compose Postgres"]
    async fn pg_activity_query_is_logged_and_sees_itself() {
        use crate::db::postgres::Postgres;

        let log = QueryLog::default();
        let db = log.wrap(Box::new(Postgres {
            name: "it".into(),
            host: Some("localhost".into()),
            port: Some("5432".into()),
            user: "d7s_user".into(),
            database: "d7s_test".into(),
            password: "d7s_password".into(),
        }));
        log.set_origin(QueryOrigin::Activity);
        let rows = db.execute_sql(crate::app::ACTIVITY_QUERY).await.unwrap();
        assert!(
            rows.iter().any(|r| r
                .values
                .iter()
                .any(|v| v.contains("pg_stat_activity")))
        );
        let snap = log.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap.first().map(|e| e.origin), Some(QueryOrigin::Activity));
        assert_eq!(
            snap.first().map(|e| e.outcome.clone()),
            Some(Ok(rows.len()))
        );
        // metadata path on Postgres too
        db.get_schemas().await.unwrap();
        assert_eq!(log.snapshot().len(), 2);
    }
}
