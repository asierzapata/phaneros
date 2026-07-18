use async_trait::async_trait;
use phaneros_sync::{hash::Hash, node::Node};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Version {
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
}

#[async_trait]
pub trait NodeRepository {
    async fn get_root(&self, drive_id: &str) -> Result<Option<Hash>, NodeRepositoryError>;

    async fn put_root(
        &self,
        drive_id: &str,
        new: Hash,
        expected: Option<Hash>,
    ) -> Result<(), NodeRepositoryError>;

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

    async fn list_versions(&self, drive_id: &str) -> Result<Vec<Version>, NodeRepositoryError>;
}
