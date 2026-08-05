use std::path::{Path, PathBuf};

use phaneros_core::telemetry::{AggregateStats, DriveStatus};
use phaneros_ipc::methods::{DriveIdParams, DriveSummary, StatsParams};
use phaneros_ipc::Request;
use serde::Serialize;

use crate::conflicts::{self, ConflictDiffDto, ConflictSummaryDto};
use crate::format::{format_bytes, format_relative_time};
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
