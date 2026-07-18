use async_trait::async_trait;
use bytes::Bytes;
use phaneros_sync::hash::Hash;

use super::bytes_repository::{BlobBytesRepository, BlobBytesRepositoryError};

/// Placeholder until a real repository (filesystem-backed) exists. Every
/// method errors, so routes wired against it behave like the hardcoded 501
/// stubs did.
#[derive(Default)]
pub struct UnimplementedBlobBytesRepository;

#[async_trait]
impl BlobBytesRepository for UnimplementedBlobBytesRepository {
    async fn put_bytes(&self, _hash: &Hash, _bytes: Bytes) -> Result<(), BlobBytesRepositoryError> {
        Err(BlobBytesRepositoryError::NotImplemented)
    }

    async fn get_bytes(&self, _hash: &Hash) -> Result<Option<Bytes>, BlobBytesRepositoryError> {
        Err(BlobBytesRepositoryError::NotImplemented)
    }
}
