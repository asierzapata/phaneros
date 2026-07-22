use std::sync::{Arc, RwLock};

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
    syncer::{diff::compute_unidirectional_diff, sync_state::DriveSession},
};

pub mod diff;
pub mod sync_state;

#[derive(Error, Debug)]
pub enum SyncError {
    // These are logic errors, the data is gone. A caller should NOT retry these.
    #[error("source is missing blob {hash} referenced by a file node")]
    MissingSourceBlob { hash: Hash },
    #[error("source is missing node {hash} that was in the transfer set")]
    MissingSourceNode { hash: Hash },
    #[error("merge strategy not implemented")]
    MergeNotImplemented,

    // These are Transport errors: a store couldn't be reached / read / written.
    // These are kept distinct from the logic errors above because a caller may reasonably
    // retry a transport failure while giving up on missing data.
    #[error(transparent)]
    NodeRepository(#[from] NodeRepositoryError),
    #[error(transparent)]
    BlobRepository(#[from] BlobRepositoryError),
}

/// High-level sync decision derived from base/local/remote roots.
///
/// - `BootstrapPull`: no known base root yet (`B = None`), so bootstrap policy applies.
/// - `Converged`: local and remote already match.
/// - `PullRemote`: remote changed while local stayed at base.
/// - `PushLocal`: local changed while remote stayed at base.
/// - `MergeDiverged`: both local and remote diverged from base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPlan {
    RemoteBootstrapPull,
    Converged,
    RemotePull,
    LocalPush,
    Merge,
}

/// Computes the next sync action from:
///
/// - `base_root` (`B`): last converged canonical root, persisted locally.
/// - `local_root` (`L`): current local scanned root.
/// - `remote_root` (`R`): current remote store root (`None` if never set).
///
/// Policy note: when `base_root` is `None`, we always return `BootstrapPull`.
pub fn plan_sync(
    base_root: Option<&Hash>,
    local_root: &Hash,
    remote_root: Option<&Hash>,
) -> SyncPlan {
    if base_root.is_none() {
        return SyncPlan::RemoteBootstrapPull;
    }

    if remote_root == Some(local_root) {
        return SyncPlan::Converged;
    }

    let base_eq_local = base_root == Some(local_root);
    let base_eq_remote = base_root == remote_root;

    match (base_eq_local, base_eq_remote) {
        (true, false) => SyncPlan::RemotePull,
        (false, true) => SyncPlan::LocalPush,
        _ => SyncPlan::Merge,
    }
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
        self.sync_once(self.initial_root_hash.clone());
        while let Ok(updated_root_hash) = self.watcher_rx.recv() {
            println!("Syncer received updated root hash: {}", updated_root_hash);
            self.sync_once(updated_root_hash);
        }
    }

    fn sync_once(&mut self, local_root_hash: Hash) {
        let base_root = self.current_base_root();
        let remote_root = self.get_fresh_remote_hash();
        let plan = plan_sync(base_root.as_ref(), &local_root_hash, remote_root.as_ref());

        println!(
            "Syncer plan: {:?} (B={:?}, L={}, R={:?})",
            plan, base_root, local_root_hash, remote_root
        );

        match plan {
            SyncPlan::Converged => {
                self.persist_converged_root(local_root_hash);
            }
            SyncPlan::LocalPush => {
                self.reconcile_with_local_push(local_root_hash);
            }
            SyncPlan::RemoteBootstrapPull => {
                self.reconcile_with_remote_bootstrap_pull(local_root_hash);
            }
            SyncPlan::RemotePull => {
                self.reconcile_with_remote_pull(local_root_hash);
            }
            SyncPlan::Merge => {
                self.reconcile_with_merge(local_root_hash);
            }
        }
    }

    fn current_base_root(&self) -> Option<Hash> {
        self.drive_session.state.last_synced_root.clone()
    }

    fn get_fresh_remote_hash(&self) -> Option<Hash> {
        let mut remote_node_repository = self.remote_node_repository.write().unwrap();
        remote_node_repository
            .refresh_root()
            .ok()
            .and_then(|root| root)
    }

    fn persist_converged_root(&mut self, root_hash: Hash) {
        self.drive_session.set_last_synced_root(Some(root_hash));
        if let Err(err) = self.drive_session.persist() {
            panic!(
                "Syncer failed to persist sync state after successful reconcile (fail-fast): {}",
                err
            );
        }
    }

    fn reconcile_with_local_push(&mut self, root_hash: Hash) {
        let mut success = false;

        {
            let local_node_repository = self.local_node_repository.read().unwrap();
            let mut remote_node_repository = self.remote_node_repository.write().unwrap();
            let local_blob_repository = self.local_blob_repository.read().unwrap();
            let mut remote_blob_repository = self.remote_blob_repository.write().unwrap();

            let nodes_before = remote_node_repository.len();
            let blobs_before = remote_blob_repository.len();
            let result = local_push(
                &*local_node_repository,
                &mut *remote_node_repository,
                &*local_blob_repository,
                &mut *remote_blob_repository,
                &root_hash,
            );

            match result {
                Ok(0) => {
                    println!("Syncer found no nodes to sync with remote node store.");
                    success = true;
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
                    success = true;
                }
                Err(SyncError::NodeRepository(NodeRepositoryError::RootConflict { actual })) => {
                    eprintln!(
                        "Syncer hit root conflict; reconcile aborted. Remote root: {:?}",
                        actual
                    );
                }
                // On other errors the remote root was never flipped, so the remote tree is still the old,
                // consistent one; a future trigger can retry from scratch.
                Err(err) => {
                    eprintln!("Syncer failed to reconcile: {}", err);
                    return;
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

        if success {
            self.persist_converged_root(root_hash);
        }
    }

    fn reconcile_with_remote_bootstrap_pull(&mut self, _local_root_hash: Hash) {
        let Some(remote_root_hash) = self.get_fresh_remote_hash() else {
            eprintln!("Syncer cannot bootstrap pull because remote root is unavailable.");
            return;
        };

        let mut success = false;

        {
            let mut local_node_repository = self.local_node_repository.write().unwrap();
            let mut remote_node_repository = self.remote_node_repository.write().unwrap();
            let mut local_blob_repository = self.local_blob_repository.write().unwrap();
            let mut remote_blob_repository = self.remote_blob_repository.write().unwrap();

            let nodes_before = local_node_repository.len();
            let blobs_before = local_blob_repository.len();
            let result = bootstrap_pull(
                &mut *local_node_repository,
                &mut *remote_node_repository,
                &mut *local_blob_repository,
                &mut *remote_blob_repository,
                &remote_root_hash,
            );

            match result {
                Ok(0) => {
                    println!("Syncer found no nodes to bootstrap pull from remote node store.");
                    success = true;
                }
                Ok(transferred) => {
                    println!(
                        "Syncer bootstrap-pulled {} nodes and {} blobs from remote (nodes {} -> {}, blobs {} -> {}).",
                        transferred,
                        local_blob_repository.len() - blobs_before,
                        nodes_before,
                        local_node_repository.len(),
                        blobs_before,
                        local_blob_repository.len(),
                    );
                    success = true;
                }
                Err(SyncError::NodeRepository(NodeRepositoryError::RootConflict { actual })) => {
                    eprintln!(
                        "Syncer hit root conflict during bootstrap pull; reconcile aborted. Local root: {:?}",
                        actual
                    );
                }
                Err(err) => {
                    eprintln!("Syncer failed to bootstrap pull: {}", err);
                    return;
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

        if success {
            self.persist_converged_root(remote_root_hash);
        }
    }

    fn reconcile_with_remote_pull(&mut self, _local_root_hash: Hash) {
        let Some(remote_root_hash) = self.get_fresh_remote_hash() else {
            eprintln!("Syncer cannot pull because remote root is unavailable.");
            return;
        };

        let mut success = false;

        {
            let mut local_node_repository = self.local_node_repository.write().unwrap();
            let mut remote_node_repository = self.remote_node_repository.write().unwrap();
            let mut local_blob_repository = self.local_blob_repository.write().unwrap();
            let mut remote_blob_repository = self.remote_blob_repository.write().unwrap();

            let nodes_before = local_node_repository.len();
            let blobs_before = local_blob_repository.len();
            let result = remote_pull(
                &mut *local_node_repository,
                &mut *remote_node_repository,
                &mut *local_blob_repository,
                &mut *remote_blob_repository,
                &remote_root_hash,
            );

            match result {
                Ok(0) => {
                    println!("Syncer found no nodes to pull from remote node store.");
                    success = true;
                }
                Ok(transferred) => {
                    println!(
                        "Syncer pulled {} nodes and {} blobs from remote (nodes {} -> {}, blobs {} -> {}).",
                        transferred,
                        local_blob_repository.len() - blobs_before,
                        nodes_before,
                        local_node_repository.len(),
                        blobs_before,
                        local_blob_repository.len(),
                    );
                    success = true;
                }
                Err(SyncError::NodeRepository(NodeRepositoryError::RootConflict { actual })) => {
                    eprintln!(
                        "Syncer hit root conflict during remote pull; reconcile aborted. Local root: {:?}",
                        actual
                    );
                }
                Err(err) => {
                    eprintln!("Syncer failed to pull from remote: {}", err);
                    return;
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

        if success {
            self.persist_converged_root(remote_root_hash);
        }
    }

    fn reconcile_with_merge(&mut self, local_root_hash: Hash) {
        let Some(base_root_hash) = self.current_base_root() else {
            eprintln!("Syncer cannot merge because base root is unavailable.");
            return;
        };
        let Some(remote_root_hash) = self.get_fresh_remote_hash() else {
            eprintln!("Syncer cannot merge because remote root is unavailable.");
            return;
        };

        let mut success = false;
        let mut converged_root: Option<Hash> = None;

        {
            let mut local_node_repository = self.local_node_repository.write().unwrap();
            let mut remote_node_repository = self.remote_node_repository.write().unwrap();
            let mut local_blob_repository = self.local_blob_repository.write().unwrap();
            let mut remote_blob_repository = self.remote_blob_repository.write().unwrap();

            let local_nodes_before = local_node_repository.len();
            let remote_nodes_before = remote_node_repository.len();
            let local_blobs_before = local_blob_repository.len();
            let remote_blobs_before = remote_blob_repository.len();

            let result = merge(
                &mut *local_node_repository,
                &mut *remote_node_repository,
                &mut *local_blob_repository,
                &mut *remote_blob_repository,
                &base_root_hash,
                &local_root_hash,
                &remote_root_hash,
            );

            match result {
                Ok(0) => {
                    println!("Syncer merge found no nodes to reconcile.");
                    success = true;
                }
                Ok(transferred) => {
                    println!(
                        "Syncer merge reconciled {} nodes (local nodes {} -> {}, remote nodes {} -> {}, local blobs {} -> {}, remote blobs {} -> {}).",
                        transferred,
                        local_nodes_before,
                        local_node_repository.len(),
                        remote_nodes_before,
                        remote_node_repository.len(),
                        local_blobs_before,
                        local_blob_repository.len(),
                        remote_blobs_before,
                        remote_blob_repository.len(),
                    );
                    success = true;
                }
                Err(SyncError::NodeRepository(NodeRepositoryError::RootConflict { actual })) => {
                    eprintln!(
                        "Syncer hit root conflict during merge; reconcile aborted. Observed root: {:?}",
                        actual
                    );
                }
                Err(err) => {
                    eprintln!("Syncer failed to merge: {}", err);
                    return;
                }
            }

            if success {
                converged_root = match local_node_repository.root_hash() {
                    Ok(Some(root)) => Some(root.clone()),
                    Ok(None) => {
                        eprintln!("Syncer merge reported success but local root is unavailable.");
                        None
                    }
                    Err(err) => {
                        eprintln!("Syncer failed to read local root after merge: {}", err);
                        None
                    }
                };
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

        if let Some(root_hash) = converged_root {
            self.persist_converged_root(root_hash);
        }
    }
}

/// Strategies for syncing a local node/blob store with a remote node/blob store.

/// Copies every missing blob, then every missing node, from `source` into
/// `target`, then points `target`'s root at `root_hash`.
///
/// Any missing source blob/node aborts with an error BEFORE `set_root`
/// The target may be left with orphaned blobs/nodes (its harmless since GC's will pick it up)
/// but its visible tree is never broken.
///
/// Returns the number of nodes transferred.
pub fn local_push(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &mut impl WritableNodeRepository,
    source_blob_repository: &impl BlobRepository,
    target_blob_repository: &mut impl WritableBlobRepository,
    local_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set) = compute_unidirectional_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        local_root_hash,
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

    target_node_repository.set_root(local_root_hash.clone())?;

    Ok(node_transfer_set.len())
}

pub fn remote_pull(
    local_node_repository: &mut impl WritableNodeRepository,
    remote_node_repository: &mut impl NodeRepository,
    local_blob_repository: &mut impl WritableBlobRepository,
    remote_blob_repository: &mut impl BlobRepository,
    remote_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set) = compute_unidirectional_diff(
        remote_node_repository,
        local_node_repository,
        local_blob_repository,
        remote_root_hash,
    )?;

    for hash in &blob_transfer_set {
        let blob = remote_blob_repository
            .get_blob(hash)?
            .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
        local_blob_repository.insert(hash.clone(), blob)?;
    }

    for hash in &node_transfer_set {
        let node = remote_node_repository
            .get_node(hash)?
            .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
        local_node_repository.insert(hash.clone(), node)?;
    }

    local_node_repository.set_root(remote_root_hash.clone())?;

    Ok(node_transfer_set.len())
}

fn bootstrap_pull(
    local_node_repository: &mut impl WritableNodeRepository,
    remote_node_repository: &mut impl NodeRepository,
    local_blob_repository: &mut impl WritableBlobRepository,
    remote_blob_repository: &mut impl BlobRepository,
    remote_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set) = compute_unidirectional_diff(
        remote_node_repository,
        local_node_repository,
        local_blob_repository,
        remote_root_hash,
    )?;

    for hash in &blob_transfer_set {
        let blob = remote_blob_repository
            .get_blob(hash)?
            .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
        local_blob_repository.insert(hash.clone(), blob)?;
    }

    for hash in &node_transfer_set {
        let node = remote_node_repository
            .get_node(hash)?
            .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
        local_node_repository.insert(hash.clone(), node)?;
    }

    local_node_repository.set_root(remote_root_hash.clone())?;

    Ok(node_transfer_set.len())
}

fn merge(
    local_node_repository: &mut impl WritableNodeRepository,
    remote_node_repository: &mut impl WritableNodeRepository,
    local_blob_repository: &mut impl WritableBlobRepository,
    remote_blob_repository: &mut impl WritableBlobRepository,
    base_root_hash: &Hash,
    local_root_hash: &Hash,
    remote_root_hash: &Hash,
) -> Result<usize, SyncError> {
    // TODO: implement actual three-way merge strategy.
    //
    // How does a 3 way merge work? We need to consider the following cases for each node:
    // 1. Node exists in base, local, and remote: If the node is the same in all three, do nothing. If it differs in local and remote, we need to decide which one to keep. We can use the root hash to determine which one is the "latest" and keep that one.
    // 2. Node exists in base and local, but not in remote: If the node is the same in base and local, do nothing. If it differs in local, we need to keep the local version.
    // 3. Node exists in base and remote, but not in local: If the node is the same in base and remote, do nothing. If it differs in remote, we need to keep the remote version.
    // 4. Node exists in local and remote, but not in base: We need to decide which one to keep. We can use the root hash to determine which one is the "latest" and keep that one.
    // 5. Node exists only in local: Keep the local version.
    // 6. Node exists only in remote: Keep the remote version.
    // 7. Node exists only in base: This should not happen, as it would mean that the node was deleted in both local and remote, but we can ignore it.
    //
    // For each node that we decide to keep, we need to ensure that its blobs are also present in the target blob repository. If a blob is missing, we need to fetch it from the source blob repository and insert it into the target blob repository.
    //
    // Finally, we need to set the root of the target node repository to the merged root hash, which we can compute based on the nodes that we have kept.
    let _ = (
        local_node_repository,
        remote_node_repository,
        local_blob_repository,
        remote_blob_repository,
        base_root_hash,
        local_root_hash,
        remote_root_hash,
    );

    Err(SyncError::MergeNotImplemented)
}

#[cfg(test)]
mod tests;
