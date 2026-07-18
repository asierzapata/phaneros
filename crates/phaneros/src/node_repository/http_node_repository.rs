use std::collections::HashMap;

use crate::node_repository::{Hash, Node, NodeRepository, WritableNodeRepository, repository::NodeRepositoryError};

// For now the HTTP node store its just an in-memory node store,
// but in the future it will be a node store that fetches nodes from a remote HTTP server.

#[derive(Debug, Default)]
pub struct HttpNodeRepository {
    root: Option<Hash>,
    nodes: HashMap<Hash, Node>,
}

impl HttpNodeRepository {
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

impl NodeRepository for HttpNodeRepository {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeRepositoryError> {
        Ok(self.root.as_ref())
    }

    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        Ok(self.nodes.get(hash).cloned())
    }
}

impl WritableNodeRepository for HttpNodeRepository {
    fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        HttpNodeRepository::insert(self, hash, node)
    }

    fn set_root(&mut self, hash: Hash) -> Result<(), NodeRepositoryError> {
        HttpNodeRepository::set_root(self, hash)
    }
}
