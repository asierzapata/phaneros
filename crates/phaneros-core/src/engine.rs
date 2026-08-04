use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use thiserror::Error;

use crate::blob_repository::HttpBlobRepository;
use crate::node_repository::HttpNodeRepository;
use crate::syncer::Syncer;
use crate::syncer::sync_state::DriveSession;
use crate::watcher::{WatchHandle, Watcher, spawn_remote_listener};

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

    pub async fn run(&self) -> Result<(), EngineError> {
        let drive_session = DriveSession::open(&self.config.drive_id, &self.config.path)
            .map_err(|e| EngineError::DriveSessionFailed(e.to_string()))?;

        let watcher = Watcher::new(self.config.path.to_string_lossy().into_owned());

        println!("Watcher started, waiting for changes...");

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

        let remote_node_repository = Arc::new(RwLock::new(
            HttpNodeRepository::new(
                self.config.store_url.clone(),
                self.config.drive_id.clone(),
                self.config.token.clone(),
            ).await
        ));

        let remote_rescan = rescan.clone();
        let remote_trigger_tx = sync_trigger_tx.clone();
        let _remote_listener = spawn_remote_listener(
            self.config.store_url.clone(),
            self.config.drive_id.clone(),
            self.config.token.clone(),
            move |_event| match remote_rescan.rescan() {
                Ok(root_hash) => {
                    let _ = remote_trigger_tx.try_send(root_hash);
                }
                Err(err) => {
                    eprintln!("Failed to rescan after remote root-changed event: {err}");
                }
            },
        );

        let remote_blob_repository = Arc::new(RwLock::new(
            HttpBlobRepository::new(
                self.config.store_url.clone(),
                self.config.drive_id.clone(),
                self.config.token.clone(),
            )
        ));

        let mut syncer = Syncer::new(
            sync_trigger_rx,
            initial_root_hash,
            local_node_repository,
            remote_node_repository,
            local_blob_repository,
            remote_blob_repository,
            drive_session,
            rescan,
        )
        .with_telemetry(self.config.enable_telemetry)
        .with_max_concurrent_uploads(self.config.max_concurrent_uploads);

        drop(sync_trigger_tx);

        if let Some(ref dump_dir) = self.config.dump_store {
            println!(
                "Dumping local store state to {}/ after each sync.",
                dump_dir.display()
            );
            syncer = syncer.with_store_dump(dump_dir.clone());
        }

        syncer.run().await;
        Ok(())
    }
}
