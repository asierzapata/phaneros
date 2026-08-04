use async_trait::async_trait;
use phaneros_sync::hash::Hash;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlobMetadataRepositoryError {
    #[error("not implemented")]
    NotImplemented,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobMetadataInfo {
    pub hash: Hash,
    pub size: i64,
    pub uncompressed_size: Option<i64>,
    pub compression: String,
    pub committed_at: Option<i64>,
}

#[async_trait]
pub trait BlobMetadataRepository {
    async fn exists(&self, hash: &Hash) -> Result<bool, BlobMetadataRepositoryError>;

    async fn declare(
        &self,
        hash: &Hash,
        size: i64,
        uncompressed_size: Option<i64>,
        compression: Option<&str>,
    ) -> Result<(), BlobMetadataRepositoryError>;

    async fn declared_size(&self, hash: &Hash) -> Result<Option<i64>, BlobMetadataRepositoryError>;

    async fn get_metadata(
        &self,
        hash: &Hash,
    ) -> Result<Option<BlobMetadataInfo>, BlobMetadataRepositoryError>;

    async fn mark_committed(&self, hash: &Hash) -> Result<(), BlobMetadataRepositoryError>;

    async fn get_missing(&self, hashes: &[Hash]) -> Result<Vec<Hash>, BlobMetadataRepositoryError>;
}
