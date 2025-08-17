use keyring::{Entry, Error};

const SERVICE_NAME: &str = "d7s";

pub struct Keyring {
    entry: Entry,
}

impl Keyring {
    /// Creates a new keyring entry
    ///
    /// # Errors
    ///
    /// Returns an error if the keyring entry cannot be created
    pub fn new(user: &str) -> Result<Self, Error> {
        Ok(Self {
            entry: Entry::new(SERVICE_NAME, user)?,
        })
    }

    /// Sets the password in the keyring
    ///
    /// # Errors
    ///
    /// Returns an error if the password cannot be set
    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        self.entry.set_password(password)
    }

    /// Gets the password from the keyring
    ///
    /// # Errors
    ///
    /// Returns an error if the password cannot be retrieved
    pub fn get_password(&self) -> Result<String, Error> {
        self.entry.get_password()
    }

    /// Deletes the password from the keyring
    ///
    /// # Errors
    ///
    /// Returns an error if the password cannot be deleted
    pub fn delete_password(&self) -> Result<(), Error> {
        self.entry.delete_credential()
    }
}
