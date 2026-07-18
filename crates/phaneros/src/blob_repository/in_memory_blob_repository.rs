use std::collections::HashMap;

use crate::blob_repository::{Blob, BlobRepository, Hash, WritableBlobRepository, repository::BlobRepositoryError};

#[derive(Debug, Default)]
pub struct InMemoryBlobRepository {
    blobs: HashMap<Hash, Blob>,
}

impl InMemoryBlobRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        self.blobs.entry(hash).or_insert(blob);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }
}

impl BlobRepository for InMemoryBlobRepository {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError> {
        Ok(self.blobs.get(hash).cloned())
    }

    fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError> {
        Ok(self.blobs.contains_key(hash))
    }
}

impl WritableBlobRepository for InMemoryBlobRepository {
    fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        InMemoryBlobRepository::insert(self, hash, blob)
    }
}
