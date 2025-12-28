use d7s_db::TableData;
use d7s_ui::{handlers::TableNavigationHandler, widgets::table::DataTable};

/// A wrapper for managing filtered data with original data preservation
#[derive(Clone)]
pub struct FilteredData<T: TableData + Clone> {
    /// Original unfiltered data
    pub original: Vec<T>,
    /// DataTable widget with potentially filtered items
    pub table: DataTable<T>,
}

impl<T: TableData + Clone> FilteredData<T> {
    /// Create a new FilteredData from a vector of items
    pub fn new(data: Vec<T>) -> Self {
        Self {
            original: data.clone(),
            table: DataTable::new(data),
        }
    }

    /// Apply a filter to the data
    pub fn apply_filter(&mut self, query: &str) {
        self.table.items = self.table.filter(query);
        TableNavigationHandler::wrap_rows(&mut self.table);
    }

    /// Clear the filter and restore original data
    pub fn clear_filter(&mut self) {
        self.table.items.clone_from(&self.original);
        TableNavigationHandler::wrap_rows(&mut self.table);
    }

    /// Get a reference to the table widget
    pub fn table(&self) -> &DataTable<T> {
        &self.table
    }

    /// Get a mutable reference to the table widget
    pub fn table_mut(&mut self) -> &mut DataTable<T> {
        &mut self.table
    }
}
