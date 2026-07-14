use std::collections::HashMap;

use crate::node_store::node::{Hash, Node};

pub trait NodeStore {
    fn root_hash(&self) -> Option<&Hash>;
    fn get_node(&self, hash: &Hash) -> Option<&Node>;
}

#[derive(Debug, Default)]
pub struct InMemoryNodeStore {
    root: Option<Hash>,
    nodes: HashMap<Hash, Node>,
}

impl InMemoryNodeStore {
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

impl NodeStore for InMemoryNodeStore {
    fn root_hash(&self) -> Option<&Hash> {
        self.root.as_ref()
    }

    fn get_node(&self, hash: &Hash) -> Option<&Node> {
        self.nodes.get(hash)
    }
}
