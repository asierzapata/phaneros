use thiserror::Error;

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
}

pub trait NodeRepository {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeRepositoryError>;
    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError>;
}

/// A node store that can also be written to. The syncer reads both sides
/// through `NodeRepository` and pushes missing nodes through this.
pub trait WritableNodeRepository: NodeRepository {
    fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError>;
    fn set_root(&mut self, hash: Hash) -> Result<(), NodeRepositoryError>;
}
