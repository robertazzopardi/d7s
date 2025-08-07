pub mod connection;
pub mod postgres;
pub mod sqlite;

use std::path::PathBuf;

use color_eyre::Result;

pub trait TableData {
    fn title() -> &'static str;
    fn ref_array(&self) -> Vec<String>;
    fn num_columns(&self) -> usize;
    fn cols() -> Vec<&'static str>;

    fn col(&self, column: usize) -> String {
        self.ref_array()[column].clone()
    }
}

#[allow(async_fn_in_trait)]
pub trait Database {
    async fn test(&self) -> bool;
}

pub(crate) fn get_app_data_dir() -> Result<PathBuf> {
    let mut path =
        dirs::data_dir().expect("Could not determine data directory");

    path.push("d7s");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&path)?;

    Ok(path)
}

pub(crate) fn get_db_path() -> Result<PathBuf> {
    let mut path = get_app_data_dir()?;
    path.push("d7s.db");
    Ok(path)
}
