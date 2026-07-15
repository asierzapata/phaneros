use std::sync::{Arc, RwLock};

use crate::node_store::{Hash, HttpNodeStore, InMemoryNodeStore, NodeStore, WritableNodeStore};

/// Computes the transfer set, whic is every node reachable from `root_hash` in
/// `source` that `target` does not have. When `target` already has a node,
/// its entire subtree is pruned from the walk.
pub fn compute_diff(
    source: &impl NodeStore,
    target: &impl NodeStore,
    root_hash: &Hash,
) -> Vec<Hash> {
    // TODO: walk `source` from `root_hash`, prune subtrees whose hash
    // `target` already has, collect each missing node's hash exactly once.

    let _ = (source, target, root_hash);

    let mut transfer_set = Vec::new();

    source.get_node(root_hash).map(|node| match node {
        crate::node_store::Node::Folder { folders, files } => {
            for folder_entry in folders {
                let folder_hash = &folder_entry.hash;
                if target.get_node(folder_hash).is_none() {
                    transfer_set.push(folder_hash.clone());
                    transfer_set.extend(compute_diff(source, target, folder_hash));
                }
            }
            for file_hash in files {
                if target.get_node(file_hash).is_none() {
                    transfer_set.push(file_hash.clone());
                }
            }
        }
    });

    transfer_set
}

/// Copies every node in the transfer set from `source` into `target`, then
/// points `target`'s root at `root_hash`. Root is set last so a reader of
/// `target` never observes a root whose nodes aren't all present.
/// Returns the number of nodes transferred.
pub fn reconcile_node_stores(
    source: &impl NodeStore,
    target: &mut impl WritableNodeStore,
    root_hash: &Hash,
) -> usize {
    // TODO: compute_diff, insert each missing node, set_root, count.
    let _ = (source, target, root_hash);
    0
}

pub struct Syncer {
    watcher_rx: std::sync::mpsc::Receiver<Hash>,
    initial_root_hash: Hash,
    local_node_store: Arc<RwLock<InMemoryNodeStore>>,
    remote_node_store: Arc<RwLock<HttpNodeStore>>,
}

impl Syncer {
    pub fn new(
        watcher_rx: std::sync::mpsc::Receiver<Hash>,
        initial_root_hash: Hash,
        local_node_store: Arc<RwLock<InMemoryNodeStore>>,
        remote_node_store: Arc<RwLock<HttpNodeStore>>,
    ) -> Self {
        Syncer {
            watcher_rx,
            initial_root_hash,
            local_node_store,
            remote_node_store,
        }
    }

    pub fn run(&self) {
        println!(
            "Syncer started with initial root hash: {}",
            self.initial_root_hash
        );
        self.reconcile(self.initial_root_hash.clone());
        for updated_root_hash in &self.watcher_rx {
            println!("Syncer received updated root hash: {}", updated_root_hash);
            self.reconcile(updated_root_hash);
        }
    }

    fn reconcile(&self, root_hash: Hash) {
        let local = self.local_node_store.read().unwrap();
        let mut remote = self.remote_node_store.write().unwrap();
        let transferred = reconcile_node_stores(&*local, &mut *remote, &root_hash);
        if transferred == 0 {
            println!("Syncer found no nodes to sync with remote node store.");
        } else {
            println!("Syncer transferred {} nodes to remote.", transferred);
        }
    }
}

#[cfg(test)]
mod tests;
