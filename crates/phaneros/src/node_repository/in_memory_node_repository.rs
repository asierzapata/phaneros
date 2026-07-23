use std::collections::HashMap;

use crate::node_repository::{
    Hash, Node, NodeRepository, WritableNodeRepository, repository::NodeRepositoryError,
};

#[derive(Debug, Default)]
pub struct InMemoryNodeRepository {
    root: Option<Hash>,
    nodes: HashMap<Hash, Node>,
}

impl InMemoryNodeRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        self.nodes.entry(hash).or_insert(node);
        Ok(())
    }

    pub fn set_root(&mut self, hash: Hash) -> Result<(), NodeRepositoryError> {
        self.root = Some(hash);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl NodeRepository for InMemoryNodeRepository {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeRepositoryError> {
        Ok(self.root.as_ref())
    }

    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        Ok(self.nodes.get(hash).cloned())
    }
}

impl WritableNodeRepository for InMemoryNodeRepository {
    fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        InMemoryNodeRepository::insert(self, hash, node)
    }

    fn set_root(&mut self, hash: Hash) -> Result<(), NodeRepositoryError> {
        InMemoryNodeRepository::set_root(self, hash)
    }
}
