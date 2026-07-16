use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::node_store::{Hash, HttpNodeStore, InMemoryNodeStore, NodeStore, WritableNodeStore};

/// Computes the transfer set, whic is every node reachable from `root_hash` in
/// `source` that `target` does not have. When `target` already has a node,
/// its entire subtree is pruned from the walk.
pub fn compute_diff(
    source: &impl NodeStore,
    target: &impl NodeStore,
    root_hash: &Hash,
) -> HashSet<Hash> {
    let mut transfer_set = HashSet::new();

    if let Some(node) = source.get_node(root_hash) {
        match node {
            crate::node_store::Node::Folder {
                folders: _,
                files: _,
            } => {
                compute_folder_diff(source, target, root_hash, &mut transfer_set);
            }
            crate::node_store::Node::File { blobs: _ } => {
                compute_file_diff(source, target, root_hash, &mut transfer_set);
            }
        }
    }

    transfer_set
}

fn compute_folder_diff(
    source: &impl NodeStore,
    target: &impl NodeStore,
    root_hash: &Hash,
    transfer_set: &mut HashSet<Hash>,
) {
    if let Some(node) = source.get_node(root_hash) {
        match node {
            crate::node_store::Node::Folder { folders, files } => {
                // We have to both check if the folder is not on the target node store
                // and neither has been visited to only transfer it once.
                // If we don't check the transfer set, we will transfer the same folder multiple times
                // if it is referenced by multiple folders.
                if target.get_node(root_hash).is_none() && transfer_set.insert(root_hash.clone()) {
                    for folder in folders {
                        compute_folder_diff(source, target, &folder.hash, transfer_set);
                    }
                    for file in files {
                        compute_file_diff(source, target, &file.hash, transfer_set);
                    }
                }
            }
            _ => {}
        }
    }
}

fn compute_file_diff(
    source: &impl NodeStore,
    target: &impl NodeStore,
    root_hash: &Hash,
    transfer_set: &mut HashSet<Hash>,
) {
    if let Some(node) = source.get_node(root_hash) {
        match node {
            crate::node_store::Node::File { blobs: _ } => {
                // We have to both check if the file is not on the target node store
                // and neither has been visited to only transfer it once.
                // If we don't check the transfer set, we will transfer the same file multiple times
                // if it is referenced by multiple folders.
                if target.get_node(root_hash).is_none() {
                    transfer_set.insert(root_hash.clone());
                }
            }
            _ => {}
        }
    }
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
    let transfer_set = compute_diff(source, target, root_hash);

    for hash in &transfer_set {
        if let Some(node) = source.get_node(hash) {
            target.insert(hash.clone(), node.clone());
        }
    }

    target.set_root(root_hash.clone());

    transfer_set.len()
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
