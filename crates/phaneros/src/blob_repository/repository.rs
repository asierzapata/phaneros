use thiserror::Error;

use crate::blob_repository::{Blob, Hash};

#[derive(Debug, Error)]
pub enum BlobRepositoryError {
    #[error("Failed to insert blob for hash: {0}")]
    InsertFailed(Hash),
    #[error("Failed to retrieve blob for hash: {0}")]
    RetrieveFailed(Hash),
    #[error("Failed to check existence of blob for hash: {0}")]
    ExistenceCheckFailed(Hash),
}

pub trait BlobRepository {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError>;
    fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError>;
}

/// A blob store that can also be written to. The syncer reads both sides
/// through `BlobRepository` and pushes missing blobs through this.
pub trait WritableBlobRepository: BlobRepository {
    fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError>;
}
