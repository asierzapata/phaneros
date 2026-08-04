use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use crate::blob_repository::{
    Blob, BlobRepository, Hash, WritableBlobRepository, repository::BlobRepositoryError,
};

#[derive(Debug, Default)]
pub struct InMemoryBlobRepository {
    blobs: RwLock<HashMap<Hash, Blob>>,
}

impl InMemoryBlobRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_internal(&self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        let mut blobs = self.blobs.write().unwrap();
        blobs.entry(hash).or_insert(blob);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.blobs.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.blobs.read().unwrap().is_empty()
    }
}

#[async_trait]
impl BlobRepository for InMemoryBlobRepository {
    async fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError> {
        Ok(self.blobs.read().unwrap().get(hash).cloned())
    }

    async fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError> {
        Ok(self.blobs.read().unwrap().contains_key(hash))
    }
}

#[async_trait]
impl WritableBlobRepository for InMemoryBlobRepository {
    async fn insert(&self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        self.insert_internal(hash, blob)
    }
}
