use std::fmt;

/// Application-specific errors
#[derive(Debug)]
pub enum D7sError {
    /// Database operation error
    Database(String),
    /// Connection operation error
    Connection(String),
    /// Password/authentication error
    Password(String),
    /// I/O error
    Io(std::io::Error),
    /// Generic error
    Other(String),
}

impl fmt::Display for D7sError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(msg) => write!(f, "Database error: {msg}"),
            Self::Connection(msg) => write!(f, "Connection error: {msg}"),
            Self::Password(msg) => write!(f, "Password error: {msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for D7sError {}

impl From<std::io::Error> for D7sError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<Box<dyn std::error::Error>> for D7sError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<color_eyre::Report> for D7sError {
    fn from(err: color_eyre::Report) -> Self {
        Self::Other(err.to_string())
    }
}
