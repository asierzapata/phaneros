use async_trait::async_trait;
use phaneros_sync::hash::Hash;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlobMetadataRepositoryError {
    #[error("not implemented")]
    NotImplemented,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[async_trait]
pub trait BlobMetadataRepository {
    async fn exists(&self, hash: &Hash) -> Result<bool, BlobMetadataRepositoryError>;

    async fn declare(&self, hash: &Hash, size: i64) -> Result<(), BlobMetadataRepositoryError>;

    async fn declared_size(&self, hash: &Hash) -> Result<Option<i64>, BlobMetadataRepositoryError>;

    async fn mark_committed(&self, hash: &Hash) -> Result<(), BlobMetadataRepositoryError>;
}
