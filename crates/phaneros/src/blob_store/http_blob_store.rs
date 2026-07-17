use std::collections::HashMap;

use crate::blob_store::{Blob, BlobStore, Hash, WritableBlobStore};

// For now the HTTP node store its just an in-memory node store,
// but in the future it will be a node store that fetches nodes from a remote HTTP server.

#[derive(Debug, Default)]
pub struct HttpBlobStore {
    blobs: HashMap<Hash, Blob>,
}

impl HttpBlobStore {
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

impl BlobStore for HttpBlobStore {
    fn get_blob(&self, hash: &Hash) -> Option<Blob> {
        self.blobs.get(hash).cloned()
    }

    fn contains(&self, hash: &Hash) -> bool {
        self.blobs.contains_key(hash)
    }
}

impl WritableBlobStore for HttpBlobStore {
    fn insert(&mut self, hash: Hash, blob: Blob) {
        HttpBlobStore::insert(self, hash, blob);
    }
}
