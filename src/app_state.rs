use std::fmt::{Display, Formatter, Result};

/// Application state to track whether we're viewing connections or connected to a database
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    ConnectionList,
    DatabaseConnected,
}

/// Database explorer state to track what object type is being viewed
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum DatabaseExplorerState {
    #[default]
    Connections,
    Databases,
    Schemas,
    Tables(String),            // schema name
    Columns(String, String),   // schema name, table name
    TableData(String, String), // schema name, table name
    SqlResults(String),        // SQL execution mode
}

impl Display for DatabaseExplorerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Connections => write!(f, " Connections "),
            Self::Databases => write!(f, " Databases "),
            Self::Schemas => write!(f, " Schemas "),
            Self::Tables(schema) => write!(f, " {schema} "),
            Self::Columns(schema, table) | Self::TableData(schema, table) => {
                write!(f, " {schema}.{table} ")
            }
            Self::SqlResults(query) => {
                let query_display = if query.len() > 20 {
                    query.split_at(21).0
                } else {
                    query
                };
                write!(f, " {query_display} ")
            }
        }
    }
}
