use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use thiserror::Error;

use crate::{
    blob_repository::{
        BlobRepository, BlobRepositoryError, HttpBlobRepository, InMemoryBlobRepository,
        WritableBlobRepository,
    },
    node_repository::{
        Hash, HttpNodeRepository, InMemoryNodeRepository, NodeRepository, NodeRepositoryError,
        WritableNodeRepository,
    },
    syncer::sync_state::DriveSession,
};

pub mod sync_state;

#[derive(Error, Debug)]
pub enum SyncError {
    // These are logic errors, the data is gone. A caller should NOT retry these.
    #[error("source is missing blob {hash} referenced by a file node")]
    MissingSourceBlob { hash: Hash },
    #[error("source is missing node {hash} that was in the transfer set")]
    MissingSourceNode { hash: Hash },

    // These are Transport errors: a store couldn't be reached / read / written.
    // These are kept distinct from the logic errors above because a caller may reasonably
    // retry a transport failure while giving up on missing data.
    #[error(transparent)]
    NodeRepository(#[from] NodeRepositoryError),
    #[error(transparent)]
    BlobRepository(#[from] BlobRepositoryError),
}

/// Computes the transfer sets: every node reachable from `root_hash` in
/// `source` that `target` does not have, plus every blob those file nodes
/// reference that `target_blob_repository` does not have. When the target already
/// has a node, its entire subtree is pruned from the walk:
/// reconcile writes blobs before nodes, so a node's presence on the target
/// implies its blobs' presence.
pub fn compute_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
) -> Result<(HashSet<Hash>, HashSet<Hash>), SyncError> {
    let mut node_transfer_set = HashSet::new();
    let mut blob_transfer_set = HashSet::new();

    if let Some(node) = source_node_repository.get_node(root_hash)? {
        match node {
            crate::node_repository::Node::Folder { .. } => {
                compute_folder_diff(
                    source_node_repository,
                    target_node_repository,
                    target_blob_repository,
                    root_hash,
                    &mut node_transfer_set,
                    &mut blob_transfer_set,
                )?;
            }
            crate::node_repository::Node::File { .. } => {
                compute_file_diff(
                    source_node_repository,
                    target_node_repository,
                    target_blob_repository,
                    root_hash,
                    &mut node_transfer_set,
                    &mut blob_transfer_set,
                )?;
            }
        }
    }

    Ok((node_transfer_set, blob_transfer_set))
}

fn compute_folder_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) -> Result<(), SyncError> {
    let Some(crate::node_repository::Node::Folder { folders, files }) =
        source_node_repository.get_node(root_hash)?
    else {
        return Ok(());
    };

    // We have to both check if the folder is not on the target node store
    // and neither has been visited to only transfer it once.
    // If we don't check the transfer set, we will transfer the same folder multiple times
    // if it is referenced by multiple folders.
    if target_node_repository.get_node(root_hash)?.is_none()
        && node_transfer_set.insert(root_hash.clone())
    {
        for folder in folders {
            compute_folder_diff(
                source_node_repository,
                target_node_repository,
                target_blob_repository,
                &folder.hash,
                node_transfer_set,
                blob_transfer_set,
            )?;
        }
        for file in files {
            compute_file_diff(
                source_node_repository,
                target_node_repository,
                target_blob_repository,
                &file.hash,
                node_transfer_set,
                blob_transfer_set,
            )?;
        }
    }

    Ok(())
}

fn compute_file_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) -> Result<(), SyncError> {
    let Some(crate::node_repository::Node::File { blobs }) =
        source_node_repository.get_node(root_hash)?
    else {
        return Ok(());
    };

    if target_node_repository.get_node(root_hash)?.is_none()
        && node_transfer_set.insert(root_hash.clone())
    {
        for blob_ref in blobs {
            if !target_blob_repository.contains(&blob_ref.hash)? {
                blob_transfer_set.insert(blob_ref.hash.clone());
            }
        }
    }

    Ok(())
}

/// Copies every missing blob, then every missing node, from `source` into
/// `target`, then points `target`'s root at `root_hash`.
///
/// Any missing source blob/node aborts with an error BEFORE `set_root`
/// The target may be left with orphaned blobs/nodes (its harmless since GC's will pick it up)
/// but its visible tree is never broken.
///
/// Returns the number of nodes transferred.
pub fn reconcile_node_repositorys(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &mut impl WritableNodeRepository,
    source_blob_repository: &impl BlobRepository,
    target_blob_repository: &mut impl WritableBlobRepository,
    root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set) = compute_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        root_hash,
    )?;

    for hash in &blob_transfer_set {
        let blob = source_blob_repository
            .get_blob(hash)?
            .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
        target_blob_repository.insert(hash.clone(), blob)?;
    }

    for hash in &node_transfer_set {
        let node = source_node_repository
            .get_node(hash)?
            .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
        target_node_repository.insert(hash.clone(), node)?;
    }

    target_node_repository.set_root(root_hash.clone())?;

    Ok(node_transfer_set.len())
}

pub struct Syncer {
    watcher_rx: std::sync::mpsc::Receiver<Hash>,
    initial_root_hash: Hash,
    local_node_repository: Arc<RwLock<InMemoryNodeRepository>>,
    remote_node_repository: Arc<RwLock<HttpNodeRepository>>,
    local_blob_repository: Arc<RwLock<InMemoryBlobRepository>>,
    remote_blob_repository: Arc<RwLock<HttpBlobRepository>>,
    drive_session: DriveSession,
    /// When set, the local store state is dumped to a text file in this
    /// directory after every reconcile (debug tooling, off by default).
    store_dump_dir: Option<std::path::PathBuf>,
}

impl Syncer {
    pub fn new(
        watcher_rx: std::sync::mpsc::Receiver<Hash>,
        initial_root_hash: Hash,
        local_node_repository: Arc<RwLock<InMemoryNodeRepository>>,
        remote_node_repository: Arc<RwLock<HttpNodeRepository>>,
        local_blob_repository: Arc<RwLock<InMemoryBlobRepository>>,
        remote_blob_repository: Arc<RwLock<HttpBlobRepository>>,
        drive_session: DriveSession,
    ) -> Self {
        Syncer {
            watcher_rx,
            initial_root_hash,
            local_node_repository,
            remote_node_repository,
            local_blob_repository,
            remote_blob_repository,
            drive_session,
            store_dump_dir: None,
        }
    }

    /// Enables dumping the local store state to `dir/local_store_dump.txt`
    /// after every reconcile.
    pub fn with_store_dump(mut self, dir: std::path::PathBuf) -> Self {
        self.store_dump_dir = Some(dir);
        self
    }

    pub fn run(&mut self) {
        println!(
            "Syncer started with initial root hash: {}",
            self.initial_root_hash
        );
        self.reconcile(self.initial_root_hash.clone());
        while let Ok(updated_root_hash) = self.watcher_rx.recv() {
            println!("Syncer received updated root hash: {}", updated_root_hash);
            self.reconcile(updated_root_hash);
        }
    }

    fn reconcile(&mut self, root_hash: Hash) {
        let local_node_repository = self.local_node_repository.read().unwrap();
        let mut remote_node_repository = self.remote_node_repository.write().unwrap();
        let local_blob_repository = self.local_blob_repository.read().unwrap();
        let mut remote_blob_repository = self.remote_blob_repository.write().unwrap();
        let nodes_before = remote_node_repository.len();
        let blobs_before = remote_blob_repository.len();
        let result = reconcile_node_repositorys(
            &*local_node_repository,
            &mut *remote_node_repository,
            &*local_blob_repository,
            &mut *remote_blob_repository,
            &root_hash,
        );
        match result {
            // On error the remote root was never flipped, so the remote tree is still the old, consistent one
            // on the next watcher event will naturally retry this sync from scratch.
            Err(err) => eprintln!("Syncer failed to reconcile: {}", err),
            Ok(0) => {
                println!("Syncer found no nodes to sync with remote node store.");
                self.drive_session
                    .set_last_synced_root(Some(root_hash.clone()));
                if let Err(err) = self.drive_session.persist() {
                    panic!(
                        "Syncer failed to persist sync state after successful reconcile (fail-fast): {}",
                        err
                    );
                }
            }
            Ok(transferred) => {
                println!(
                    "Syncer transferred {} nodes and {} blobs to remote (nodes {} -> {}, blobs {} -> {}).",
                    transferred,
                    remote_blob_repository.len() - blobs_before,
                    nodes_before,
                    remote_node_repository.len(),
                    blobs_before,
                    remote_blob_repository.len(),
                );
                self.drive_session
                    .set_last_synced_root(Some(root_hash.clone()));
                if let Err(err) = self.drive_session.persist() {
                    panic!(
                        "Syncer failed to persist sync state after successful reconcile (fail-fast): {}",
                        err
                    );
                }
            }
        }
        if let Some(dump_dir) = &self.store_dump_dir {
            if let Err(err) = crate::utils::store_dump::dump_store(
                &*local_node_repository,
                &*local_blob_repository,
                &dump_dir.join("local_store_dump.txt"),
            ) {
                eprintln!("Syncer failed to dump local store state: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests;
