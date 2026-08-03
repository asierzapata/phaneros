use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncPhase {
    Idle,
    Scanning,
    Diffing,
    TicketAllocation,
    UploadingPayloads,
    Committing,
    Materializing,
    Converged,
    Failed,
}

impl std::fmt::Display for SyncPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncPhase::Idle => write!(f, "Idle"),
            SyncPhase::Scanning => write!(f, "Scanning"),
            SyncPhase::Diffing => write!(f, "Diffing"),
            SyncPhase::TicketAllocation => write!(f, "Ticket Allocation"),
            SyncPhase::UploadingPayloads => write!(f, "Uploading Payloads"),
            SyncPhase::Committing => write!(f, "Committing"),
            SyncPhase::Materializing => write!(f, "Materializing"),
            SyncPhase::Converged => write!(f, "Converged"),
            SyncPhase::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompressionMetrics {
    pub total_raw_bytes: u64,
    pub total_compressed_bytes: u64,
    pub zstd_count: u64,
    pub bypassed_count: u64,
}

impl CompressionMetrics {
    pub fn compression_ratio(&self) -> f64 {
        if self.total_raw_bytes == 0 {
            return 0.0;
        }
        (1.0 - (self.total_compressed_bytes as f64 / self.total_raw_bytes as f64)) * 100.0
    }

    pub fn bandwidth_saved_bytes(&self) -> u64 {
        self.total_raw_bytes.saturating_sub(self.total_compressed_bytes)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferMetrics {
    pub logical_bytes: u64,
    pub deduplicated_bytes_saved: u64,
    pub wire_bytes_sent: u64,
    pub blobs_total: u64,
    pub blobs_completed: u64,
    pub blobs_skipped_dedup: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseTimings {
    pub scan_duration: Duration,
    pub diff_duration: Duration,
    pub upload_tickets_duration: Duration,
    pub payload_transfer_duration: Duration,
    pub commit_duration: Duration,
    pub materialize_duration: Duration,
    pub total_duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummary {
    pub session_id: String,
    pub drive_id: String,
    pub timestamp_epoch_sec: u64,
    pub phase_timings: PhaseTimings,
    pub compression: CompressionMetrics,
    pub transfer: TransferMetrics,
    pub avg_upload_rate_bps: u64,
    pub peak_upload_rate_bps: u64,
}

impl SyncSummary {
    pub fn new(drive_id: impl Into<String>) -> Self {
        let timestamp_epoch_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_id: blake3::hash(format!("{}:{}", timestamp_epoch_sec, rand_id()).as_bytes())
                .to_hex()[..16]
                .to_string(),
            drive_id: drive_id.into(),
            timestamp_epoch_sec,
            phase_timings: PhaseTimings::default(),
            compression: CompressionMetrics::default(),
            transfer: TransferMetrics::default(),
            avg_upload_rate_bps: 0,
            peak_upload_rate_bps: 0,
        }
    }
}

fn rand_id() -> u128 {
    let now = std::time::Instant::now();
    now.elapsed().as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_ratio() {
        let metrics = CompressionMetrics {
            total_raw_bytes: 1000,
            total_compressed_bytes: 400,
            zstd_count: 5,
            bypassed_count: 0,
        };

        assert_eq!(metrics.compression_ratio(), 60.0);
        assert_eq!(metrics.bandwidth_saved_bytes(), 600);
    }

    #[test]
    fn test_empty_raw_bytes_compression_ratio() {
        let metrics = CompressionMetrics::default();
        assert_eq!(metrics.compression_ratio(), 0.0);
        assert_eq!(metrics.bandwidth_saved_bytes(), 0);
    }
}
