use async_trait::async_trait;
use thiserror::Error;

use std::collections::{HashMap, HashSet};

use crate::node_repository::{Hash, Node};

#[derive(Debug, Error, PartialEq)]
pub enum NodeRepositoryError {
    #[error("Failed to insert node for hash: {0}")]
    InsertFailed(Hash),
    #[error("Failed to set root hash: {0}")]
    SetRootFailed(Hash),
    #[error("Failed to retrieve node for hash: {0}")]
    NodeRetrieveFailed(Hash),
    #[error("Failed to retrieve root hash")]
    RootRetrieveFailed,
    /// The store rejected a root PUT because another client moved the root
    /// (HTTP 409). `actual` is the store's current root, if any. Per the sync
    /// protocol the client must re-scan and reconcile from scratch rather than
    /// retry the same PUT; the syncer does this naturally on the next watcher
    /// event, using the corrected expected root the caller records here.
    #[error("root compare-and-swap lost the race; store root is now {actual:?}")]
    RootConflict { actual: Option<Hash> },
}

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn root_hash(&self) -> Result<Option<Hash>, NodeRepositoryError>;
    async fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError>;

    /// Returns the subset of `hashes` that this repository does NOT hold.
    /// Default implementation probes one at a time (used by InMemory).
    async fn get_missing(&self, hashes: &[Hash]) -> Result<HashSet<Hash>, NodeRepositoryError> {
        let mut missing = HashSet::new();
        for h in hashes {
            if self.get_node(h).await?.is_none() {
                missing.insert(h.clone());
            }
        }
        Ok(missing)
    }

    /// Fetches multiple nodes in a single call. Returns a map of hash→Node for nodes that exist.
    /// Default implementation fetches one at a time (used by InMemory).
    async fn get_nodes_batch(
        &self,
        hashes: &[Hash],
    ) -> Result<HashMap<Hash, Node>, NodeRepositoryError> {
        let mut nodes = HashMap::new();
        for h in hashes {
            if let Some(node) = self.get_node(h).await? {
                nodes.insert(h.clone(), node);
            }
        }
        Ok(nodes)
    }
}

/// A node store that can also be written to. The syncer reads both sides
/// through `NodeRepository` and pushes missing nodes through this.
#[async_trait]
pub trait WritableNodeRepository: NodeRepository + Send + Sync {
    async fn insert(&self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError>;
    async fn set_root(&self, hash: Hash) -> Result<(), NodeRepositoryError>;
}
