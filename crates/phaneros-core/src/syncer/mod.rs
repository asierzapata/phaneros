use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use futures::StreamExt;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

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
    syncer::{
        diff::compute_unidirectional_diff, materialize::materialize, merge::merge,
        sync_state::DriveSession,
    },
    telemetry::DriveStatus,
    utils::filesystem_write::trash_batch_path,
    watcher::RescanHandle,
};

pub mod diff;
pub mod materialize;
pub mod merge;
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
/// Policy notes:
///
/// - when `base_root` is `None` and the remote has a root, bootstrap policy
///   applies (`RemoteBootstrapPull`).
/// - when `remote_root` is `None` there is nothing to pull, so we push local
///   (`LocalPush`). This covers both a first-run client against a brand new
///   drive and a remote that lost its visible tree.
pub fn plan_sync(
    base_root: Option<&Hash>,
    local_root: &Hash,
    remote_root: Option<&Hash>,
) -> SyncPlan {
    if base_root.is_none() {
        return if remote_root.is_none() {
            SyncPlan::LocalPush
        } else {
            SyncPlan::RemoteBootstrapPull
        };
    }

    if remote_root == Some(local_root) {
        return SyncPlan::Converged;
    }

    if remote_root.is_none() {
        return SyncPlan::LocalPush;
    }

    let base_eq_local = base_root == Some(local_root);
    let base_eq_remote = base_root == remote_root;

    match (base_eq_local, base_eq_remote) {
        (true, false) => SyncPlan::RemotePull,
        (false, true) => SyncPlan::LocalPush,
        _ => SyncPlan::Merge,
    }
}

struct SyncerStatus {
    value: DriveStatus,
    /// We coalesce multiple watcher events into a single reconcile, but we still want to know if the local store has changed since the last reconcile.
    is_dirty: bool,
}

pub struct Syncer {
    watcher_rx: tokio::sync::mpsc::Receiver<Hash>,
    initial_root_hash: Hash,
    local_node_repository: Arc<InMemoryNodeRepository>,
    remote_node_repository: Arc<HttpNodeRepository>,
    local_blob_repository: Arc<InMemoryBlobRepository>,
    remote_blob_repository: Arc<HttpBlobRepository>,
    drive_session: DriveSession,
    rescan: RescanHandle,
    vault_path: PathBuf,
    status: SyncerStatus,
    /// Published alongside `status.value` so callers outside this struct
    /// (an `EngineHandle`, and through it the daemon's IPC layer) can
    /// observe drive lifecycle status without touching `Syncer` internals.
    status_tx: watch::Sender<DriveStatus>,
    /// Holds the `ProgressTracker` of the in-flight (or most recently
    /// finished) reconcile, so a caller can poll current progress without
    /// coupling to `Syncer`'s internals.
    progress: Arc<RwLock<crate::telemetry::ProgressTracker>>,
    /// Cooperative shutdown signal, checked in the main event loop.
    cancel: CancellationToken,
    /// When set, the local store state is dumped to a text file in this
    /// directory after every reconcile (debug tooling, off by default).
    store_dump_dir: Option<std::path::PathBuf>,
    pub enable_telemetry: bool,
    pub max_concurrent_uploads: usize,
}

impl Syncer {
    pub fn new(
        watcher_rx: tokio::sync::mpsc::Receiver<Hash>,
        initial_root_hash: Hash,
        local_node_repository: Arc<InMemoryNodeRepository>,
        remote_node_repository: Arc<HttpNodeRepository>,
        local_blob_repository: Arc<InMemoryBlobRepository>,
        remote_blob_repository: Arc<HttpBlobRepository>,
        drive_session: DriveSession,
        rescan: RescanHandle,
    ) -> Self {
        let vault_path = PathBuf::from(&drive_session.state.local_path);
        let (status_tx, _status_rx) = watch::channel(DriveStatus::Idle);
        let progress = Arc::new(RwLock::new(crate::telemetry::ProgressTracker::new(
            &drive_session.state.drive_id,
        )));

        Syncer {
            watcher_rx,
            initial_root_hash,
            local_node_repository,
            remote_node_repository,
            local_blob_repository,
            remote_blob_repository,
            drive_session,
            rescan,
            vault_path,
            status: SyncerStatus {
                value: DriveStatus::Idle,
                is_dirty: false,
            },
            status_tx,
            progress,
            cancel: CancellationToken::new(),
            store_dump_dir: None,
            enable_telemetry: false,
            max_concurrent_uploads: 10,
        }
    }

    pub fn with_telemetry(mut self, enabled: bool) -> Self {
        self.enable_telemetry = enabled;
        self
    }

    pub fn with_max_concurrent_uploads(mut self, max: usize) -> Self {
        self.max_concurrent_uploads = max;
        self
    }

    /// Overrides the status channel, so a caller (e.g. `EngineHandle`) can
    /// hold on to the `Receiver` half before this `Syncer` moves into its
    /// background task.
    pub fn with_status_channel(mut self, status_tx: watch::Sender<DriveStatus>) -> Self {
        let _ = status_tx.send(self.status.value.clone());
        self.status_tx = status_tx;
        self
    }

    /// Overrides the cancellation token, so a caller can request cooperative
    /// shutdown of the run loop from outside.
    pub fn with_cancellation(mut self, cancel: CancellationToken) -> Self {
        self.cancel = cancel;
        self
    }

    /// Overrides the shared progress handle, so a caller can keep reading
    /// current sync progress via the same `Arc` after this `Syncer` moves
    /// into its background task.
    pub fn with_progress_handle(
        mut self,
        progress: Arc<RwLock<crate::telemetry::ProgressTracker>>,
    ) -> Self {
        self.progress = progress;
        self
    }

    /// Enables dumping the local store state to `dir/local_store_dump.txt`
    /// after every reconcile.
    pub fn with_store_dump(mut self, dir: std::path::PathBuf) -> Self {
        self.store_dump_dir = Some(dir);
        self
    }

    pub async fn run(&mut self) {
        tracing::info!(root_hash = %self.initial_root_hash, "syncer started");
        self.sync_once(self.initial_root_hash.clone()).await;

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    tracing::info!("syncer received cancellation, stopping");
                    break;
                }
                maybe_hash = self.watcher_rx.recv() => {
                    match maybe_hash {
                        Some(updated_root_hash) => {
                            tracing::info!(root_hash = %updated_root_hash, "syncer received updated root hash");
                            self.sync_once(updated_root_hash).await;
                        }
                        None => break,
                    }
                }
            }
        }

        self.set_status(DriveStatus::Stopped);
    }

    fn set_status(&mut self, status: DriveStatus) {
        self.status.value = status.clone();
        let _ = self.status_tx.send(status);
    }

    async fn sync_once(&mut self, local_root_hash: Hash) {
        if !matches!(self.status.value, DriveStatus::Idle) {
            tracing::debug!("syncer is already busy; skipping this event");
            self.status.is_dirty = true;
            return;
        }

        let tracker = crate::telemetry::ProgressTracker::new(&self.drive_session.state.drive_id);
        self.remote_blob_repository.set_tracker(tracker.clone());
        if let Ok(mut current) = self.progress.write() {
            *current = tracker.clone();
        }

        tracker.set_phase(crate::telemetry::SyncPhase::Diffing);
        self.set_status(DriveStatus::Reconciling);
        self.status.is_dirty = false;

        let base_root = self.current_base_root();
        let remote_root = self.get_fresh_remote_hash().await;
        let plan = plan_sync(base_root.as_ref(), &local_root_hash, remote_root.as_ref());

        tracing::info!(?plan, ?base_root, local_root = %local_root_hash, ?remote_root, "syncer plan computed");

        if plan == SyncPlan::LocalPush {
            tracker.set_phase(crate::telemetry::SyncPhase::UploadingPayloads);
        }

        let converged_root = match plan {
            SyncPlan::Converged => Some(local_root_hash.clone()),
            SyncPlan::LocalPush => self.reconcile_with_local_push(&local_root_hash).await,
            SyncPlan::RemoteBootstrapPull => self.reconcile_with_remote_bootstrap_pull().await,
            SyncPlan::RemotePull => self.reconcile_with_remote_pull().await,
            SyncPlan::Merge => self.reconcile_with_merge(&local_root_hash).await,
        };

        if converged_root.is_some() {
            tracker.set_phase(crate::telemetry::SyncPhase::Materializing);
        }

        let next_root = self.persist_and_materialize(&local_root_hash, converged_root.clone()).await;

        if converged_root.is_some() {
            tracker.set_phase(crate::telemetry::SyncPhase::Converged);
        } else {
            tracker.set_phase(crate::telemetry::SyncPhase::Failed);
        }

        let summary = tracker.finalize();
        if self.enable_telemetry {
            if let Ok(db) = crate::telemetry::MetricsDatabase::connect_default().await {
                let _ = db.insert_summary(&summary).await;
            }
        }

        tracing::info!(
            duration = ?summary.phase_timings.total_duration,
            compression_pct = summary.compression.compression_ratio(),
            wire_bytes = summary.transfer.wire_bytes_sent,
            dedup_bytes_saved = summary.transfer.deduplicated_bytes_saved,
            avg_upload_rate_bps = summary.avg_upload_rate_bps,
            "sync completed"
        );

        self.set_status(DriveStatus::Idle);
        self.status.is_dirty = false;

        if let Some(next_root) = next_root {
            tracing::info!("vault changed underneath the syncer; syncing again");
            // Async fns can't recurse directly, so this needs Box::pin.
            Box::pin(self.sync_once(next_root)).await;
        }
    }

    async fn persist_and_materialize(
        &mut self,
        local_root_hash: &Hash,
        converged_root: Option<Hash>,
    ) -> Option<Hash> {
        let converged_root = converged_root?;

        let mut next_root = None;

        if converged_root != *local_root_hash {
            self.set_status(DriveStatus::Materializing);

            if !self.materialize_converged_root(local_root_hash, &converged_root).await {
                return None;
            }

            // Hashes queued while we were writing describe either the vault
            // before we touched it or our own writes; a scan now supersedes
            // both, and picks up anything the user changed meanwhile.
            while self.watcher_rx.try_recv().is_ok() {}

            match self.rescan.rescan() {
                Ok(fresh_root) if fresh_root != converged_root => next_root = Some(fresh_root),
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!("Syncer failed to rescan after materializing: {}", err);
                }
            }
        }

        self.persist_converged_root(&converged_root);

        next_root
    }

    async fn materialize_converged_root(&self, from_root: &Hash, to_root: &Hash) -> bool {
        let local_node_repository = self.local_node_repository.clone();
        let local_blob_repository = self.local_blob_repository.clone();
        let trash_batch = trash_batch_path(&self.vault_path);

        match materialize(
            &*local_node_repository,
            &*local_blob_repository,
            &self.vault_path,
            &trash_batch,
            Some(from_root),
            to_root,
        ).await {
            Ok(stats) if stats.touched_nothing() => {
                tracing::info!("Syncer found the vault already matching {}.", to_root);
                true
            }
            Ok(stats) => {
                tracing::info!(
                    "Syncer materialized {} into the vault ({} files written, {} folders created, {} entries trashed).",
                    to_root, stats.files_written, stats.folders_created, stats.entries_trashed,
                );
                true
            }
            Err(err) => {
                tracing::warn!(
                    "Syncer failed to materialize {} into the vault: {}",
                    to_root, err
                );
                false
            }
        }
    }

    fn current_base_root(&self) -> Option<Hash> {
        self.drive_session.state.last_synced_root.clone()
    }

    async fn get_fresh_remote_hash(&self) -> Option<Hash> {
        self.remote_node_repository
            .refresh_root().await
            .ok()
            .flatten()
    }

    fn persist_converged_root(&mut self, root_hash: &Hash) {
        self.drive_session
            .set_last_synced_root(Some(root_hash.clone()));
        if let Err(err) = self.drive_session.persist() {
            panic!(
                "Syncer failed to persist sync state after successful reconcile (fail-fast): {}",
                err
            );
        }
    }

    async fn reconcile_with_local_push(&mut self, root_hash: &Hash) -> Option<Hash> {
        let mut success = false;

        {
            let local_node_repository = self.local_node_repository.clone();
            let remote_node_repository = self.remote_node_repository.clone();
            let local_blob_repository = self.local_blob_repository.clone();
            let remote_blob_repository = self.remote_blob_repository.clone();

            let nodes_before = remote_node_repository.len();
            let blobs_before = remote_blob_repository.len();
            let result = local_push(
                &*local_node_repository,
                &*remote_node_repository,
                &*local_blob_repository,
                &*remote_blob_repository,
                &root_hash,
            ).await;

            match result {
                Ok(0) => {
                    tracing::info!("Syncer found no nodes to sync with remote node store.");
                    success = true;
                }
                Ok(transferred) => {
                    tracing::info!(
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
                    tracing::warn!(
                        "Syncer hit root conflict; reconcile aborted. Remote root: {:?}",
                        actual
                    );
                }
                // On other errors the remote root was never flipped, so the remote tree is still the old,
                // consistent one; a future trigger can retry from scratch.
                Err(err) => {
                    tracing::warn!("Syncer failed to reconcile: {}", err);
                    return None;
                }
            }

            if let Some(dump_dir) = &self.store_dump_dir {
                if let Err(err) = crate::utils::store_dump::dump_store(
                    &*local_node_repository,
                    &*local_blob_repository,
                    &dump_dir.join("local_store_dump.txt"),
                ).await {
                    tracing::warn!("Syncer failed to dump local store state: {}", err);
                }
            }
        }

        success.then(|| root_hash.clone())
    }

    async fn reconcile_with_remote_bootstrap_pull(&mut self) -> Option<Hash> {
        let Some(remote_root_hash) = self.get_fresh_remote_hash().await else {
            tracing::warn!("Syncer cannot bootstrap pull because remote root is unavailable.");
            return None;
        };

        let mut success = false;

        {
            let local_node_repository = self.local_node_repository.clone();
            let remote_node_repository = self.remote_node_repository.clone();
            let local_blob_repository = self.local_blob_repository.clone();
            let remote_blob_repository = self.remote_blob_repository.clone();

            let nodes_before = local_node_repository.len();
            let blobs_before = local_blob_repository.len();
            let result = bootstrap_pull(
                &*local_node_repository,
                &*remote_node_repository,
                &*local_blob_repository,
                &*remote_blob_repository,
                &remote_root_hash,
            ).await;

            match result {
                Ok(0) => {
                    tracing::info!("Syncer found no nodes to bootstrap pull from remote node store.");
                    success = true;
                }
                Ok(transferred) => {
                    tracing::info!(
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
                    tracing::warn!(
                        "Syncer hit root conflict during bootstrap pull; reconcile aborted. Local root: {:?}",
                        actual
                    );
                }
                Err(err) => {
                    tracing::warn!("Syncer failed to bootstrap pull: {}", err);
                    return None;
                }
            }

            if let Some(dump_dir) = &self.store_dump_dir {
                if let Err(err) = crate::utils::store_dump::dump_store(
                    &*local_node_repository,
                    &*local_blob_repository,
                    &dump_dir.join("local_store_dump.txt"),
                ).await {
                    tracing::warn!("Syncer failed to dump local store state: {}", err);
                }
            }
        }

        success.then_some(remote_root_hash)
    }

    async fn reconcile_with_remote_pull(&mut self) -> Option<Hash> {
        let Some(remote_root_hash) = self.get_fresh_remote_hash().await else {
            tracing::warn!("Syncer cannot pull because remote root is unavailable.");
            return None;
        };

        let mut success = false;

        {
            let local_node_repository = self.local_node_repository.clone();
            let remote_node_repository = self.remote_node_repository.clone();
            let local_blob_repository = self.local_blob_repository.clone();
            let remote_blob_repository = self.remote_blob_repository.clone();

            let nodes_before = local_node_repository.len();
            let blobs_before = local_blob_repository.len();
            let result = remote_pull(
                &*local_node_repository,
                &*remote_node_repository,
                &*local_blob_repository,
                &*remote_blob_repository,
                &remote_root_hash,
            ).await;

            match result {
                Ok(0) => {
                    tracing::info!("Syncer found no nodes to pull from remote node store.");
                    success = true;
                }
                Ok(transferred) => {
                    tracing::info!(
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
                    tracing::warn!(
                        "Syncer hit root conflict during remote pull; reconcile aborted. Local root: {:?}",
                        actual
                    );
                }
                Err(err) => {
                    tracing::warn!("Syncer failed to pull from remote: {}", err);
                    return None;
                }
            }

            if let Some(dump_dir) = &self.store_dump_dir {
                if let Err(err) = crate::utils::store_dump::dump_store(
                    &*local_node_repository,
                    &*local_blob_repository,
                    &dump_dir.join("local_store_dump.txt"),
                ).await {
                    tracing::warn!("Syncer failed to dump local store state: {}", err);
                }
            }
        }

        success.then_some(remote_root_hash)
    }

    async fn reconcile_with_merge(&mut self, local_root_hash: &Hash) -> Option<Hash> {
        let Some(base_root_hash) = self.current_base_root() else {
            tracing::warn!("Syncer cannot merge because base root is unavailable.");
            return None;
        };
        let Some(remote_root_hash) = self.get_fresh_remote_hash().await else {
            tracing::warn!("Syncer cannot merge because remote root is unavailable.");
            return None;
        };

        let mut success = false;
        let mut converged_root: Option<Hash> = None;

        {
            let local_node_repository = self.local_node_repository.clone();
            let remote_node_repository = self.remote_node_repository.clone();
            let local_blob_repository = self.local_blob_repository.clone();
            let remote_blob_repository = self.remote_blob_repository.clone();

            let local_nodes_before = local_node_repository.len();
            let remote_nodes_before = remote_node_repository.len();
            let local_blobs_before = local_blob_repository.len();
            let remote_blobs_before = remote_blob_repository.len();

            let result = merge(
                &*local_node_repository,
                &*remote_node_repository,
                &*local_blob_repository,
                &*remote_blob_repository,
                &base_root_hash,
                &local_root_hash,
                &remote_root_hash,
            ).await;

            match result {
                Ok(0) => {
                    tracing::info!("Syncer merge found no nodes to reconcile.");
                    success = true;
                }
                Ok(transferred) => {
                    tracing::info!(
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
                    tracing::warn!(
                        "Syncer hit root conflict during merge; reconcile aborted. Observed root: {:?}",
                        actual
                    );
                }
                Err(err) => {
                    tracing::warn!("Syncer failed to merge: {}", err);
                    return None;
                }
            }

            if success {
                converged_root = match local_node_repository.root_hash().await {
                    Ok(Some(root)) => Some(root.clone()),
                    Ok(None) => {
                        tracing::warn!("Syncer merge reported success but local root is unavailable.");
                        None
                    }
                    Err(err) => {
                        tracing::warn!("Syncer failed to read local root after merge: {}", err);
                        None
                    }
                };
            }

            if let Some(dump_dir) = &self.store_dump_dir {
                if let Err(err) = crate::utils::store_dump::dump_store(
                    &*local_node_repository,
                    &*local_blob_repository,
                    &dump_dir.join("local_store_dump.txt"),
                ).await {
                    tracing::warn!("Syncer failed to dump local store state: {}", err);
                }
            }
        }

        converged_root
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
pub async fn local_push(
    source_node_repository: &(impl NodeRepository + Send + Sync),
    target_node_repository: &(impl WritableNodeRepository + Send + Sync),
    source_blob_repository: &(impl BlobRepository + Send + Sync),
    target_blob_repository: &(impl WritableBlobRepository + Send + Sync),
    local_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set, mut node_cache) = compute_unidirectional_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        local_root_hash,
    ).await?;

    // Concurrent blob transfers
    futures::stream::iter(blob_transfer_set.into_iter())
        .map(|hash| async move {
            let blob = source_blob_repository
                .get_blob(&hash).await?
                .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
            target_blob_repository.insert(hash.clone(), blob).await?;
            Ok::<(), SyncError>(())
        })
        .buffer_unordered(10) // max_concurrent
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, SyncError>>()?;

    for hash in &node_transfer_set {
        let node = match node_cache.remove(hash) {
            Some(n) => n,
            None => source_node_repository.get_node(hash).await?.ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?
        };
        target_node_repository.insert(hash.clone(), node).await?;
    }

    target_node_repository.set_root(local_root_hash.clone()).await?;

    Ok(node_transfer_set.len())
}

pub async fn remote_pull(
    local_node_repository: &(impl WritableNodeRepository + Send + Sync),
    remote_node_repository: &(impl NodeRepository + Send + Sync),
    local_blob_repository: &(impl WritableBlobRepository + Send + Sync),
    remote_blob_repository: &(impl BlobRepository + Send + Sync),
    remote_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set, mut node_cache) = compute_unidirectional_diff(
        remote_node_repository,
        local_node_repository,
        local_blob_repository,
        remote_root_hash,
    ).await?;

    futures::stream::iter(blob_transfer_set.into_iter())
        .map(|hash| async move {
            let blob = remote_blob_repository
                .get_blob(&hash).await?
                .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
            local_blob_repository.insert(hash.clone(), blob).await?;
            Ok::<(), SyncError>(())
        })
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, SyncError>>()?;

    for hash in &node_transfer_set {
        let node = match node_cache.remove(hash) {
            Some(n) => n,
            None => remote_node_repository.get_node(hash).await?.ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?
        };
        local_node_repository.insert(hash.clone(), node).await?;
    }

    local_node_repository.set_root(remote_root_hash.clone()).await?;

    Ok(node_transfer_set.len())
}

async fn bootstrap_pull(
    local_node_repository: &(impl WritableNodeRepository + Send + Sync),
    remote_node_repository: &(impl NodeRepository + Send + Sync),
    local_blob_repository: &(impl WritableBlobRepository + Send + Sync),
    remote_blob_repository: &(impl BlobRepository + Send + Sync),
    remote_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let (node_transfer_set, blob_transfer_set, mut node_cache) = compute_unidirectional_diff(
        remote_node_repository,
        local_node_repository,
        local_blob_repository,
        remote_root_hash,
    ).await?;

    futures::stream::iter(blob_transfer_set.into_iter())
        .map(|hash| async move {
            let blob = remote_blob_repository
                .get_blob(&hash).await?
                .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
            local_blob_repository.insert(hash.clone(), blob).await?;
            Ok::<(), SyncError>(())
        })
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, SyncError>>()?;

    for hash in &node_transfer_set {
        let node = match node_cache.remove(hash) {
            Some(n) => n,
            None => remote_node_repository.get_node(hash).await?.ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?
        };
        local_node_repository.insert(hash.clone(), node).await?;
    }

    local_node_repository.set_root(remote_root_hash.clone()).await?;

    Ok(node_transfer_set.len())
}

#[cfg(test)]
mod tests;
