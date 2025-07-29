pub mod connection;
pub mod postgres;

pub trait TableData {
    fn title() -> &'static str;
    fn ref_array(&self) -> Vec<String>;
    fn num_columns(&self) -> usize;
    fn cols() -> Vec<&'static str>;

    fn col(&self, column: usize) -> String {
        self.ref_array()[column].clone()
    }
}

pub trait Database {
    async fn test(&self) -> bool;
}
