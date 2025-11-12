use d7s_auth::Keyring;
use d7s_db::{Database, connection::Connection};

use crate::widgets::modal::{PasswordStorageType, TestResult};

/// Handles saving a connection from the modal
pub fn handle_save_connection(
    keyring: &mut Option<Keyring>,
    connection: Connection,
    mode: crate::widgets::modal::Mode,
    original_name: Option<String>,
) -> Result<(), String> {
    // Handle password storage based on connection's storage preference
    if let Some(password) = &connection.password {
        let storage_type = connection
            .password_storage
            .as_ref()
            .map(|s| PasswordStorageType::from_str(s))
            .unwrap_or(PasswordStorageType::Keyring);

        match storage_type {
            PasswordStorageType::Keyring => {
                // In dev mode, don't actually save to keyring (but UI still shows the option)
                #[cfg(not(debug_assertions))]
                {
                    let keyring_result = if let Some(keyring) = keyring {
                        keyring.set_password(password)
                    } else {
                        match Keyring::new(&connection.user) {
                            Ok(new_keyring) => {
                                let result = new_keyring.set_password(password);
                                *keyring = Some(new_keyring);
                                result
                            }
                            Err(e) => {
                                return Err(format!(
                                    "Failed to create keyring: {e}"
                                ));
                            }
                        }
                    };

                    if let Err(e) = keyring_result {
                        let error_msg = e.to_string();
                        if error_msg.contains("locked collection") {
                            return Err(
                                "Keyring is locked. Please unlock your keyring first.\n\n\
                                On Linux, you can unlock it using:\n\
                                - seahorse (GUI: search for 'Passwords and Keys')\n\
                                - Or unlock it when prompted by your desktop environment\n\n\
                                Alternatively, you can save the connection without storing the password in the keyring."
                                    .to_string(),
                            );
                        }
                        return Err(format!(
                            "Failed to store password in keyring: {error_msg}\n\n\
                            Hint: If your keyring is locked, unlock it first using your system's keyring manager."
                        ));
                    }
                }
                #[cfg(debug_assertions)]
                {
                    // In dev mode, passwords are not saved to keyring
                    // The preference is still stored in the database for consistency
                }
            }
            PasswordStorageType::DontSave => {
                // Don't save password - connection will work but password won't be stored
            }
        }
    }

    if matches!(mode, crate::widgets::modal::Mode::New) {
        d7s_db::sqlite::save_connection(&connection)
            .map_err(|e| format!("Failed to save connection: {e}"))?;
    } else if matches!(mode, crate::widgets::modal::Mode::Edit) {
        if let Some(original_name) = original_name {
            d7s_db::sqlite::update_connection(&original_name, &connection)
                .map_err(|e| format!("Failed to update connection: {e}"))?;
        }
    }

    Ok(())
}

/// Tests a database connection
pub async fn test_connection(connection: &Connection) -> TestResult {
    let postgres = connection.to_postgres();
    let result = postgres.test().await;

    if result {
        TestResult::Success
    } else {
        TestResult::Failed("Connection failed".to_string())
    }
}
