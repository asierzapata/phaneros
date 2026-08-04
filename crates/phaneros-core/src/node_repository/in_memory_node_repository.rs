use std::collections::HashMap;
use std::sync::RwLock;
use async_trait::async_trait;

use crate::node_repository::{
    Hash, Node, NodeRepository, WritableNodeRepository, repository::NodeRepositoryError,
};

#[derive(Debug, Default)]
pub struct InMemoryNodeRepository {
    root: RwLock<Option<Hash>>,
    nodes: RwLock<HashMap<Hash, Node>>,
}

impl InMemoryNodeRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_internal(&self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        let mut nodes = self.nodes.write().unwrap();
        nodes.entry(hash).or_insert(node);
        Ok(())
    }

    pub fn set_root_internal(&self, hash: Hash) -> Result<(), NodeRepositoryError> {
        let mut root = self.root.write().unwrap();
        *root = Some(hash);
        Ok(())
    }

    pub fn get_node_internal(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        Ok(self.nodes.read().unwrap().get(hash).cloned())
    }

    pub fn len(&self) -> usize {
        self.nodes.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.read().unwrap().is_empty()
    }
}

#[async_trait]
impl NodeRepository for InMemoryNodeRepository {
    async fn root_hash(&self) -> Result<Option<Hash>, NodeRepositoryError> {
        Ok(self.root.read().unwrap().clone())
    }

    async fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        Ok(self.nodes.read().unwrap().get(hash).cloned())
    }
}

#[async_trait]
impl WritableNodeRepository for InMemoryNodeRepository {
    async fn insert(&self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        self.insert_internal(hash, node)
    }

    async fn set_root(&self, hash: Hash) -> Result<(), NodeRepositoryError> {
        self.set_root_internal(hash)
    }
}
