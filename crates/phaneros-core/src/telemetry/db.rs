use std::path::{Path, PathBuf};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use serde::{Deserialize, Serialize};

use super::metrics::{CompressionMetrics, PhaseTimings, SyncSummary, TransferMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateStats {
    pub total_syncs: u64,
    pub total_raw_bytes: u64,
    pub total_wire_bytes: u64,
    pub total_dedup_bytes: u64,
    pub overall_compression_ratio_pct: f64,
    pub avg_upload_rate_bps: u64,
    pub last_sync_timestamp_epoch_sec: Option<u64>,
}

#[derive(Clone)]
pub struct MetricsDatabase {
    pool: SqlitePool,
}

impl MetricsDatabase {
    pub fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::env::temp_dir());
        path.push("phaneros");
        path.push("metrics.db");
        path
    }

    pub async fn connect(path: &Path) -> Result<Self, sqlx::Error> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn connect_default() -> Result<Self, sqlx::Error> {
        Self::connect(&Self::default_path()).await
    }

    pub async fn connect_in_memory() -> Result<Self, sqlx::Error> {
        let options = "sqlite::memory:"
            .parse::<SqliteConnectOptions>()?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn insert_summary_blocking(&self, summary: &SyncSummary) -> Result<(), sqlx::Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(self.insert_summary(summary))
    }

    pub async fn insert_summary(&self, summary: &SyncSummary) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO sync_history (
                id, drive_id, timestamp, total_duration_ms, scan_duration_ms, diff_duration_ms,
                upload_tickets_duration_ms, payload_transfer_duration_ms, commit_duration_ms,
                materialize_duration_ms, raw_bytes, wire_bytes, dedup_bytes, compression_ratio_pct,
                avg_speed_bps, peak_speed_bps
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        )
        .bind(&summary.session_id)
        .bind(&summary.drive_id)
        .bind(summary.timestamp_epoch_sec as i64)
        .bind(summary.phase_timings.total_duration.as_millis() as i64)
        .bind(summary.phase_timings.scan_duration.as_millis() as i64)
        .bind(summary.phase_timings.diff_duration.as_millis() as i64)
        .bind(summary.phase_timings.upload_tickets_duration.as_millis() as i64)
        .bind(summary.phase_timings.payload_transfer_duration.as_millis() as i64)
        .bind(summary.phase_timings.commit_duration.as_millis() as i64)
        .bind(summary.phase_timings.materialize_duration.as_millis() as i64)
        .bind(summary.compression.total_raw_bytes as i64)
        .bind(summary.transfer.wire_bytes_sent as i64)
        .bind(summary.transfer.deduplicated_bytes_saved as i64)
        .bind(summary.compression.compression_ratio())
        .bind(summary.avg_upload_rate_bps as i64)
        .bind(summary.peak_upload_rate_bps as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_history(&self, drive_id: Option<&str>, limit: usize) -> Result<Vec<SyncSummary>, sqlx::Error> {
        let rows = if let Some(did) = drive_id {
            sqlx::query(
                "SELECT id, drive_id, timestamp, total_duration_ms, scan_duration_ms, diff_duration_ms,
                        upload_tickets_duration_ms, payload_transfer_duration_ms, commit_duration_ms,
                        materialize_duration_ms, raw_bytes, wire_bytes, dedup_bytes, avg_speed_bps, peak_speed_bps
                 FROM sync_history WHERE drive_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
            )
            .bind(did)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, drive_id, timestamp, total_duration_ms, scan_duration_ms, diff_duration_ms,
                        upload_tickets_duration_ms, payload_transfer_duration_ms, commit_duration_ms,
                        materialize_duration_ms, raw_bytes, wire_bytes, dedup_bytes, avg_speed_bps, peak_speed_bps
                 FROM sync_history ORDER BY timestamp DESC LIMIT ?1"
            )
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?
        };

        let mut result = Vec::new();
        for row in rows {
            let id: String = row.get(0);
            let drive_id: String = row.get(1);
            let timestamp: i64 = row.get(2);
            let total_ms: i64 = row.get(3);
            let scan_ms: i64 = row.get(4);
            let diff_ms: i64 = row.get(5);
            let tickets_ms: i64 = row.get(6);
            let payload_ms: i64 = row.get(7);
            let commit_ms: i64 = row.get(8);
            let mat_ms: i64 = row.get(9);
            let raw_bytes: i64 = row.get(10);
            let wire_bytes: i64 = row.get(11);
            let dedup_bytes: i64 = row.get(12);
            let avg_speed: i64 = row.get(13);
            let peak_speed: i64 = row.get(14);

            result.push(SyncSummary {
                session_id: id,
                drive_id,
                timestamp_epoch_sec: timestamp as u64,
                phase_timings: PhaseTimings {
                    scan_duration: std::time::Duration::from_millis(scan_ms as u64),
                    diff_duration: std::time::Duration::from_millis(diff_ms as u64),
                    upload_tickets_duration: std::time::Duration::from_millis(tickets_ms as u64),
                    payload_transfer_duration: std::time::Duration::from_millis(payload_ms as u64),
                    commit_duration: std::time::Duration::from_millis(commit_ms as u64),
                    materialize_duration: std::time::Duration::from_millis(mat_ms as u64),
                    total_duration: std::time::Duration::from_millis(total_ms as u64),
                },
                compression: CompressionMetrics {
                    total_raw_bytes: raw_bytes as u64,
                    total_compressed_bytes: wire_bytes as u64,
                    zstd_count: 0,
                    bypassed_count: 0,
                },
                transfer: TransferMetrics {
                    logical_bytes: raw_bytes as u64,
                    deduplicated_bytes_saved: dedup_bytes as u64,
                    wire_bytes_sent: wire_bytes as u64,
                    blobs_total: 0,
                    blobs_completed: 0,
                    blobs_skipped_dedup: 0,
                },
                avg_upload_rate_bps: avg_speed as u64,
                peak_upload_rate_bps: peak_speed as u64,
            });
        }

        Ok(result)
    }

    pub async fn get_aggregate_stats(&self, drive_id: Option<&str>) -> Result<AggregateStats, sqlx::Error> {
        let row = if let Some(did) = drive_id {
            sqlx::query(
                "SELECT COUNT(*), SUM(raw_bytes), SUM(wire_bytes), SUM(dedup_bytes), AVG(avg_speed_bps), MAX(timestamp)
                 FROM sync_history WHERE drive_id = ?1"
            )
            .bind(did)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT COUNT(*), SUM(raw_bytes), SUM(wire_bytes), SUM(dedup_bytes), AVG(avg_speed_bps), MAX(timestamp)
                 FROM sync_history"
            )
            .fetch_one(&self.pool)
            .await?
        };

        let count: i64 = row.get(0);
        let raw: Option<i64> = row.get(1);
        let wire: Option<i64> = row.get(2);
        let dedup: Option<i64> = row.get(3);
        let avg_speed: Option<f64> = row.get(4);
        let last_timestamp: Option<i64> = row.get(5);

        let raw_bytes = raw.unwrap_or(0) as u64;
        let wire_bytes = wire.unwrap_or(0) as u64;
        let dedup_bytes = dedup.unwrap_or(0) as u64;

        let compression_ratio = if raw_bytes > 0 {
            (1.0 - (wire_bytes as f64 / raw_bytes as f64)) * 100.0
        } else {
            0.0
        };

        Ok(AggregateStats {
            total_syncs: count as u64,
            total_raw_bytes: raw_bytes,
            total_wire_bytes: wire_bytes,
            total_dedup_bytes: dedup_bytes,
            overall_compression_ratio_pct: compression_ratio,
            avg_upload_rate_bps: avg_speed.unwrap_or(0.0) as u64,
            last_sync_timestamp_epoch_sec: last_timestamp.map(|t| t as u64),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_database_insert_and_query() {
        let db = MetricsDatabase::connect_in_memory().await.unwrap();
        let mut summary = SyncSummary::new("test_drive");
        summary.compression.total_raw_bytes = 10000;
        summary.transfer.wire_bytes_sent = 4000;
        summary.transfer.deduplicated_bytes_saved = 1000;
        summary.avg_upload_rate_bps = 50000;

        db.insert_summary(&summary).await.unwrap();

        let history = db.get_history(Some("test_drive"), 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].drive_id, "test_drive");
        assert_eq!(history[0].compression.total_raw_bytes, 10000);
        assert_eq!(history[0].transfer.wire_bytes_sent, 4000);

        let stats = db.get_aggregate_stats(Some("test_drive")).await.unwrap();
        assert_eq!(stats.total_syncs, 1);
        assert_eq!(stats.total_raw_bytes, 10000);
        assert_eq!(stats.total_wire_bytes, 4000);
        assert_eq!(stats.total_dedup_bytes, 1000);
        assert_eq!(stats.overall_compression_ratio_pct, 60.0);
    }
}
