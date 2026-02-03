use super::filtered_data::FilteredData;
use crate::{app::App, app_state::DatabaseExplorerState};

impl App<'_> {
    /// Apply the current search filter to the active table
    pub fn apply_filter(&mut self) {
        let query: String = self.search_filter.get_filter_query().to_string();
        self.apply_filter_with_query(&query);
    }

    /// Check if any filter is currently active
    pub fn has_active_filter(&self) -> bool {
        let explorer = &self.database_explorer;
        match &explorer.state {
            DatabaseExplorerState::Connections => {
                self.connections.is_filtered()
            }
            DatabaseExplorerState::Databases => explorer
                .databases
                .as_ref()
                .is_some_and(FilteredData::is_filtered),
            DatabaseExplorerState::Schemas => explorer
                .schemas
                .as_ref()
                .is_some_and(FilteredData::is_filtered),
            DatabaseExplorerState::Tables(_) => explorer
                .tables
                .as_ref()
                .is_some_and(FilteredData::is_filtered),
            DatabaseExplorerState::Columns(_, _) => explorer
                .columns
                .as_ref()
                .is_some_and(FilteredData::is_filtered),
            DatabaseExplorerState::TableData(_, _) => explorer
                .table_data
                .as_ref()
                .is_some_and(FilteredData::is_filtered),
            DatabaseExplorerState::SqlExecutor => false,
        }
    }

    /// Clear the current filter and restore original data
    pub fn clear_filter(&mut self) {
        let explorer = &mut self.database_explorer;
        match explorer.state {
            DatabaseExplorerState::Connections => {
                self.connections.clear_filter();
            }
            DatabaseExplorerState::Databases => {
                if let Some(ref mut databases) = explorer.databases {
                    databases.clear_filter();
                }
            }
            DatabaseExplorerState::Schemas => {
                if let Some(ref mut schemas) = explorer.schemas {
                    schemas.clear_filter();
                }
            }
            DatabaseExplorerState::Tables(_) => {
                if let Some(ref mut tables) = explorer.tables {
                    tables.clear_filter();
                }
            }
            DatabaseExplorerState::Columns(_, _) => {
                if let Some(ref mut columns) = explorer.columns {
                    columns.clear_filter();
                }
            }
            DatabaseExplorerState::TableData(_, _) => {
                if let Some(ref mut table_data) = explorer.table_data {
                    table_data.clear_filter();
                }
            }
            DatabaseExplorerState::SqlExecutor => {}
        }
    }

    /// Apply filter with a specific query string
    fn apply_filter_with_query(&mut self, query: &str) {
        let explorer = &mut self.database_explorer;
        match explorer.state {
            DatabaseExplorerState::Connections => {
                self.connections.apply_filter(query);
            }
            DatabaseExplorerState::Databases => {
                if let Some(ref mut databases) = explorer.databases {
                    databases.apply_filter(query);
                }
            }
            DatabaseExplorerState::Schemas => {
                if let Some(ref mut schemas) = explorer.schemas {
                    schemas.apply_filter(query);
                }
            }
            DatabaseExplorerState::Tables(_) => {
                if let Some(ref mut tables) = explorer.tables {
                    tables.apply_filter(query);
                }
            }
            DatabaseExplorerState::Columns(_, _) => {
                if let Some(ref mut columns) = explorer.columns {
                    columns.apply_filter(query);
                }
            }
            DatabaseExplorerState::TableData(_, _) => {
                if let Some(ref mut table_data) = explorer.table_data {
                    table_data.apply_filter(query);
                }
            }
            DatabaseExplorerState::SqlExecutor => {}
        }
    }
}
