use std::collections::HashMap;

use crate::blob_repository::{Blob, BlobRepository, Hash, WritableBlobRepository, repository::BlobRepositoryError};

// For now the HTTP node store its just an in-memory node store,
// but in the future it will be a node store that fetches nodes from a remote HTTP server.

#[derive(Debug, Default)]
pub struct HttpBlobRepository {
    blobs: HashMap<Hash, Blob>,
}

impl HttpBlobRepository {
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

impl BlobRepository for HttpBlobRepository {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError> {
        Ok(self.blobs.get(hash).cloned())
    }

    fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError> {
        Ok(self.blobs.contains_key(hash))
    }
}

impl WritableBlobRepository for HttpBlobRepository {
    fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        HttpBlobRepository::insert(self, hash, blob)
    }
}
