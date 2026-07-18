use std::sync::Arc;

use phaneros_sync::{hash::Hash, node::Node};

use super::repository::{NodeRepository, NodeRepositoryError, Version};

#[derive(Clone)]
pub struct NodeService {
    repository: Arc<dyn NodeRepository + Send + Sync>,
}

impl NodeService {
    pub fn new(repository: Arc<dyn NodeRepository + Send + Sync>) -> Self {
        Self { repository }
    }

    pub async fn get_root(&self, drive_id: &str) -> Result<Option<Hash>, NodeRepositoryError> {
        self.repository.get_root(drive_id).await
    }

    pub async fn put_root(
        &self,
        drive_id: &str,
        new: Hash,
        expected: Option<Hash>,
    ) -> Result<(), NodeRepositoryError> {
        self.repository.put_root(drive_id, new, expected).await
    }

    pub async fn get_node(
        &self,
        drive_id: &str,
        hash: &Hash,
    ) -> Result<Option<Node>, NodeRepositoryError> {
        self.repository.get_node(drive_id, hash).await
    }

    pub async fn put_node(
        &self,
        drive_id: &str,
        hash: Hash,
        node: Node,
    ) -> Result<(), NodeRepositoryError> {
        self.repository.put_node(drive_id, hash, node).await
    }

    pub async fn list_versions(&self, drive_id: &str) -> Result<Vec<Version>, NodeRepositoryError> {
        self.repository.list_versions(drive_id).await
    }
}
