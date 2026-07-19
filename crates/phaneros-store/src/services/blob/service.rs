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
    #[error("blob bytes are not present, cannot commit")]
    BytesMissing,
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

        if !self.bytes_repository.has(hash).await? {
            return Err(BlobServiceError::BytesMissing);
        }

        self.metadata_repository.mark_committed(hash).await?;
        Ok(())
    }

    pub async fn create_download_ticket(
        &self,
        hash: &Hash,
    ) -> Result<Option<Ticket>, BlobServiceError> {
        if !self.metadata_repository.exists(hash).await? {
            return Ok(None);
        }

        Ok(Some(Ticket {
            url: format!("{}/api/blobs/{}/bytes", self.base_url, hash),
            expires_at: None,
        }))
    }

    pub async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobServiceError> {
        Ok(self.bytes_repository.get_bytes(hash).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::blob::{FsBlobBytesRepository, SqliteBlobMetadataRepository};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn service() -> BlobService {
        let options = SqliteConnectOptions::new().in_memory(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        let root = std::env::temp_dir().join(format!(
            "phaneros-svc-test-{}-{:p}",
            std::process::id(),
            &pool
        ));

        BlobService::new(
            Arc::new(SqliteBlobMetadataRepository::new(pool)),
            Arc::new(FsBlobBytesRepository::new(root)),
            "http://localhost".to_string(),
        )
    }

    // Walks the whole upload lifecycle across the metadata and bytes planes:
    // declare -> (size enforced) -> store bytes -> (commit gated on bytes) -> commit.
    #[tokio::test]
    async fn upload_lifecycle_declares_stores_and_commits() {
        let svc = service().await;
        let hash: Hash = "a".repeat(64);

        assert!(!svc.exists(&hash).await.unwrap());

        // Ticket declares the size but does not make the blob held.
        assert!(matches!(
            svc.create_ticket(&hash, 5).await.unwrap(),
            UploadTicket::Upload(_)
        ));
        assert!(!svc.exists(&hash).await.unwrap());

        // Bytes are enforced against the declared size.
        assert!(matches!(
            svc.put_bytes(&hash, Bytes::from_static(b"hi")).await,
            Err(BlobServiceError::SizeMismatch {
                declared: 5,
                actual: 2
            })
        ));

        // Commit before the bytes land is refused.
        assert!(matches!(
            svc.confirm_upload(&hash).await,
            Err(BlobServiceError::BytesMissing)
        ));

        // Correct bytes store, but the blob is still only declared until commit.
        svc.put_bytes(&hash, Bytes::from_static(b"hello"))
            .await
            .unwrap();
        assert!(!svc.exists(&hash).await.unwrap());

        // Commit flips it to held; download and the dedup short-circuit follow.
        svc.confirm_upload(&hash).await.unwrap();
        assert!(svc.exists(&hash).await.unwrap());
        assert!(svc.create_download_ticket(&hash).await.unwrap().is_some());
        assert!(matches!(
            svc.create_ticket(&hash, 5).await.unwrap(),
            UploadTicket::AlreadyStored
        ));
    }

    #[tokio::test]
    async fn put_bytes_without_a_ticket_is_rejected() {
        let svc = service().await;
        let hash: Hash = "b".repeat(64);
        assert!(matches!(
            svc.put_bytes(&hash, Bytes::from_static(b"x")).await,
            Err(BlobServiceError::Unregistered)
        ));
    }

    #[tokio::test]
    async fn download_ticket_absent_for_unknown_blob() {
        let svc = service().await;
        assert!(
            svc.create_download_ticket(&"c".repeat(64))
                .await
                .unwrap()
                .is_none()
        );
    }
}
