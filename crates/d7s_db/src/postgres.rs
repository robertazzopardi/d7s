use tokio_postgres::NoTls;

use crate::{Database, TableData};

#[derive(Debug, Clone, Default)]

pub struct Postgres {
    pub name: String,
    pub host: Option<String>,
    pub port: Option<String>,
    pub user: String,
    pub database: String,
    pub password: String,
}

impl Database for Postgres {
    async fn test(&self) -> bool {
        let config = format!(
            "host={} port={} user={} password={} dbname={}",
            self.host.clone().unwrap_or_else(|| "localhost".to_string()),
            self.port.clone().unwrap_or_else(|| "5432".to_string()),
            self.user,
            self.password,
            self.database
        );

        let Ok(_) = tokio_postgres::connect(&config, NoTls).await else {
            return false;
        };

        true
    }
}

impl TableData for Postgres {
    fn title() -> &'static str {
        "Postgres"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.host.clone().unwrap_or_default(),
            self.port.clone().unwrap_or_default(),
            self.user.clone(),
            self.password.clone(),
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Host", "Port", "User", "Password"]
    }
}
