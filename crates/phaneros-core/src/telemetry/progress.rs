use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

use super::metrics::{CompressionMetrics, PhaseTimings, SyncPhase, SyncSummary, TransferMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgressEvent {
    pub phase: SyncPhase,
    pub blobs_completed: u64,
    pub blobs_total: u64,
    pub bytes_sent: u64,
    pub bytes_total: u64,
    pub instantaneous_speed_bps: u64,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug)]
struct Sample {
    at: Instant,
    bytes: u64,
}

#[derive(Debug)]
pub struct ProgressTrackerInner {
    pub current_phase: SyncPhase,
    pub compression: CompressionMetrics,
    pub transfer: TransferMetrics,
    pub phase_timings: PhaseTimings,
    pub start_time: Instant,
    pub phase_start_time: Instant,
    pub peak_speed_bps: u64,
    samples: VecDeque<Sample>,
}

#[derive(Debug, Clone)]
pub struct ProgressTracker {
    inner: Arc<Mutex<ProgressTrackerInner>>,
    drive_id: String,
}

impl ProgressTracker {
    pub fn new(drive_id: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            drive_id: drive_id.into(),
            inner: Arc::new(Mutex::new(ProgressTrackerInner {
                current_phase: SyncPhase::Idle,
                compression: CompressionMetrics::default(),
                transfer: TransferMetrics::default(),
                phase_timings: PhaseTimings::default(),
                start_time: now,
                phase_start_time: now,
                peak_speed_bps: 0,
                samples: VecDeque::new(),
            })),
        }
    }

    pub fn set_phase(&self, phase: SyncPhase) {
        let mut inner = self.inner.lock().unwrap();
        let elapsed = inner.phase_start_time.elapsed();
        match inner.current_phase {
            SyncPhase::Scanning => inner.phase_timings.scan_duration = elapsed,
            SyncPhase::Diffing => inner.phase_timings.diff_duration = elapsed,
            SyncPhase::TicketAllocation => inner.phase_timings.upload_tickets_duration = elapsed,
            SyncPhase::UploadingPayloads => inner.phase_timings.payload_transfer_duration = elapsed,
            SyncPhase::Committing => inner.phase_timings.commit_duration = elapsed,
            SyncPhase::Materializing => inner.phase_timings.materialize_duration = elapsed,
            _ => {}
        }
        inner.current_phase = phase;
        inner.phase_start_time = Instant::now();
    }

    pub fn record_blob_compressed(&self, raw_bytes: u64, compressed_bytes: u64, is_zstd: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.compression.total_raw_bytes += raw_bytes;
        inner.compression.total_compressed_bytes += compressed_bytes;
        if is_zstd {
            inner.compression.zstd_count += 1;
        } else {
            inner.compression.bypassed_count += 1;
        }
    }

    pub fn record_blob_skipped_dedup(&self, raw_bytes: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.transfer.deduplicated_bytes_saved += raw_bytes;
        inner.transfer.blobs_skipped_dedup += 1;
        inner.transfer.blobs_completed += 1;
    }

    pub fn set_blobs_total(&self, total: u64, logical_bytes: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.transfer.blobs_total = total;
        inner.transfer.logical_bytes = logical_bytes;
    }

    pub fn record_bytes_sent(&self, wire_bytes: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.transfer.wire_bytes_sent += wire_bytes;
        inner.transfer.blobs_completed += 1;

        let now = Instant::now();
        inner.samples.push_back(Sample {
            at: now,
            bytes: wire_bytes,
        });

        // Retain samples within 3 seconds window
        while let Some(front) = inner.samples.front() {
            if now.duration_since(front.at) > Duration::from_secs(3) {
                inner.samples.pop_front();
            } else {
                break;
            }
        }

        let speed = Self::calc_instantaneous_speed(&inner.samples);
        if speed > inner.peak_speed_bps {
            inner.peak_speed_bps = speed;
        }
    }

    fn calc_instantaneous_speed(samples: &VecDeque<Sample>) -> u64 {
        if samples.len() < 2 {
            return 0;
        }
        let first = samples.front().unwrap();
        let last = samples.back().unwrap();
        let duration = last.at.duration_since(first.at).as_secs_f64();
        if duration <= 0.0 {
            return 0;
        }
        let total_bytes: u64 = samples.iter().map(|s| s.bytes).sum();
        (total_bytes as f64 / duration) as u64
    }

    pub fn current_event(&self) -> SyncProgressEvent {
        let inner = self.inner.lock().unwrap();
        let speed = Self::calc_instantaneous_speed(&inner.samples);
        let bytes_sent = inner.transfer.wire_bytes_sent;
        let bytes_total = inner.transfer.logical_bytes;

        let eta_seconds = if speed > 0 && bytes_total > bytes_sent {
            Some((bytes_total - bytes_sent) / speed)
        } else {
            None
        };

        SyncProgressEvent {
            phase: inner.current_phase,
            blobs_completed: inner.transfer.blobs_completed,
            blobs_total: inner.transfer.blobs_total,
            bytes_sent,
            bytes_total,
            instantaneous_speed_bps: speed,
            eta_seconds,
        }
    }

    pub fn finalize(&self) -> SyncSummary {
        let mut inner = self.inner.lock().unwrap();
        let total_elapsed = inner.start_time.elapsed();
        inner.phase_timings.total_duration = total_elapsed;

        let wire_bytes = inner.transfer.wire_bytes_sent;
        let total_secs = total_elapsed.as_secs_f64();
        let avg_speed = if total_secs > 0.0 {
            (wire_bytes as f64 / total_secs) as u64
        } else {
            0
        };

        let mut summary = SyncSummary::new(&self.drive_id);
        summary.phase_timings = inner.phase_timings.clone();
        summary.compression = inner.compression.clone();
        summary.transfer = inner.transfer.clone();
        summary.avg_upload_rate_bps = avg_speed;
        summary.peak_upload_rate_bps = inner.peak_speed_bps;
        summary
    }
}
