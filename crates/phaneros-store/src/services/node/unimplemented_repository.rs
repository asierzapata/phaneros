use async_trait::async_trait;
use phaneros_sync::{hash::Hash, node::Node};

use super::repository::{NodeRepository, NodeRepositoryError, Version, VersionEvent};

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
    ) -> Result<VersionEvent, NodeRepositoryError> {
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

    async fn get_missing_nodes(
        &self,
        _drive_id: &str,
        _hashes: &[Hash],
    ) -> Result<Vec<Hash>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn get_nodes_batch(
        &self,
        _drive_id: &str,
        _hashes: &[Hash],
    ) -> Result<std::collections::HashMap<Hash, Node>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn list_versions(&self, _drive_id: &str) -> Result<Vec<Version>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn max_version_id(&self) -> Result<i64, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn list_versions_after(
        &self,
        _after_id: i64,
        _limit: i64,
    ) -> Result<Vec<VersionEvent>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }

    async fn list_drive_versions_after(
        &self,
        _drive_id: &str,
        _after_id: i64,
        _limit: i64,
    ) -> Result<Vec<VersionEvent>, NodeRepositoryError> {
        Err(NodeRepositoryError::NotImplemented)
    }
}
