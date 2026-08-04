use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use phaneros_core::syncer::sync_state::DriveSession;
use phaneros_core::telemetry::{DriveStatus, MetricsDatabase};
use phaneros_core::{EngineConfig, EngineHandle, PhanerosConfig, SyncEngine};
use phaneros_ipc::methods::{AddDriveParams, DriveStatusResult, DriveSummary};
use phaneros_ipc::{JsonRpcError, PingResult};
use serde_json::{Value, json};
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

use crate::notify::forward_status_changes;

pub type Reply = oneshot::Sender<Result<Value, JsonRpcError>>;

pub enum Command {
    Ping(Reply),
    Shutdown(Reply),
    ListDrives(Reply),
    DriveStatus { drive_id: String, reply: Reply },
    StartDrive { drive_id: String, reply: Reply },
    StopDrive { drive_id: String, reply: Reply },
    AddDrive { params: AddDriveParams, reply: Reply },
    RemoveDrive { drive_id: String, reply: Reply },
    TriggerSync { drive_id: String, reply: Reply },
    ReloadConfig(Reply),
    StatsAggregate { drive_id: Option<String>, reply: Reply },
}

fn internal_error(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, message)
}

fn invalid_params(message: impl Into<String>) -> JsonRpcError {
    JsonRpcError::new(JsonRpcError::INVALID_PARAMS, message)
}

pub struct DaemonState {
    config: PhanerosConfig,
    config_path: PathBuf,
    drives: HashMap<String, EngineHandle>,
    broadcast_tx: broadcast::Sender<phaneros_ipc::Notification>,
    started_at: Instant,
    daemon_cancel: CancellationToken,
}

impl DaemonState {
    pub fn new(
        config: PhanerosConfig,
        config_path: PathBuf,
        broadcast_tx: broadcast::Sender<phaneros_ipc::Notification>,
        daemon_cancel: CancellationToken,
    ) -> Self {
        Self {
            config,
            config_path,
            drives: HashMap::new(),
            broadcast_tx,
            started_at: Instant::now(),
            daemon_cancel,
        }
    }

    /// Spawns an engine for every drive marked `enabled` in the loaded config.
    pub async fn start_enabled_drives(&mut self) {
        let drive_ids: Vec<String> = self
            .config
            .drives
            .iter()
            .filter(|(_, drive)| drive.enabled)
            .map(|(id, _)| id.clone())
            .collect();

        for drive_id in drive_ids {
            if let Err(err) = self.spawn_drive(&drive_id).await {
                tracing::error!(drive_id, "failed to start drive on daemon startup: {}", err);
            }
        }
    }

    async fn spawn_drive(&mut self, drive_id: &str) -> Result<(), String> {
        let drive = self
            .config
            .drives
            .get(drive_id)
            .ok_or_else(|| format!("unknown drive '{}'", drive_id))?
            .clone();

        let store_url = drive
            .get_effective_store_url(&self.config.daemon.store_url)
            .to_string();

        let engine_config = EngineConfig::new(
            drive.expanded_path(),
            store_url,
            drive_id.to_string(),
            drive.token.clone(),
            None,
        )
        .with_telemetry(self.config.daemon.enable_telemetry)
        .with_max_concurrent_uploads(self.config.daemon.max_concurrent_uploads);

        let handle = SyncEngine::new(engine_config)
            .spawn()
            .await
            .map_err(|e| e.to_string())?;

        forward_status_changes(
            drive_id.to_string(),
            handle.subscribe_status(),
            self.broadcast_tx.clone(),
            self.daemon_cancel.clone(),
        );

        self.drives.insert(drive_id.to_string(), handle);
        Ok(())
    }

    fn persist_config(&self) -> Result<(), String> {
        self.config
            .save_to_path(&self.config_path)
            .map_err(|e| e.to_string())
    }

    fn drive_summary(&self, drive_id: &str, drive: &phaneros_core::DriveConfig) -> DriveSummary {
        let status = match self.drives.get(drive_id) {
            Some(handle) => handle.status(),
            None => DriveStatus::Stopped,
        };

        let last_synced_root = DriveSession::open(drive_id, &drive.expanded_path())
            .ok()
            .and_then(|session| session.state.last_synced_root.clone())
            .map(|hash| hash.to_string());

        DriveSummary {
            drive_id: drive_id.to_string(),
            path: drive.expanded_path().to_string_lossy().to_string(),
            store_url: drive
                .get_effective_store_url(&self.config.daemon.store_url)
                .to_string(),
            enabled: drive.enabled,
            status,
            last_synced_root,
            last_error: None,
        }
    }

    pub async fn handle(&mut self, command: Command) {
        match command {
            Command::Ping(reply) => {
                let result = PingResult {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    pid: std::process::id(),
                    uptime_seconds: self.started_at.elapsed().as_secs(),
                };
                let _ = reply.send(Ok(serde_json::to_value(result).unwrap()));
            }

            Command::Shutdown(reply) => {
                tracing::info!("daemon.shutdown received; stopping all drives");
                let handles: Vec<_> = self.drives.drain().collect();
                for (drive_id, handle) in handles {
                    tracing::info!(drive_id, "shutting down drive");
                    handle.shutdown().await;
                }
                let _ = reply.send(Ok(json!({})));
                self.daemon_cancel.cancel();
            }

            Command::ListDrives(reply) => {
                let drives = self
                    .config
                    .drives
                    .iter()
                    .map(|(id, drive)| self.drive_summary(id, drive))
                    .collect::<Vec<_>>();
                let _ = reply.send(Ok(serde_json::to_value(drives).unwrap()));
            }

            Command::DriveStatus { drive_id, reply } => {
                let result = match self.config.drives.get(&drive_id).cloned() {
                    Some(drive) => {
                        let summary = self.drive_summary(&drive_id, &drive);
                        let progress = self.drives.get(&drive_id).map(|h| h.progress());
                        Ok(serde_json::to_value(DriveStatusResult { summary, progress }).unwrap())
                    }
                    None => Err(invalid_params(format!("unknown drive '{}'", drive_id))),
                };
                let _ = reply.send(result);
            }

            Command::StartDrive { drive_id, reply } => {
                let result = if self.drives.contains_key(&drive_id) {
                    Err(invalid_params(format!("drive '{}' is already running", drive_id)))
                } else if !self.config.drives.contains_key(&drive_id) {
                    Err(invalid_params(format!("unknown drive '{}'", drive_id)))
                } else {
                    match self.spawn_drive(&drive_id).await {
                        Ok(()) => {
                            if let Some(drive) = self.config.drives.get_mut(&drive_id) {
                                drive.enabled = true;
                            }
                            let _ = self.persist_config();
                            Ok(json!({}))
                        }
                        Err(err) => Err(internal_error(err)),
                    }
                };
                let _ = reply.send(result);
            }

            Command::StopDrive { drive_id, reply } => {
                let result = match self.drives.remove(&drive_id) {
                    Some(handle) => {
                        handle.shutdown().await;
                        if let Some(drive) = self.config.drives.get_mut(&drive_id) {
                            drive.enabled = false;
                        }
                        let _ = self.persist_config();
                        Ok(json!({}))
                    }
                    None => Err(invalid_params(format!("drive '{}' is not running", drive_id))),
                };
                let _ = reply.send(result);
            }

            Command::AddDrive { params, reply } => {
                let result = if self.config.drives.contains_key(&params.drive_id) {
                    Err(invalid_params(format!(
                        "drive '{}' already exists",
                        params.drive_id
                    )))
                } else {
                    let drive_id = params.drive_id.clone();
                    let drive = phaneros_core::DriveConfig {
                        path: PathBuf::from(params.path),
                        token: params.token.unwrap_or_default(),
                        store_url: params.store_url,
                        enabled: params.enabled,
                    };
                    self.config.drives.insert(drive_id.clone(), drive);

                    let spawn_result = if params.enabled {
                        self.spawn_drive(&drive_id).await
                    } else {
                        Ok(())
                    };

                    match spawn_result {
                        Ok(()) => {
                            let _ = self.persist_config();
                            Ok(json!({}))
                        }
                        Err(err) => {
                            self.config.drives.remove(&drive_id);
                            Err(internal_error(err))
                        }
                    }
                };
                let _ = reply.send(result);
            }

            Command::RemoveDrive { drive_id, reply } => {
                if let Some(handle) = self.drives.remove(&drive_id) {
                    handle.shutdown().await;
                }
                let result = if self.config.drives.remove(&drive_id).is_some() {
                    let _ = self.persist_config();
                    Ok(json!({}))
                } else {
                    Err(invalid_params(format!("unknown drive '{}'", drive_id)))
                };
                let _ = reply.send(result);
            }

            Command::TriggerSync { drive_id, reply } => {
                let result = match self.drives.get(&drive_id) {
                    Some(handle) => handle
                        .trigger_sync()
                        .await
                        .map(|()| json!({}))
                        .map_err(|e| internal_error(e.to_string())),
                    None => Err(invalid_params(format!("drive '{}' is not running", drive_id))),
                };
                let _ = reply.send(result);
            }

            Command::ReloadConfig(reply) => {
                let result = self.reload_config().await;
                let _ = reply.send(result);
            }

            Command::StatsAggregate { drive_id, reply } => {
                let result = match MetricsDatabase::connect_default().await {
                    Ok(db) => db
                        .get_aggregate_stats(drive_id.as_deref())
                        .await
                        .map(|stats| serde_json::to_value(stats).unwrap())
                        .map_err(|e| internal_error(e.to_string())),
                    Err(err) => Err(internal_error(err.to_string())),
                };
                let _ = reply.send(result);
            }
        }
    }

    async fn reload_config(&mut self) -> Result<Value, JsonRpcError> {
        let fresh = PhanerosConfig::load_from_path(&self.config_path)
            .map_err(|e| internal_error(e.to_string()))?;

        let currently_running: Vec<String> = self.drives.keys().cloned().collect();
        for drive_id in currently_running {
            let should_run = fresh
                .drives
                .get(&drive_id)
                .map(|d| d.enabled)
                .unwrap_or(false);
            if !should_run {
                if let Some(handle) = self.drives.remove(&drive_id) {
                    handle.shutdown().await;
                }
            }
        }

        self.config = fresh;

        let to_start: Vec<String> = self
            .config
            .drives
            .iter()
            .filter(|(id, drive)| drive.enabled && !self.drives.contains_key(*id))
            .map(|(id, _)| id.clone())
            .collect();

        for drive_id in to_start {
            if let Err(err) = self.spawn_drive(&drive_id).await {
                tracing::error!(drive_id, "failed to start drive after config reload: {}", err);
            }
        }

        Ok(json!({}))
    }
}
