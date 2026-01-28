use d7s_db::TableData;
use d7s_ui::{
    handlers::TableNavigationHandler, widgets::table::TableDataState,
};

/// A wrapper for managing filtered data with original data preservation
#[derive(Clone)]
pub struct FilteredData<T: TableData + Clone> {
    /// Original unfiltered data
    pub original: Vec<T>,
    /// Table state with potentially filtered items
    pub table: TableDataState<T>,
}

impl<T: TableData + Clone> FilteredData<T> {
    /// Create a new `FilteredData` from a vector of items
    pub fn new(data: Vec<T>) -> Self {
        Self {
            original: data.clone(),
            table: TableDataState::new(data),
        }
    }

    /// Apply a filter to the data
    pub fn apply_filter(&mut self, query: &str) {
        self.table.model.items = self.table.filter(query);
        TableNavigationHandler::wrap_rows(&mut self.table);
    }

    /// Clear the filter and restore original data
    pub fn clear_filter(&mut self) {
        self.table.model.items.clone_from(&self.original);
        TableNavigationHandler::wrap_rows(&mut self.table);
    }

    /// Check if the data is currently filtered
    pub const fn is_filtered(&self) -> bool {
        self.table.model.items.len() != self.original.len()
    }
}
