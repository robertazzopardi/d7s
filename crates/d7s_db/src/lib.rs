pub mod connection;

pub trait TableData {
    fn title() -> &'static str;
    fn ref_array(&self) -> Vec<&String>;
    fn num_columns(&self) -> usize;
    fn cols() -> Vec<&'static str>;

    fn col(&self, column: usize) -> &str {
        self.ref_array()[column]
    }
}

pub fn check_connection() -> bool {
    false
}
