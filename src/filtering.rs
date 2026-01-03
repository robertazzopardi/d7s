use d7s_db::TableData;

use crate::{
    app::App,
    app_state::{AppState, DatabaseExplorerState},
    filtered_data::FilteredData,
};

impl App<'_> {
    /// Apply the current search filter to the active table
    pub fn apply_filter(&mut self) {
        let query: String = self.search_filter.get_filter_query().to_string();
        self.apply_filter_with_query(&query);
    }

    /// Clear the current filter and restore original data
    pub fn clear_filter(&mut self) {
        self.apply_to_active_filtered_data(|data| data.clear_filter());
    }

    /// Apply filter with a specific query string
    fn apply_filter_with_query(&mut self, query: &str) {
        self.apply_to_active_filtered_data(|data| data.apply_filter(query));
    }

    /// Helper to apply an operation to the currently active filtered data
    fn apply_to_active_filtered_data<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut dyn FilteredDataMut),
    {
        match self.state {
            AppState::ConnectionList => {
                f(&mut self.connections);
            }
            AppState::DatabaseConnected => {
                if let Some(explorer) = &mut self.database_explorer {
                    match explorer.state {
                        DatabaseExplorerState::Schemas => {
                            if let Some(ref mut schemas) = explorer.schemas {
                                f(schemas);
                            }
                        }
                        DatabaseExplorerState::Tables(_) => {
                            if let Some(ref mut tables) = explorer.tables {
                                f(tables);
                            }
                        }
                        DatabaseExplorerState::Columns(_, _) => {
                            if let Some(ref mut columns) = explorer.columns {
                                f(columns);
                            }
                        }
                        DatabaseExplorerState::TableData(_, _) => {
                            if let Some(ref mut table_data) =
                                explorer.table_data
                            {
                                f(table_data);
                            }
                        }
                        DatabaseExplorerState::SqlExecutor => {
                            // No filtering for SQL executor
                        }
                    }
                }
            }
        }
    }
}

/// Trait to allow polymorphic access to `FilteredData` operations
trait FilteredDataMut {
    fn apply_filter(&mut self, query: &str);
    fn clear_filter(&mut self);
}

impl<T: TableData + Clone> FilteredDataMut for FilteredData<T> {
    fn apply_filter(&mut self, query: &str) {
        Self::apply_filter(self, query);
    }

    fn clear_filter(&mut self) {
        Self::clear_filter(self);
    }
}
