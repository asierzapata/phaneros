use async_trait::async_trait;
use phaneros_sync::{hash::Hash, node::Node};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    pub root: Hash,
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionEvent {
    pub id: i64,
    pub drive_id: String,
    pub root: Hash,
    pub at: i64,
}

#[derive(Debug, Error)]
pub enum NodeRepositoryError {
    #[error("not implemented")]
    NotImplemented,
    #[error("root compare-and-swap mismatch: expected {expected:?}, found {actual:?}")]
    RootMismatch {
        expected: Option<Hash>,
        actual: Option<Hash>,
    },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("stored node is corrupt: {0}")]
    Corruption(#[from] serde_json::Error),
}

#[async_trait]
pub trait NodeRepository {
    async fn get_root(&self, drive_id: &str) -> Result<Option<Hash>, NodeRepositoryError>;

    async fn put_root(
        &self,
        drive_id: &str,
        new: Hash,
        expected: Option<Hash>,
    ) -> Result<VersionEvent, NodeRepositoryError>;

    async fn get_node(
        &self,
        drive_id: &str,
        hash: &Hash,
    ) -> Result<Option<Node>, NodeRepositoryError>;

    async fn put_node(
        &self,
        drive_id: &str,
        hash: Hash,
        node: Node,
    ) -> Result<(), NodeRepositoryError>;

    async fn get_missing_nodes(
        &self,
        drive_id: &str,
        hashes: &[Hash],
    ) -> Result<Vec<Hash>, NodeRepositoryError>;

    async fn get_nodes_batch(
        &self,
        drive_id: &str,
        hashes: &[Hash],
    ) -> Result<std::collections::HashMap<Hash, Node>, NodeRepositoryError>;

    async fn list_versions(&self, drive_id: &str) -> Result<Vec<Version>, NodeRepositoryError>;

    async fn max_version_id(&self) -> Result<i64, NodeRepositoryError>;

    async fn list_versions_after(
        &self,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<VersionEvent>, NodeRepositoryError>;

    async fn list_drive_versions_after(
        &self,
        drive_id: &str,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<VersionEvent>, NodeRepositoryError>;
}
