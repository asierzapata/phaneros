use std::collections::HashMap;

use crate::node_store::{Hash, Node, NodeStore, WritableNodeStore};

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

    pub fn insert(&mut self, hash: Hash, node: Node) {
        self.nodes.entry(hash).or_insert(node);
    }

    pub fn set_root(&mut self, hash: Hash) {
        self.root = Some(hash);
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl NodeStore for HttpNodeStore {
    fn root_hash(&self) -> Option<&Hash> {
        self.root.as_ref()
    }

    fn get_node(&self, hash: &Hash) -> Option<Node> {
        self.nodes.get(hash).cloned()
    }
}

impl WritableNodeStore for HttpNodeStore {
    fn insert(&mut self, hash: Hash, node: Node) {
        HttpNodeStore::insert(self, hash, node);
    }

    fn set_root(&mut self, hash: Hash) {
        HttpNodeStore::set_root(self, hash);
    }
}
