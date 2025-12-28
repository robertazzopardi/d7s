use std::collections::HashMap;
use color_eyre::Result;
use d7s_auth::Keyring;
use d7s_db::connection::Connection;

/// Service for managing passwords across keyring and session storage
pub struct PasswordService {
    /// Session password storage (in-memory only, cleared when app exits)
    /// Key format: "{user}@{host}:{port}/{database}"
    session_passwords: HashMap<String, String>,
    /// Whether to automatically store passwords in session memory when "ask every time" is enabled
    /// Default: true (auto-store for better UX)
    auto_store_session: bool,
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordService {
    /// Create a new password service with auto-store enabled
    pub fn new() -> Self {
        Self {
            session_passwords: HashMap::new(),
            auto_store_session: true,
        }
    }

    /// Generate a unique key for a connection to use in session password storage
    fn connection_key(connection: &Connection) -> String {
        format!(
            "{}@{}:{}/{}",
            connection.user, connection.host, connection.port, connection.database
        )
    }

    // Keyring operations

    /// Get password from keyring for a connection
    pub fn get_from_keyring(connection_name: &str) -> Result<String> {
        let keyring = Keyring::new(connection_name)?;
        Ok(keyring.get_password()?)
    }

    /// Save password to keyring for a connection
    pub fn save_to_keyring(connection_name: &str, password: &str) -> Result<()> {
        let keyring = Keyring::new(connection_name)?;
        keyring.set_password(password)?;
        Ok(())
    }

    /// Delete password from keyring for a connection
    pub fn delete_from_keyring(connection_name: &str) -> Result<()> {
        let keyring = Keyring::new(connection_name)?;
        keyring.delete_password()?;
        Ok(())
    }

    // Session operations

    /// Get password from session storage for a connection
    pub fn get_session_password(&self, connection: &Connection) -> Option<&String> {
        let key = Self::connection_key(connection);
        self.session_passwords.get(&key)
    }

    /// Store password in session memory for a connection
    pub fn store_session_password(&mut self, connection: &Connection, password: String) {
        if self.auto_store_session && connection.should_ask_every_time() {
            let key = Self::connection_key(connection);
            self.session_passwords.insert(key, password);
        }
    }

    /// Clear all session passwords
    pub fn clear_session(&mut self) {
        self.session_passwords.clear();
    }

    // High-level API

    /// Get password for a connection from the appropriate source
    /// Returns Some(password) if found in session or keyring, None if needs prompting
    pub fn get_password(&self, connection: &Connection) -> Option<String> {
        if connection.should_ask_every_time() {
            // Check session storage first
            self.get_session_password(connection).cloned()
        } else {
            // Try keyring
            Self::get_from_keyring(&connection.name).ok()
        }
    }

    /// Check if we should prompt the user for a password
    pub fn should_prompt_for_password(&self, connection: &Connection) -> bool {
        self.get_password(connection).is_none()
    }

    /// Get password for connection, returning empty string if "ask every time" and not in session
    pub fn get_connection_password(&self, connection: &Connection) -> String {
        if connection.should_ask_every_time() {
            String::new()
        } else {
            Self::get_from_keyring(&connection.name).unwrap_or_default()
        }
    }
}
