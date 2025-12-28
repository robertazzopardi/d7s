use d7s_db::{Column, Schema, Table, connection::Connection, postgres::Postgres};
use d7s_ui::widgets::table::RawTableRow;

use crate::{app_state::DatabaseExplorerState, filtered_data::FilteredData};

/// Groups all database exploration state together
pub struct DatabaseExplorer {
    /// The active database connection
    pub connection: Connection,
    /// The active database client
    pub database: Postgres,
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
    /// Create a new DatabaseExplorer with a connection and database client
    pub fn new(connection: Connection, database: Postgres) -> Self {
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
}
