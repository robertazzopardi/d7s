use std::fmt::Display;

use crate::TableData;

#[derive(Debug, Default, Clone)]
pub struct Connection {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub database: String,
    pub schema: String,
    pub table: String,
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            " Name: {}\n Host: {}\n Port: {}\n User: {}\n Database: {}\n Schema: {}\n Table: {}",
            self.name,
            self.host,
            self.port,
            self.user,
            self.database,
            self.schema,
            self.table
        )
    }
}

impl TableData for Connection {
    fn title() -> &'static str {
        "Connection"
    }

    fn ref_array(&self) -> Vec<&String> {
        vec![
            &self.name,
            &self.host,
            &self.port,
            &self.user,
            &self.database,
            &self.schema,
            &self.table,
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec![
            "Name", "Host", "Port", "User", "Database", "Schema", "Table",
        ]
    }
}
