use std::time::{SystemTime, UNIX_EPOCH};

use phaneros_core::telemetry::SyncSummary;

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_relative_time(epoch_sec: Option<u64>) -> String {
    let Some(ts) = epoch_sec else {
        return "Never".to_string();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let elapsed = now.saturating_sub(ts);

    if elapsed < 60 {
        "just now".to_string()
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else if elapsed < 86400 {
        format!("{}h ago", elapsed / 3600)
    } else {
        format!("{}d ago", elapsed / 86400)
    }
}

/// Human-readable summary of a completed sync session, e.g.
/// "Synced 4.2 MB → 1.1 MB (74% smaller)", or just "Synced 4.2 MB" when
/// there was no compression gain to report.
pub fn format_activity_summary(summary: &SyncSummary) -> String {
    let raw_bytes = summary.compression.total_raw_bytes;
    let wire_bytes = summary.transfer.wire_bytes_sent;

    if wire_bytes == 0 || wire_bytes >= raw_bytes {
        return format!("Synced {}", format_bytes(raw_bytes));
    }

    let ratio_pct = (1.0 - (wire_bytes as f64 / raw_bytes as f64)) * 100.0;
    format!(
        "Synced {} \u{2192} {} ({:.0}% smaller)",
        format_bytes(raw_bytes),
        format_bytes(wire_bytes),
        ratio_pct.max(0.0)
    )
}
