use std::fmt::Display;

#[derive(Debug, Default)]
pub struct Connection {
    pub name: String,
    pub host: String,
    pub port: u16,
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
            if self.port == 0 {
                "".to_string()
            } else {
                self.port.to_string()
            },
            self.user,
            self.database,
            self.schema,
            self.table
        )
    }
}
