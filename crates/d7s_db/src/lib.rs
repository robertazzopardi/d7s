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

/// Database object types for the explorer
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseObjectType {
    Schema,
    Table,
    View,
    Column,
}

/// Schema information
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub owner: String,
}

/// Table information
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub schema: String,
    pub size: Option<String>,
}

/// Column information
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Sample data row
#[derive(Debug, Clone)]
pub struct DataRow {
    pub values: Vec<String>,
}

impl TableData for Schema {
    fn title() -> &'static str {
        "Schemas"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![self.name.clone(), self.owner.clone()]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Owner", "Description"]
    }
}

impl TableData for Table {
    fn title() -> &'static str {
        "Tables"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.schema.clone(),
            self.size.clone().unwrap_or_default(),
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Schema", "Size"]
    }
}

impl TableData for Column {
    fn title() -> &'static str {
        "Columns"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.data_type.clone(),
            if self.is_nullable {
                "YES".to_string()
            } else {
                "NO".to_string()
            },
            self.default_value.clone().unwrap_or_default(),
            self.description.clone().unwrap_or_default(),
        ]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }

    fn cols() -> Vec<&'static str> {
        vec!["Name", "Type", "Nullable", "Default", "Description"]
    }
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
