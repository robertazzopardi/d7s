use d7s_db::{
    Column, Database, DatabaseInfo, Schema, Table,
    connection::Connection,
};
use d7s_ui::widgets::table::RawTableRow;

use crate::{app_state::DatabaseExplorerState, filtered_data::FilteredData};

/// Groups all database exploration state together
#[derive(Default)]
pub struct DatabaseExplorer {
    /// The active database connection
    pub connection: Connection,
    /// The active database client
    pub database: Option<Box<dyn Database>>,
    /// Current navigation state in the database
    pub state: DatabaseExplorerState,
    /// Previous state before entering SQL executor (to restore on exit)
    pub previous_state: Option<DatabaseExplorerState>,
    /// Cached database list
    pub databases: Option<FilteredData<DatabaseInfo>>,
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
    pub fn new(
        connection: Connection,
        database: Option<Box<dyn Database>>,
    ) -> Self {
        Self {
            connection,
            database,
            state: DatabaseExplorerState::Databases,
            previous_state: None,
            databases: None,
            schemas: None,
            tables: None,
            columns: None,
            table_data: None,
        }
    }
}
