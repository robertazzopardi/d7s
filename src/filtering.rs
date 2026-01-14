use crate::{
    app::App,
    app_state::{AppState, DatabaseExplorerState},
};

impl App<'_> {
    /// Apply the current search filter to the active table
    pub fn apply_filter(&mut self) {
        let query: String = self.search_filter.get_filter_query().to_string();
        self.apply_filter_with_query(&query);
    }

    /// Clear the current filter and restore original data
    pub fn clear_filter(&mut self) {
        match self.state {
            AppState::ConnectionList => {
                self.connections.clear_filter();
            }
            AppState::DatabaseConnected => {
                if let Some(explorer) = &mut self.database_explorer {
                    match explorer.state {
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
                            if let Some(ref mut table_data) =
                                explorer.table_data
                            {
                                table_data.clear_filter();
                            }
                        }
                        DatabaseExplorerState::SqlExecutor => {}
                    }
                }
            }
        }
    }

    /// Apply filter with a specific query string
    fn apply_filter_with_query(&mut self, query: &str) {
        match self.state {
            AppState::ConnectionList => {
                self.connections.apply_filter(query);
            }
            AppState::DatabaseConnected => {
                if let Some(explorer) = &mut self.database_explorer {
                    match explorer.state {
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
                            if let Some(ref mut table_data) =
                                explorer.table_data
                            {
                                table_data.apply_filter(query);
                            }
                        }
                        DatabaseExplorerState::SqlExecutor => {}
                    }
                }
            }
        }
    }
}
