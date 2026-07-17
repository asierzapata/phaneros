use std::collections::HashMap;

use crate::blob_store::{Blob, BlobStore, Hash, WritableBlobStore, store::BlobStoreError};

#[derive(Debug, Default)]
pub struct InMemoryBlobStore {
    blobs: HashMap<Hash, Blob>,
}

impl InMemoryBlobStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobStoreError> {
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

impl BlobStore for InMemoryBlobStore {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobStoreError> {
        Ok(self.blobs.get(hash).cloned())
    }

    fn contains(&self, hash: &Hash) -> Result<bool, BlobStoreError> {
        Ok(self.blobs.contains_key(hash))
    }
}

impl WritableBlobStore for InMemoryBlobStore {
    fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobStoreError> {
        InMemoryBlobStore::insert(self, hash, blob)
    }
}
