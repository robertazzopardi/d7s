#[cfg(debug_assertions)]
use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::sync::{Mutex, OnceLock};

#[cfg(not(debug_assertions))]
use keyring::Entry;

#[cfg(not(debug_assertions))]
const SERVICE_NAME: &str = "d7s";

#[cfg(not(debug_assertions))]
pub struct Keyring {
    entry: Entry,
}

#[cfg(debug_assertions)]
pub struct Keyring {
    user: String,
}

#[cfg(debug_assertions)]
fn dev_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(not(debug_assertions))]
#[derive(Debug)]
pub enum Error {
    KeyringError(keyring::Error),
}

#[cfg(not(debug_assertions))]
impl From<keyring::Error> for Error {
    fn from(err: keyring::Error) -> Self {
        Error::KeyringError(err)
    }
}

#[cfg(debug_assertions)]
#[derive(Debug)]
pub enum Error {
    NotFound,
    Other(String),
}

#[cfg(debug_assertions)]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "Password not found"),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

#[cfg(debug_assertions)]
impl std::error::Error for Error {}

#[cfg(not(debug_assertions))]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::KeyringError(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(not(debug_assertions))]
impl std::error::Error for Error {}

impl Keyring {
    /// Creates a new keyring entry
    ///
    /// # Errors
    ///
    /// Returns an error if the keyring entry cannot be created
    #[cfg(not(debug_assertions))]
    pub fn new(user: &str) -> Result<Self, Error> {
        Ok(Self {
            entry: Entry::new(SERVICE_NAME, user)?,
        })
    }

    /// Creates a new in-memory credential store (dev mode)
    ///
    /// # Errors
    ///
    /// Never returns an error in dev mode
    #[cfg(debug_assertions)]
    pub fn new(user: &str) -> Result<Self, Error> {
        Ok(Self {
            user: user.to_string(),
        })
    }

    /// Sets the password in the keyring
    ///
    /// # Errors
    ///
    /// Returns an error if the password cannot be set
    #[cfg(not(debug_assertions))]
    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        self.entry.set_password(password).map_err(Error::from)
    }

    /// Sets the password in the in-memory store (dev mode)
    ///
    /// # Errors
    ///
    /// Never returns an error in dev mode
    #[cfg(debug_assertions)]
    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        let mut store = dev_store().lock().unwrap();
        store.insert(self.user.clone(), password.to_string());
        Ok(())
    }

    /// Gets the password from the keyring
    ///
    /// # Errors
    ///
    /// Returns an error if the password cannot be retrieved
    #[cfg(not(debug_assertions))]
    pub fn get_password(&self) -> Result<String, Error> {
        self.entry.get_password().map_err(Error::from)
    }

    /// Gets the password from the in-memory store (dev mode)
    ///
    /// # Errors
    ///
    /// Returns an error if the password is not found
    #[cfg(debug_assertions)]
    pub fn get_password(&self) -> Result<String, Error> {
        let store = dev_store().lock().unwrap();
        store.get(&self.user).cloned().ok_or(Error::NotFound)
    }

    /// Deletes the password from the keyring
    ///
    /// # Errors
    ///
    /// Returns an error if the password cannot be deleted
    #[cfg(not(debug_assertions))]
    pub fn delete_password(&self) -> Result<(), Error> {
        self.entry.delete_credential().map_err(Error::from)
    }

    /// Deletes the password from the in-memory store (dev mode)
    ///
    /// # Errors
    ///
    /// Never returns an error in dev mode
    #[cfg(debug_assertions)]
    pub fn delete_password(&self) -> Result<(), Error> {
        let mut store = dev_store().lock().unwrap();
        store.remove(&self.user);
        Ok(())
    }
}
