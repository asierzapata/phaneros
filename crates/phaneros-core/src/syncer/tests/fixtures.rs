use async_trait::async_trait;
use crate::blob_repository::WritableBlobRepository;
use crate::node_repository::WritableNodeRepository;
use std::sync::RwLock;

use crate::blob_repository::{Blob, BlobRef, BlobRepository, InMemoryBlobRepository};
use crate::node_repository::{
    Entry, Hash, InMemoryNodeRepository, Node, NodeRepository, NodeRepositoryError,
};

/// One side of a sync: node store + blob store, which always travel together.
pub(super) struct TestStore {
    pub(super) nodes: InMemoryNodeRepository,
    pub(super) blobs: InMemoryBlobRepository,
}

impl TestStore {
    pub(super) fn new() -> Self {
        TestStore {
            nodes: InMemoryNodeRepository::new(),
            blobs: InMemoryBlobRepository::new(),
        }
    }

    /// Inserts a file node built from `content` — and, like the scanner does,
    /// also stores the content bytes in the blob store.
    pub(super) async fn add_file(&mut self, name: &str, content: &[u8]) -> Entry {
        let blob_ref = BlobRef::from_bytes(content);
        self.blobs
            .insert(
                blob_ref.hash.clone(),
                Blob {
                    bytes: content.to_vec(),
                },
            )
            .await
            .unwrap();
        let (hash, node) = Node::file(vec![blob_ref]);
        self.nodes.insert(hash.clone(), node).await.unwrap();
        Entry::new(name, hash)
    }

    /// Inserts a folder node from child entries and returns its entry.
    pub(super) async fn add_folder(
        &mut self,
        name: &str,
        folders: Vec<Entry>,
        files: Vec<Entry>,
    ) -> Entry {
        let (hash, node) = Node::folder(folders, files);
        self.nodes.insert(hash.clone(), node).await.unwrap();
        Entry::new(name, hash)
    }
}

/// A NodeRepository wrapper that records every hash requested from it, so tests
/// can assert that shared subtrees are pruned (never walked).
pub(super) struct RecordingStore<'a> {
    pub(super) inner: &'a InMemoryNodeRepository,
    pub(super) requested: RwLock<Vec<Hash>>,
}

impl<'a> RecordingStore<'a> {
    pub(super) fn new(inner: &'a InMemoryNodeRepository) -> Self {
        RecordingStore {
            inner,
            requested: RwLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl NodeRepository for RecordingStore<'_> {
    async fn root_hash(&self) -> Result<Option<Hash>, NodeRepositoryError> {
        self.inner.root_hash().await
    }

    async fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        self.requested.write().unwrap().push(hash.clone());
        self.inner.get_node(hash).await
    }
}

pub(super) async fn assert_has_node(store: &impl NodeRepository, hash: &Hash) {
    assert!(
        store.get_node(hash).await.unwrap().is_some(),
        "node {} should exist",
        hash
    );
}

pub(super) async fn assert_missing_node(store: &impl NodeRepository, hash: &Hash) {
    assert!(
        store.get_node(hash).await.unwrap().is_none(),
        "node {} should be missing",
        hash
    );
}

pub(super) async fn assert_has_blob(store: &impl BlobRepository, hash: &Hash) {
    assert!(
        store.get_blob(hash).await.unwrap().is_some(),
        "blob {} should exist",
        hash
    );
}

pub(super) async fn assert_missing_blob(store: &impl BlobRepository, hash: &Hash) {
    assert!(
        store.get_blob(hash).await.unwrap().is_none(),
        "blob {} should be missing",
        hash
    );
}
