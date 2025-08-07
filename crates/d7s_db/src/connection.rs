use std::fmt::Display;

use crate::{TableData, postgres::Postgres};

#[derive(Debug, Default, Clone)]
pub struct Connection {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub database: String,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub password: Option<String>,
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
            self.schema.clone().unwrap_or_default(),
            self.table.clone().unwrap_or_default(),
        )
    }
}

impl TableData for Connection {
    fn title() -> &'static str {
        "Connection"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.host.clone(),
            self.port.clone(),
            self.user.clone(),
            self.database.clone(),
            // Mask password field with dots
            if let Some(password) = &self.password {
                "•".repeat(password.len())
            } else {
                String::new()
            },
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Host", "Port", "User", "Database", "Password"]
    }
}

impl Connection {
    /// Convert this connection to a Postgres instance for testing
    #[must_use]
    pub fn to_postgres(&self) -> Postgres {
        Postgres {
            name: self.name.clone(),
            host: Some(self.host.clone()),
            port: Some(self.port.clone()),
            user: self.user.clone(),
            database: self.database.clone(),
            password: self.password.clone().unwrap_or_default(),
        }
    }
}
