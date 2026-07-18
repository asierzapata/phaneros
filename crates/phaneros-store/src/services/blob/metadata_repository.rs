use async_trait::async_trait;
use phaneros_sync::hash::Hash;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlobMetadataRepositoryError {
    #[error("not implemented")]
    NotImplemented,
}

#[async_trait]
pub trait BlobMetadataRepository {
    async fn exists(&self, hash: &Hash) -> Result<bool, BlobMetadataRepositoryError>;

    async fn record(&self, hash: &Hash) -> Result<(), BlobMetadataRepositoryError>;
}
