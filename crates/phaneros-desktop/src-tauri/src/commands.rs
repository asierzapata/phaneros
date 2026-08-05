use std::path::{Path, PathBuf};

use phaneros_core::telemetry::{AggregateStats, DriveStatus, SyncSummary};
use phaneros_ipc::methods::{
    ActivityListParams, AddDriveParams, DriveIdParams, DriveSummary, StatsParams,
};
use phaneros_ipc::Request;
use serde::{Deserialize, Serialize};

use crate::conflicts::{self, ConflictDiffDto, ConflictSummaryDto};
use crate::daemon_locate;
use crate::format::{format_activity_summary, format_bytes, format_relative_time};
use crate::fs_scan::{self, FileNodeDto};
use crate::ipc_client;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveVaultDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: String,
    pub used_bytes: Option<u64>,
    pub quota_bytes: Option<u64>,
    pub file_count: Option<u64>,
}

fn map_status(status: &DriveStatus) -> &'static str {
    match status {
        DriveStatus::Idle => "synced",
        DriveStatus::Reconciling | DriveStatus::Materializing => "syncing",
        DriveStatus::Stopped => "paused",
        // The DriveVault status enum has no generic "error" bucket; surface
        // errored drives as needing attention the same way the UI already
        // treats conflicts, until per-drive error state gets its own affordance.
        DriveStatus::Error(_) => "conflict",
    }
}

/// Walks a vault's local directory to compute the size/file-count fields
/// `DriveSummary` doesn't carry. Best-effort: unreadable entries are skipped
/// rather than failing the whole command.
fn local_dir_stats(path: &Path) -> (u64, u64) {
    let mut total_bytes = 0u64;
    let mut file_count = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(entry.path());
            } else if metadata.is_file() {
                file_count += 1;
                total_bytes += metadata.len();
            }
        }
    }

    (total_bytes, file_count)
}

async fn to_dto(summary: DriveSummary) -> DriveVaultDto {
    let path = PathBuf::from(&summary.path);
    let (used_bytes, file_count) = tokio::task::spawn_blocking(move || local_dir_stats(&path))
        .await
        .unwrap_or((0, 0));

    DriveVaultDto {
        name: summary.drive_id.clone(),
        status: map_status(&summary.status).to_string(),
        id: summary.drive_id,
        path: summary.path,
        used_bytes: Some(used_bytes),
        quota_bytes: None,
        file_count: Some(file_count),
    }
}

#[tauri::command]
pub async fn list_vaults() -> Result<Vec<DriveVaultDto>, String> {
    let value = ipc_client::call(Request::DrivesList).await?;
    let drives: Vec<DriveSummary> = serde_json::from_value(value).map_err(|e| e.to_string())?;

    let mut dtos = Vec::with_capacity(drives.len());
    for drive in drives {
        dtos.push(to_dto(drive).await);
    }
    Ok(dtos)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryMetricsDto {
    pub last_synced: String,
    pub deduplication_ratio: String,
    pub compression_ratio: String,
    pub transfer_speed: String,
}

#[tauri::command]
pub async fn get_telemetry() -> Result<TelemetryMetricsDto, String> {
    let value = ipc_client::call(Request::StatsAggregate(StatsParams { drive_id: None })).await?;
    let stats: AggregateStats = serde_json::from_value(value).map_err(|e| e.to_string())?;

    Ok(TelemetryMetricsDto {
        last_synced: format_relative_time(stats.last_sync_timestamp_epoch_sec),
        deduplication_ratio: format_dedup_ratio(stats.total_raw_bytes, stats.total_dedup_bytes),
        compression_ratio: format!("{:.0}%", stats.overall_compression_ratio_pct.max(0.0)),
        transfer_speed: format!("{}/s", format_bytes(stats.avg_upload_rate_bps)),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySessionDto {
    pub id: String,
    pub drive_id: String,
    pub timestamp: String,
    pub summary: String,
}

fn to_activity_dto(summary: SyncSummary) -> ActivitySessionDto {
    ActivitySessionDto {
        id: summary.session_id.clone(),
        drive_id: summary.drive_id.clone(),
        timestamp: format_relative_time(Some(summary.timestamp_epoch_sec)),
        summary: format_activity_summary(&summary),
    }
}

#[tauri::command]
pub async fn list_activity(limit: Option<usize>) -> Result<Vec<ActivitySessionDto>, String> {
    let value = ipc_client::call(Request::ActivityList(ActivityListParams {
        drive_id: None,
        limit: limit.unwrap_or(20),
    }))
    .await?;
    let sessions: Vec<SyncSummary> = serde_json::from_value(value).map_err(|e| e.to_string())?;

    Ok(sessions.into_iter().map(to_activity_dto).collect())
}

#[tauri::command]
pub async fn trigger_sync() -> Result<(), String> {
    let value = ipc_client::call(Request::DrivesList).await?;
    let drives: Vec<DriveSummary> = serde_json::from_value(value).map_err(|e| e.to_string())?;

    for drive in drives {
        ipc_client::call(Request::DrivesTriggerSync(DriveIdParams {
            drive_id: drive.drive_id,
        }))
        .await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_file_tree(path: String) -> Result<Vec<FileNodeDto>, String> {
    let root = PathBuf::from(path);
    tokio::task::spawn_blocking(move || fs_scan::build_file_tree(&root))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_conflicts(vault_path: String) -> Result<Vec<ConflictSummaryDto>, String> {
    let root = PathBuf::from(vault_path);
    tokio::task::spawn_blocking(move || conflicts::scan(&root))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_conflict_diff(conflict_id: String) -> Result<ConflictDiffDto, String> {
    let path = PathBuf::from(conflict_id);
    tokio::task::spawn_blocking(move || conflicts::diff(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_conflict(conflict_id: String, keep_local: bool) -> Result<(), String> {
    let path = PathBuf::from(conflict_id);
    tokio::task::spawn_blocking(move || conflicts::resolve(&path, keep_local))
        .await
        .map_err(|e| e.to_string())?
}

fn format_dedup_ratio(raw_bytes: u64, dedup_bytes: u64) -> String {
    let denominator = raw_bytes.saturating_sub(dedup_bytes).max(1);
    format!("{:.2}\u{d7}", raw_bytes as f64 / denominator as f64)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonPingDto {
    pub version: String,
    pub configured: bool,
}

/// Confirms a `phanerosd` instance is reachable at the default control-plane
/// socket, and whether it has at least one drive configured. Used by the
/// onboarding wizard's "Test Connection" step and by the desktop app's
/// daemon-connectivity polling to decide between the "daemon unreachable",
/// onboarding, and main-app views.
#[tauri::command]
pub async fn daemon_ping() -> Result<DaemonPingDto, String> {
    let value = ipc_client::call(Request::DaemonPing).await?;
    let result: phaneros_ipc::PingResult =
        serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok(DaemonPingDto {
        version: result.version,
        configured: result.configured,
    })
}

/// Spawns `phanerosd` as a detached, one-off process. Best-effort: this only
/// confirms the process launched, not that it stayed up (e.g. it will exit
/// immediately if another instance already holds the control socket) —
/// callers should re-poll `daemon_ping` after a short delay to confirm.
#[tauri::command]
pub async fn start_daemon() -> Result<(), String> {
    let daemon_path = daemon_locate::locate_daemon_binary()?;
    std::process::Command::new(daemon_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start phanerosd: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn add_vault(
    drive_id: String,
    path: String,
    store_url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    ipc_client::call(Request::DrivesAdd(AddDriveParams {
        drive_id,
        path,
        token,
        store_url,
        enabled: true,
    }))
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingStateDto {
    pub is_completed: bool,
    pub destination_mode: String,
    pub server_url: String,
}

fn onboarding_state_path() -> Result<PathBuf, String> {
    let dir = dirs::config_dir()
        .ok_or_else(|| "Could not determine the config directory".to_string())?
        .join("phaneros");
    Ok(dir.join("desktop-onboarding.json"))
}

/// Reads the persisted onboarding completion state. Missing/unreadable file
/// is treated as "onboarding not yet completed" rather than an error, since
/// that's the expected state on first launch.
#[tauri::command]
pub async fn load_onboarding_state() -> Result<Option<OnboardingStateDto>, String> {
    let path = onboarding_state_path()?;
    let contents = match tokio::fs::read_to_string(&path).await {
        Ok(contents) => contents,
        Err(_) => return Ok(None),
    };
    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_onboarding_state(state: OnboardingStateDto) -> Result<(), String> {
    let path = onboarding_state_path()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }
    let contents = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, contents)
        .await
        .map_err(|e| e.to_string())
}
