pub mod file_chunker;
pub mod ignore;
pub mod scan;

pub use ignore::IgnoreFilter;
pub use scan::{Scanner, ScannerError};

#[cfg(test)]
mod tests;
