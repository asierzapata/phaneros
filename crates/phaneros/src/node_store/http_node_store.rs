use std::collections::HashMap;

use crate::node_store::{Hash, Node, NodeStore, WritableNodeStore, store::NodeStoreError};

// For now the HTTP node store its just an in-memory node store,
// but in the future it will be a node store that fetches nodes from a remote HTTP server.

#[derive(Debug, Default)]
pub struct HttpNodeStore {
    root: Option<Hash>,
    nodes: HashMap<Hash, Node>,
}

impl HttpNodeStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeStoreError> {
        self.nodes.entry(hash).or_insert(node);
        Ok(())
    }

    pub fn set_root(&mut self, hash: Hash) -> Result<(), NodeStoreError> {
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

impl NodeStore for HttpNodeStore {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeStoreError> {
        self.root
            .as_ref()
            .ok_or(NodeStoreError::RootRetrieveFailed)
            .map(Some)
    }

    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeStoreError> {
        self.nodes
            .get(hash)
            .cloned()
            .ok_or(NodeStoreError::NodeRetrieveFailed(hash.clone()))
            .map(Some)
    }
}

impl WritableNodeStore for HttpNodeStore {
    fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeStoreError> {
        HttpNodeStore::insert(self, hash, node)
    }

    fn set_root(&mut self, hash: Hash) -> Result<(), NodeStoreError> {
        HttpNodeStore::set_root(self, hash)
    }
}
