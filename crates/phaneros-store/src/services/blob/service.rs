use std::sync::Arc;

use bytes::Bytes;
use phaneros_sync::hash::Hash;
use serde::Serialize;
use thiserror::Error;

use super::bytes_repository::{BlobBytesRepository, BlobBytesRepositoryError};
use super::metadata_repository::{BlobMetadataRepository, BlobMetadataRepositoryError};

#[derive(Debug, Error)]
pub enum BlobServiceError {
    #[error(transparent)]
    Metadata(#[from] BlobMetadataRepositoryError),
    #[error(transparent)]
    Bytes(#[from] BlobBytesRepositoryError),
    #[error("blob was not registered for upload")]
    Unregistered,
    #[error("declared size {declared} does not match uploaded size {actual}")]
    SizeMismatch { declared: i64, actual: i64 },
}

#[derive(Debug, Clone, Serialize)]
pub struct Ticket {
    pub url: String,
    pub expires_at: Option<i64>,
}

pub enum UploadTicket {
    AlreadyStored,
    Upload(Ticket),
}

#[derive(Clone)]
pub struct BlobService {
    metadata_repository: Arc<dyn BlobMetadataRepository + Send + Sync>,
    bytes_repository: Arc<dyn BlobBytesRepository + Send + Sync>,
    base_url: String,
}

impl BlobService {
    pub fn new(
        metadata_repository: Arc<dyn BlobMetadataRepository + Send + Sync>,
        bytes_repository: Arc<dyn BlobBytesRepository + Send + Sync>,
        base_url: String,
    ) -> Self {
        Self {
            metadata_repository,
            bytes_repository,
            base_url,
        }
    }

    pub async fn exists(&self, hash: &Hash) -> Result<bool, BlobServiceError> {
        Ok(self.metadata_repository.exists(hash).await?)
    }

    pub async fn create_ticket(
        &self,
        hash: &Hash,
        size: i64,
    ) -> Result<UploadTicket, BlobServiceError> {
        if self.metadata_repository.exists(hash).await? {
            return Ok(UploadTicket::AlreadyStored);
        }

        self.metadata_repository.declare(hash, size).await?;

        Ok(UploadTicket::Upload(Ticket {
            url: format!("{}/api/blobs/{}/bytes", self.base_url, hash),
            expires_at: None,
        }))
    }

    pub async fn put_bytes(&self, hash: &Hash, bytes: Bytes) -> Result<(), BlobServiceError> {
        let declared = self
            .metadata_repository
            .declared_size(hash)
            .await?
            .ok_or(BlobServiceError::Unregistered)?;

        let actual = bytes.len() as i64;
        if actual != declared {
            return Err(BlobServiceError::SizeMismatch { declared, actual });
        }

        self.bytes_repository.put_bytes(hash, bytes).await?;
        Ok(())
    }

    pub async fn confirm_upload(&self, hash: &Hash) -> Result<(), BlobServiceError> {
        if self
            .metadata_repository
            .declared_size(hash)
            .await?
            .is_none()
        {
            return Err(BlobServiceError::Unregistered);
        }

        self.metadata_repository.mark_committed(hash).await?;
        Ok(())
    }

    pub async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobServiceError> {
        Ok(self.bytes_repository.get_bytes(hash).await?)
    }
}
