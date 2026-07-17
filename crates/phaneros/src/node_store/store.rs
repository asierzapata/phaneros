use thiserror::Error;

use crate::node_store::{Hash, Node};

#[derive(Debug, Error, PartialEq)]
pub enum NodeStoreError {
    #[error("Failed to insert node for hash: {0}")]
    InsertFailed(Hash),
    #[error("Failed to set root hash: {0}")]
    SetRootFailed(Hash),
    #[error("Failed to retrieve node for hash: {0}")]
    NodeRetrieveFailed(Hash),
    #[error("Failed to retrieve root hash")]
    RootRetrieveFailed,
}

pub trait NodeStore {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeStoreError>;
    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeStoreError>;
}

/// A node store that can also be written to. The syncer reads both sides
/// through `NodeStore` and pushes missing nodes through this.
pub trait WritableNodeStore: NodeStore {
    fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeStoreError>;
    fn set_root(&mut self, hash: Hash) -> Result<(), NodeStoreError>;
}
