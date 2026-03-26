use crossterm::event::KeyCode;
use ratatui::widgets::TableState;

use crate::{
    app_state::DatabaseExplorerState,
    db::{
        Column, Database, DatabaseInfo, Schema, Table, connection::Connection,
    },
    filtered_data::FilteredData,
    ui::{sql_executor::SqlExecutorState, widgets::table::RawTableRow},
};

/// Groups all database exploration state together
#[derive(Default)]
pub struct DatabaseExplorer {
    /// The active database connection
    pub connection: Connection,
    /// The active database client
    pub database: Option<Box<dyn Database>>,
    /// Current navigation state in the database
    pub state: DatabaseExplorerState,
    /// Connection list with filtering
    pub connections: FilteredData<Connection>,
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
    /// SQL executor state
    pub sql_executor: SqlExecutorState,
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
            connections: FilteredData::default(),
            state: DatabaseExplorerState::Databases,
            previous_state: None,
            databases: None,
            schemas: None,
            tables: None,
            columns: None,
            table_data: None,
            sql_executor: SqlExecutorState::new(),
        }
    }

    /// Navigate the currently active explorer table (excludes Connections and `SqlExecutor`)
    pub fn navigate_current(&mut self, key: KeyCode) {
        match &self.state {
            DatabaseExplorerState::Databases => {
                if let Some(ref mut t) = self.databases {
                    t.navigate(key);
                }
            }
            DatabaseExplorerState::Schemas => {
                if let Some(ref mut t) = self.schemas {
                    t.navigate(key);
                }
            }
            DatabaseExplorerState::Tables(_) => {
                if let Some(ref mut t) = self.tables {
                    t.navigate(key);
                }
            }
            DatabaseExplorerState::Columns(_, _) => {
                if let Some(ref mut t) = self.columns {
                    t.navigate(key);
                }
            }
            DatabaseExplorerState::TableData(_, _) => {
                if let Some(ref mut t) = self.table_data {
                    t.navigate(key);
                }
            }
            DatabaseExplorerState::Connections
            | DatabaseExplorerState::SqlExecutor => {}
        }
    }

    pub fn current_table_state_mut(&mut self) -> Option<&mut TableState> {
        let state = &mut self.state;
        match state {
            DatabaseExplorerState::Connections => {
                Some(&mut self.connections.table.view.state)
            }
            DatabaseExplorerState::Databases => {
                self.databases.as_mut().map(|dbs| &mut dbs.table.view.state)
            }
            DatabaseExplorerState::Schemas => self
                .schemas
                .as_mut()
                .map(|schemas| &mut schemas.table.view.state),
            DatabaseExplorerState::Tables(_) => self
                .tables
                .as_mut()
                .map(|tables| &mut tables.table.view.state),
            DatabaseExplorerState::Columns(_, _) => self
                .columns
                .as_mut()
                .map(|columns| &mut columns.table.view.state),
            DatabaseExplorerState::TableData(_, _) => self
                .table_data
                .as_mut()
                .map(|table_data| &mut table_data.table.view.state),
            DatabaseExplorerState::SqlExecutor => {
                Some(&mut self.sql_executor.table_state.view.state)
            }
        }
    }
}
