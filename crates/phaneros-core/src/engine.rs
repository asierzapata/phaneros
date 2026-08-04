use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::blob_repository::HttpBlobRepository;
use crate::node_repository::{Hash, HttpNodeRepository};
use crate::syncer::Syncer;
use crate::syncer::sync_state::DriveSession;
use crate::telemetry::{DriveStatus, ProgressTracker, SyncProgressEvent};
use crate::watcher::{RescanHandle, WatchHandle, Watcher, spawn_remote_listener};
use tracing::Instrument;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub path: PathBuf,
    pub store_url: String,
    pub drive_id: String,
    pub token: String,
    pub dump_store: Option<PathBuf>,
    pub enable_telemetry: bool,
    pub max_concurrent_uploads: usize,
}

impl EngineConfig {
    pub fn new(
        path: PathBuf,
        store_url: String,
        drive_id: String,
        token: String,
        dump_store: Option<PathBuf>,
    ) -> Self {
        Self {
            path,
            store_url,
            drive_id,
            token,
            dump_store,
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
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Failed to initialize sync state drive session: {0}")]
    DriveSessionFailed(String),
    #[error("Failed to start directory watcher: {0}")]
    WatcherFailed(String),
    #[error("Failed to trigger a manual sync: {0}")]
    TriggerSyncFailed(String),
}

/// A handle to a running sync engine, returned by [`SyncEngine::spawn`].
///
/// Lets a supervisor (the daemon, or an embedding app/GUI) observe and
/// control an engine without owning the sync loop itself: read current
/// [`DriveStatus`]/[`SyncProgressEvent`], force an out-of-band sync, or
/// request a graceful shutdown.
pub struct EngineHandle {
    drive_id: String,
    status_rx: watch::Receiver<DriveStatus>,
    progress: Arc<RwLock<ProgressTracker>>,
    trigger_tx: tokio::sync::mpsc::Sender<Hash>,
    rescan: RescanHandle,
    cancel: CancellationToken,
    join: tokio::task::JoinHandle<()>,
    /// Aborted on shutdown: it otherwise polls for remote events forever.
    remote_listener_join: tokio::task::JoinHandle<()>,
}

impl EngineHandle {
    pub fn drive_id(&self) -> &str {
        &self.drive_id
    }

    /// Current lifecycle status of the drive (does not block).
    pub fn status(&self) -> DriveStatus {
        self.status_rx.borrow().clone()
    }

    /// A clone of the status-change receiver, for a supervisor to watch
    /// transitions (e.g. to forward them as IPC notifications) without
    /// polling.
    pub fn subscribe_status(&self) -> watch::Receiver<DriveStatus> {
        self.status_rx.clone()
    }

    /// Current progress of the in-flight (or most recently finished) sync.
    pub fn progress(&self) -> SyncProgressEvent {
        self.progress
            .read()
            .expect("progress tracker lock poisoned")
            .current_event()
    }

    /// Forces an immediate reconcile pass, as if the watcher had just fired.
    pub async fn trigger_sync(&self) -> Result<(), EngineError> {
        let rescan = self.rescan.clone();
        let root_hash = tokio::task::spawn_blocking(move || rescan.rescan())
            .await
            .map_err(|e| EngineError::TriggerSyncFailed(e.to_string()))?
            .map_err(|e| EngineError::TriggerSyncFailed(e.to_string()))?;

        self.trigger_tx
            .send(root_hash)
            .await
            .map_err(|e| EngineError::TriggerSyncFailed(e.to_string()))
    }

    /// Requests cooperative shutdown and waits for the sync loop to exit.
    ///
    /// Also stops the native filesystem-watcher thread and aborts the
    /// remote-event listener task; without this, both keep running forever
    /// (the watcher thread blocks on its own channel and never observes it
    /// as closed, and the listener just keeps polling), which would prevent
    /// a daemon process hosting this drive from ever fully exiting.
    pub async fn shutdown(self) {
        self.rescan.stop();
        self.remote_listener_join.abort();
        self.cancel.cancel();
        let _ = self.join.await;
    }
}

/// The core sync engine for Phaneros.
///
/// Can be instantiated directly by the daemon (`phanerosd`), embedded in a GUI desktop app (e.g. Tauri/Electron),
/// or invoked directly by the CLI client (`phaneros`).
pub struct SyncEngine {
    config: EngineConfig,
}

impl SyncEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Performs setup (drive session, filesystem watcher, remote repository
    /// handles) and spawns the sync loop as a background task, returning an
    /// [`EngineHandle`] to observe and control it. Setup failures are
    /// returned directly; once spawned, the loop keeps running until
    /// [`EngineHandle::shutdown`] is called.
    pub async fn spawn(self) -> Result<EngineHandle, EngineError> {
        let span = tracing::info_span!("drive", drive_id = %self.config.drive_id);
        self.spawn_inner().instrument(span).await
    }

    async fn spawn_inner(self) -> Result<EngineHandle, EngineError> {
        let drive_session = DriveSession::open(&self.config.drive_id, &self.config.path)
            .map_err(|e| EngineError::DriveSessionFailed(e.to_string()))?;

        let watcher = Watcher::new(self.config.path.to_string_lossy().into_owned());

        tracing::info!("watcher started, waiting for changes");

        let watch_handle = watcher
            .watch()
            .map_err(|e| EngineError::WatcherFailed(e.to_string()))?;

        let WatchHandle {
            root_hashes: watcher_root_hashes,
            initial_root_hash,
            node_repository: local_node_repository,
            blob_repository: local_blob_repository,
            rescan,
        } = watch_handle;

        let (sync_trigger_tx, sync_trigger_rx) = tokio::sync::mpsc::channel(100);

        let watcher_forward_tx = sync_trigger_tx.clone();
        tokio::task::spawn_blocking(move || {
            for root_hash in watcher_root_hashes {
                if watcher_forward_tx.blocking_send(root_hash).is_err() {
                    break;
                }
            }
        });

        let remote_node_repository = Arc::new(
            HttpNodeRepository::new(
                self.config.store_url.clone(),
                self.config.drive_id.clone(),
                self.config.token.clone(),
            ).await
        );

        let remote_rescan = rescan.clone();
        let remote_trigger_tx = sync_trigger_tx.clone();
        let drive_id_for_listener = self.config.drive_id.clone();
        let remote_listener_join = spawn_remote_listener(
            self.config.store_url.clone(),
            self.config.drive_id.clone(),
            self.config.token.clone(),
            move |_event| match remote_rescan.rescan() {
                Ok(root_hash) => {
                    let _ = remote_trigger_tx.try_send(root_hash);
                }
                Err(err) => {
                    tracing::warn!(drive_id = %drive_id_for_listener, "failed to rescan after remote root-changed event: {}", err);
                }
            },
        );

        let remote_blob_repository = Arc::new(
            HttpBlobRepository::new(
                self.config.store_url.clone(),
                self.config.drive_id.clone(),
                self.config.token.clone(),
            )
        );

        let (status_tx, status_rx) = watch::channel(DriveStatus::Idle);
        let progress = Arc::new(RwLock::new(ProgressTracker::new(&self.config.drive_id)));
        let cancel = CancellationToken::new();

        let mut syncer = Syncer::new(
            sync_trigger_rx,
            initial_root_hash,
            local_node_repository,
            remote_node_repository,
            local_blob_repository,
            remote_blob_repository,
            drive_session,
            rescan.clone(),
        )
        .with_telemetry(self.config.enable_telemetry)
        .with_max_concurrent_uploads(self.config.max_concurrent_uploads)
        .with_status_channel(status_tx)
        .with_progress_handle(progress.clone())
        .with_cancellation(cancel.clone());

        drop(sync_trigger_tx.clone());

        if let Some(ref dump_dir) = self.config.dump_store {
            tracing::info!(dir = %dump_dir.display(), "dumping local store state after each sync");
            syncer = syncer.with_store_dump(dump_dir.clone());
        }

        let drive_id = self.config.drive_id.clone();
        let span_for_task = tracing::info_span!("drive", drive_id = %drive_id);
        let join = tokio::spawn(async move { syncer.run().await }.instrument(span_for_task));

        Ok(EngineHandle {
            drive_id,
            status_rx,
            progress,
            trigger_tx: sync_trigger_tx,
            rescan,
            cancel,
            join,
            remote_listener_join,
        })
    }
}
