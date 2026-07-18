use std::sync::Arc;

use bytes::Bytes;
use phaneros_sync::hash::Hash;
use thiserror::Error;

use super::bytes_repository::{BlobBytesRepository, BlobBytesRepositoryError};
use super::metadata_repository::{BlobMetadataRepository, BlobMetadataRepositoryError};

#[derive(Debug, Error)]
pub enum BlobServiceError {
    #[error(transparent)]
    Metadata(#[from] BlobMetadataRepositoryError),
    #[error(transparent)]
    Bytes(#[from] BlobBytesRepositoryError),
}

#[derive(Clone)]
pub struct BlobService {
    metadata_repository: Arc<dyn BlobMetadataRepository + Send + Sync>,
    bytes_repository: Arc<dyn BlobBytesRepository + Send + Sync>,
}

impl BlobService {
    pub fn new(
        metadata_repository: Arc<dyn BlobMetadataRepository + Send + Sync>,
        bytes_repository: Arc<dyn BlobBytesRepository + Send + Sync>,
    ) -> Self {
        Self {
            metadata_repository,
            bytes_repository,
        }
    }

    pub async fn exists(&self, hash: &Hash) -> Result<bool, BlobServiceError> {
        Ok(self.metadata_repository.exists(hash).await?)
    }

    pub async fn put_bytes(&self, hash: &Hash, bytes: Bytes) -> Result<(), BlobServiceError> {
        // Important to first put the bytes and then record the metadata to avoid race
        // condition where the metadata is recorded but the bytes are not yet stored.
        self.bytes_repository.put_bytes(hash, bytes).await?;
        self.metadata_repository.record(hash).await?;
        Ok(())
    }

    pub async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobServiceError> {
        Ok(self.bytes_repository.get_bytes(hash).await?)
    }
}
