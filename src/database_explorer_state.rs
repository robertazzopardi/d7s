use d7s_db::{Column, Database, Schema, Table, connection::Connection};
use d7s_ui::widgets::table::RawTableRow;

use crate::{app_state::DatabaseExplorerState, filtered_data::FilteredData};

/// Groups all database exploration state together
pub struct DatabaseExplorer {
    /// The active database connection
    pub connection: Connection,
    /// The active database client
    pub database: Box<dyn Database>,
    /// Current navigation state in the database
    pub state: DatabaseExplorerState,
    /// Cached schema data
    pub schemas: Option<FilteredData<Schema>>,
    /// Cached table data for current schema
    pub tables: Option<FilteredData<Table>>,
    /// Cached column data for current table
    pub columns: Option<FilteredData<Column>>,
    /// Cached table row data
    pub table_data: Option<FilteredData<RawTableRow>>,
}

impl DatabaseExplorer {
    /// Create a new `DatabaseExplorer` with a connection and database client
    pub fn new(connection: Connection, database: Box<dyn Database>) -> Self {
        Self {
            connection,
            database,
            state: DatabaseExplorerState::Schemas,
            schemas: None,
            tables: None,
            columns: None,
            table_data: None,
        }
    }

    /// Execute a function with mutable access to schemas if currently viewing schemas
    pub fn with_schemas<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut FilteredData<Schema>) -> R,
    {
        matches!(self.state, DatabaseExplorerState::Schemas)
            .then(|| self.schemas.as_mut().map(f))?
    }

    /// Execute a function with mutable access to tables if currently viewing tables
    pub fn with_tables<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut FilteredData<Table>) -> R,
    {
        matches!(self.state, DatabaseExplorerState::Tables(_))
            .then(|| self.tables.as_mut().map(f))?
    }

    /// Execute a function with mutable access to columns if currently viewing columns
    pub fn with_columns<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut FilteredData<Column>) -> R,
    {
        matches!(self.state, DatabaseExplorerState::Columns(_, _))
            .then(|| self.columns.as_mut().map(f))?
    }

    /// Execute a function with mutable access to table data if currently viewing table data
    pub fn with_table_data<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut FilteredData<RawTableRow>) -> R,
    {
        matches!(self.state, DatabaseExplorerState::TableData(_, _))
            .then(|| self.table_data.as_mut().map(f))?
    }
}
