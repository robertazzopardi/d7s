use keyring::{Entry, Error};

const SERVICE_NAME: &str = "d7s";

pub struct Keyring {
    entry: Entry,
}

impl Keyring {
    pub fn new(user: &str) -> Self {
        Self {
            entry: Entry::new(SERVICE_NAME, user).unwrap(),
        }
    }

    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        self.entry.set_password(password)
    }

    pub fn get_password(&self) -> Result<String, Error> {
        self.entry.get_password()
    }

    pub fn delete_password(&self) -> Result<(), Error> {
        self.entry.delete_credential()
    }
}
