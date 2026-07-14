pub mod file_chunker;
pub mod scan;

pub use scan::{Scanner, ScannerError};

#[cfg(test)]
mod tests;
