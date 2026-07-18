use async_trait::async_trait;
use phaneros_sync::{hash::Hash, node::Node};

use super::repository::{NodeRepository, NodeRepositoryError, Version};

/// Placeholder until a real repository (SQLite) exists. Every method errors,
/// so routes wired against it behave the same as the hardcoded 501 stubs did.
#[derive(Default)]
pub struct UnimplementedNodeRepository;

#[async_trait]
impl NodeRepository for UnimplementedNodeRepository {
    async fn get_root(&self, _drive_id: &str) -> Result<Option<Hash>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn put_root(
        &self,
        _drive_id: &str,
        _new: Hash,
        _expected: Option<Hash>,
    ) -> Result<(), NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn get_node(
        &self,
        _drive_id: &str,
        _hash: &Hash,
    ) -> Result<Option<Node>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn put_node(
        &self,
        _drive_id: &str,
        _hash: Hash,
        _node: Node,
    ) -> Result<(), NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn list_versions(&self, _drive_id: &str) -> Result<Vec<Version>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }
}
