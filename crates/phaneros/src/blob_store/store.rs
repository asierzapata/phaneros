use thiserror::Error;

use crate::blob_store::{Blob, Hash};

#[derive(Debug, Error)]
pub enum BlobStoreError {
    #[error("Failed to insert blob for hash: {0}")]
    InsertFailed(Hash),
    #[error("Failed to retrieve blob for hash: {0}")]
    RetrieveFailed(Hash),
    #[error("Failed to check existence of blob for hash: {0}")]
    ExistenceCheckFailed(Hash),
}

pub trait BlobStore {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobStoreError>;
    fn contains(&self, hash: &Hash) -> Result<bool, BlobStoreError>;
}

/// A blob store that can also be written to. The syncer reads both sides
/// through `BlobStore` and pushes missing blobs through this.
pub trait WritableBlobStore: BlobStore {
    fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobStoreError>;
}
