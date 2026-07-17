use std::collections::HashMap;

use crate::blob_store::{Blob, BlobStore, Hash, WritableBlobStore};

#[derive(Debug, Default)]
pub struct InMemoryBlobStore {
    blobs: HashMap<Hash, Blob>,
}

impl InMemoryBlobStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, hash: Hash, blob: Blob) {
        self.blobs.entry(hash).or_insert(blob);
    }

    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }
}

impl BlobStore for InMemoryBlobStore {
    fn get_blob(&self, hash: &Hash) -> Option<Blob> {
        self.blobs.get(hash).cloned()
    }

    fn contains(&self, hash: &Hash) -> bool {
        self.blobs.contains_key(hash)
    }
}

impl WritableBlobStore for InMemoryBlobStore {
    fn insert(&mut self, hash: Hash, blob: Blob) {
        InMemoryBlobStore::insert(self, hash, blob);
    }
}
