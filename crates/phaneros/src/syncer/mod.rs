use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use thiserror::Error;

use crate::{
    blob_store::{BlobStore, HttpBlobStore, InMemoryBlobStore, WritableBlobStore},
    node_store::{Hash, HttpNodeStore, InMemoryNodeStore, NodeStore, WritableNodeStore},
};

#[derive(Error, Debug, PartialEq)]
pub enum SyncError {
    #[error("source is missing blob {hash} referenced by a file node")]
    MissingSourceBlob { hash: Hash },
    #[error("source is missing node {hash} that was in the transfer set")]
    MissingSourceNode { hash: Hash },
}

/// Computes the transfer sets: every node reachable from `root_hash` in
/// `source` that `target` does not have, plus every blob those file nodes
/// reference that `target_blob_store` does not have. When the target already
/// has a node, its entire subtree is pruned from the walk:
/// reconcile writes blobs before nodes, so a node's presence on the target
/// implies its blobs' presence.
pub fn compute_diff(
    source_node_store: &impl NodeStore,
    target_node_store: &impl NodeStore,
    target_blob_store: &impl BlobStore,
    root_hash: &Hash,
) -> (HashSet<Hash>, HashSet<Hash>) {
    let mut node_transfer_set = HashSet::new();
    let mut blob_transfer_set = HashSet::new();

    if let Some(node) = source_node_store.get_node(root_hash) {
        match node {
            crate::node_store::Node::Folder { .. } => {
                compute_folder_diff(
                    source_node_store,
                    target_node_store,
                    target_blob_store,
                    root_hash,
                    &mut node_transfer_set,
                    &mut blob_transfer_set,
                );
            }
            crate::node_store::Node::File { .. } => {
                compute_file_diff(
                    source_node_store,
                    target_node_store,
                    target_blob_store,
                    root_hash,
                    &mut node_transfer_set,
                    &mut blob_transfer_set,
                );
            }
        }
    }

    (node_transfer_set, blob_transfer_set)
}

fn compute_folder_diff(
    source_node_store: &impl NodeStore,
    target_node_store: &impl NodeStore,
    target_blob_store: &impl BlobStore,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) {
    if let Some(node) = source_node_store.get_node(root_hash) {
        match node {
            crate::node_store::Node::Folder { folders, files } => {
                // We have to both check if the folder is not on the target node store
                // and neither has been visited to only transfer it once.
                // If we don't check the transfer set, we will transfer the same folder multiple times
                // if it is referenced by multiple folders.
                if target_node_store.get_node(root_hash).is_none()
                    && node_transfer_set.insert(root_hash.clone())
                {
                    for folder in folders {
                        compute_folder_diff(
                            source_node_store,
                            target_node_store,
                            target_blob_store,
                            &folder.hash,
                            node_transfer_set,
                            blob_transfer_set,
                        );
                    }
                    for file in files {
                        compute_file_diff(
                            source_node_store,
                            target_node_store,
                            target_blob_store,
                            &file.hash,
                            node_transfer_set,
                            blob_transfer_set,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn compute_file_diff(
    source_node_store: &impl NodeStore,
    target_node_store: &impl NodeStore,
    target_blob_store: &impl BlobStore,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) {
    if let Some(node) = source_node_store.get_node(root_hash) {
        match node {
            crate::node_store::Node::File { blobs } => {
                // Same double check as folders: skip if the target already has
                // the file, and only walk the blobs on the FIRST visit. If the
                // target has the file node, reconcile's blobs-before-nodes
                // ordering guarantees it already has the blobs too, so pruning
                // here is sound.
                if target_node_store.get_node(root_hash).is_none()
                    && node_transfer_set.insert(root_hash.clone())
                {
                    for blob_ref in blobs {
                        if !target_blob_store.contains(&blob_ref.hash) {
                            blob_transfer_set.insert(blob_ref.hash.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Copies every missing blob, then every missing node, from `source` into
/// `target`, then points `target`'s root at `root_hash`. The order is the
/// whole design: bytes land before the nodes that reference them, and the
/// root flips last, so a reader of `target` can never follow a reference to
/// something that isn't there yet.
///
/// Any missing source blob/node aborts with an error BEFORE `set_root`: the
/// target may be left with orphaned blobs/nodes (its harmless since GC's will pick it up)
/// but its visible tree is never broken.
///
/// Returns the number of nodes transferred.
pub fn reconcile_node_stores(
    source_node_store: &impl NodeStore,
    target_node_store: &mut impl WritableNodeStore,
    source_blob_store: &impl BlobStore,
    target_blob_store: &mut impl WritableBlobStore,
    root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set) = compute_diff(
        source_node_store,
        target_node_store,
        target_blob_store,
        root_hash,
    );

    for hash in &blob_transfer_set {
        let blob = source_blob_store
            .get_blob(hash)
            .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
        target_blob_store.insert(hash.clone(), blob.clone());
    }

    for hash in &node_transfer_set {
        let node = source_node_store
            .get_node(hash)
            .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
        target_node_store.insert(hash.clone(), node.clone());
    }

    target_node_store.set_root(root_hash.clone());

    Ok(node_transfer_set.len())
}

pub struct Syncer {
    watcher_rx: std::sync::mpsc::Receiver<Hash>,
    initial_root_hash: Hash,
    local_node_store: Arc<RwLock<InMemoryNodeStore>>,
    remote_node_store: Arc<RwLock<HttpNodeStore>>,
    local_blob_store: Arc<RwLock<InMemoryBlobStore>>,
    remote_blob_store: Arc<RwLock<HttpBlobStore>>,
}

impl Syncer {
    pub fn new(
        watcher_rx: std::sync::mpsc::Receiver<Hash>,
        initial_root_hash: Hash,
        local_node_store: Arc<RwLock<InMemoryNodeStore>>,
        remote_node_store: Arc<RwLock<HttpNodeStore>>,
        local_blob_store: Arc<RwLock<InMemoryBlobStore>>,
        remote_blob_store: Arc<RwLock<HttpBlobStore>>,
    ) -> Self {
        Syncer {
            watcher_rx,
            initial_root_hash,
            local_node_store,
            remote_node_store,
            local_blob_store,
            remote_blob_store,
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
        let local_node_store = self.local_node_store.read().unwrap();
        let mut remote_node_store = self.remote_node_store.write().unwrap();
        let local_blob_store = self.local_blob_store.read().unwrap();
        let mut remote_blob_store = self.remote_blob_store.write().unwrap();
        let result = reconcile_node_stores(
            &*local_node_store,
            &mut *remote_node_store,
            &*local_blob_store,
            &mut *remote_blob_store,
            &root_hash,
        );
        match result {
            // On error the remote root was never flipped, so the remote tree is still the old, consistent one
            // on the next watcher event will naturally retry this sync from scratch.
            Err(err) => eprintln!("Syncer failed to reconcile: {}", err),
            Ok(0) => println!("Syncer found no nodes to sync with remote node store."),
            Ok(transferred) => println!("Syncer transferred {} nodes to remote.", transferred),
        }
    }
}

#[cfg(test)]
mod tests;
