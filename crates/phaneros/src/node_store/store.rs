use crate::node_store::node::{Hash, Node};

pub trait NodeStore {
    fn root_hash(&self) -> Option<&Hash>;
    fn get_node(&self, hash: &Hash) -> Option<Node>;
}

/// A node store that can also be written to. The syncer reads both sides
/// through `NodeStore` and pushes missing nodes through this.
pub trait WritableNodeStore: NodeStore {
    fn insert(&mut self, hash: Hash, node: Node);
    fn set_root(&mut self, hash: Hash);
}
