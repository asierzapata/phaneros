use async_trait::async_trait;
use thiserror::Error;

use std::collections::{HashMap, HashSet};

use crate::blob_repository::{Blob, Hash};

#[derive(Debug, Error)]
pub enum BlobRepositoryError {
    #[error("Failed to insert blob for hash: {0}")]
    InsertFailed(Hash),
    #[error("Failed to retrieve blob for hash: {0}")]
    RetrieveFailed(Hash),
    #[error("Failed to check existence of blob for hash: {0}")]
    ExistenceCheckFailed(Hash),
    #[error("Upload rejected by store for hash {hash}: {reason}")]
    UploadRejected { hash: Hash, reason: String },
}

#[async_trait]
pub trait BlobRepository: Send + Sync {
    async fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError>;
    async fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError>;
    
    async fn get_blobs_batch(
        &self,
        hashes: &[Hash],
    ) -> Result<HashMap<Hash, Blob>, BlobRepositoryError> {
        let mut blobs = HashMap::new();
        for h in hashes {
            if let Some(blob) = self.get_blob(h).await? {
                blobs.insert(h.clone(), blob);
            }
        }
        Ok(blobs)
    }

    /// Returns the subset of `hashes` that this repository does NOT hold.
    /// Default implementation probes one at a time (used by InMemory).
    async fn get_missing(&self, hashes: &[Hash]) -> Result<HashSet<Hash>, BlobRepositoryError> {
        let mut missing = HashSet::new();
        for h in hashes {
            if !self.contains(h).await? {
                missing.insert(h.clone());
            }
        }
        Ok(missing)
    }
}

/// A blob store that can also be written to. The syncer reads both sides
/// through `BlobRepository` and pushes missing blobs through this.
#[async_trait]
pub trait WritableBlobRepository: BlobRepository + Send + Sync {
    async fn insert(&self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError>;
}
