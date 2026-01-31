use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{TableData, postgres::Postgres, sqlite::Sqlite};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    #[default]
    Postgres,
    Sqlite,
}

impl Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Sqlite => write!(f, "sqlite"),
        }
    }
}

impl std::str::FromStr for ConnectionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        Ok(match lower.as_str() {
            "postgres" | "postgresql" => Self::Postgres,
            "sqlite" => Self::Sqlite,
            _ => return Err(()),
        })
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Dev,
    Staging,
    Prod,
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Staging => write!(f, "staging"),
            Self::Prod => write!(f, "prod"),
        }
    }
}

impl std::str::FromStr for Environment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        Ok(match lower.as_str() {
            "dev" => Self::Dev,
            "staging" => Self::Staging,
            "prod" => Self::Prod,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Connection {
    /// Friendly alias (e.g. "Production Cache")
    pub name: String,
    /// postgres or sqlite
    pub r#type: ConnectionType,
    /// Full DSN (e.g. postgres://..., or path for sqlite)
    pub url: String,
    /// dev, staging, prod
    pub environment: Environment,
    /// Extra fields stored as JSON.
    pub metadata: serde_json::Value,
    /// Runtime UI state (current database when connected; used by to_postgres)
    pub selected_database: Option<String>,
    /// Runtime UI state
    pub schema: Option<String>,
    pub table: Option<String>,
    /// Password (not persisted; from keyring or prompt)
    pub password: Option<String>,
    /// Where to store password: "keyring" or "dont_save". Persisted in metadata.
    pub password_storage: Option<String>,
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (host, port, user, database_from_url) =
            parse_postgres_url(&self.url);
        let database = self
            .selected_database
            .as_deref()
            .unwrap_or(&database_from_url);

        let (host, port, user, database) = match self.r#type {
            ConnectionType::Postgres => {
                (host, port, user, database.to_string())
            }
            ConnectionType::Sqlite => (
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                self.url.clone(),
            ),
        };

        write!(
            f,
            " Name: {}\n Host: {}\n Port: {}\n User: {}\n Database: {}\n Schema: {}\n Table: {}",
            self.name,
            host,
            port,
            user,
            database,
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
            self.r#type.to_string(),
            redact_password_in_url(self.url.as_str()),
            self.environment.to_string(),
            self.metadata.to_string(),
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Type", "Url", "Environment", "Metadata", "Password"]
    }
}

/// Redact password in a URL for display (e.g. postgres://user:xxx@host/db)
fn redact_password_in_url(url: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(url) {
        if parsed.password().is_some()
            && !parsed.password().unwrap_or_default().is_empty()
        {
            parsed
                .set_password(Some("***"))
                .map_or_else(|_| url.to_string(), |_| parsed.to_string())
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    }
}

impl Connection {
    /// Convert this connection to a Postgres instance for testing/connecting.
    /// Parses `url` and uses `password` for authentication.
    /// Uses `selected_database` if set (when connected to a specific database), otherwise parses from URL.
    #[must_use]
    pub fn to_postgres(&self) -> Postgres {
        let (host, port, user, database_from_url) =
            parse_postgres_url(&self.url);
        let database =
            self.selected_database.clone().unwrap_or(database_from_url);
        Postgres {
            name: self.name.clone(),
            host: Some(host),
            port: Some(port),
            user,
            database,
            password: self.password.clone().unwrap_or_default(),
        }
    }

    #[must_use]
    pub fn to_sqlite(&self) -> Sqlite {
        Sqlite {
            name: self.name.clone(),
            path: self.url.clone(),
        }
    }

    /// Check if this connection is configured to ask for password every time
    #[must_use]
    pub fn should_ask_every_time(&self) -> bool {
        self.password_storage
            .as_ref()
            .is_some_and(|s| s.eq_ignore_ascii_case("dont_save"))
    }

    /// Check if this connection is configured to use keyring storage
    #[must_use]
    pub fn uses_keyring(&self) -> bool {
        self.password_storage
            .as_ref()
            .is_some_and(|s| s.eq_ignore_ascii_case("keyring"))
    }

    /// User part of the connection (for prompts). Parsed from URL for postgres.
    #[must_use]
    pub fn user_display(&self) -> String {
        if self.r#type == ConnectionType::Postgres {
            if let Ok(u) = url::Url::parse(&self.url) {
                return u.username().to_string();
            }
        }
        self.name.clone()
    }

    /// Values for the connection form (Name, Type, Url, Environment, Metadata, Password).
    #[must_use]
    pub fn form_values(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.r#type.to_string(),
            self.url.clone(),
            self.environment.to_string(),
            self.metadata.to_string(),
            self.password.clone().unwrap_or_default(),
        ]
    }
}

/// Parse a postgres/postgresql URL into (host, port, user, database).
fn parse_postgres_url(url_str: &str) -> (String, String, String, String) {
    let default_host = "localhost".to_string();
    let default_port = "5432".to_string();
    let default_user = String::new();
    let default_db = "postgres".to_string();

    let Ok(url) = url::Url::parse(url_str) else {
        return (default_host, default_port, default_user, default_db);
    };
    let host = url
        .host_str()
        .map(std::string::ToString::to_string)
        .unwrap_or(default_host);
    let port = url.port().map(|p| p.to_string()).unwrap_or(default_port);
    let user = url.username().to_string();
    let database = url
        .path()
        .strip_prefix('/')
        .unwrap_or("postgres")
        .to_string();
    (host, port, user, database)
}
