pub mod http_blob_repository;
pub mod in_memory_blob_repository;
pub mod repository;

pub use http_blob_repository::HttpBlobRepository;
pub use in_memory_blob_repository::InMemoryBlobRepository;
pub use phaneros_sync::blob::{Blob, BlobRef};
pub use phaneros_sync::hash::Hash;
pub use repository::{BlobRepository, BlobRepositoryError, WritableBlobRepository};

#[cfg(test)]
mod tests;
