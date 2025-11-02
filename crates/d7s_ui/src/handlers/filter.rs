// Filter state enum - actual filter logic is kept in app.rs for now

/// Represents the current filter state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterState {
    Connections,
    Schemas,
    Tables,
    Columns,
    TableData,
}
