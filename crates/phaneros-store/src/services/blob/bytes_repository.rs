use async_trait::async_trait;
use bytes::Bytes;
use phaneros_sync::hash::Hash;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlobBytesRepositoryError {
    #[error("not implemented")]
    NotImplemented,
    #[error("invalid blob hash")]
    InvalidHash,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait BlobBytesRepository {
    async fn put_bytes(&self, hash: &Hash, bytes: Bytes) -> Result<(), BlobBytesRepositoryError>;

    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobBytesRepositoryError>;

    async fn has(&self, hash: &Hash) -> Result<bool, BlobBytesRepositoryError>;
}
