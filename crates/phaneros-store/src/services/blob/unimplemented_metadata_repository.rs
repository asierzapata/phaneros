use async_trait::async_trait;
use phaneros_sync::hash::Hash;

use super::metadata_repository::{BlobMetadataRepository, BlobMetadataRepositoryError};

/// Placeholder until a real repository (SQLite) exists. Every method
/// errors, so routes wired against it behave like the hardcoded 501 stubs
/// did.
#[derive(Default)]
pub struct UnimplementedBlobMetadataRepository;

#[async_trait]
impl BlobMetadataRepository for UnimplementedBlobMetadataRepository {
    async fn exists(&self, _hash: &Hash) -> Result<bool, BlobMetadataRepositoryError> {
        Err(BlobMetadataRepositoryError::NotImplemented)
    }

    async fn record(&self, _hash: &Hash) -> Result<(), BlobMetadataRepositoryError> {
        Err(BlobMetadataRepositoryError::NotImplemented)
    }
}
