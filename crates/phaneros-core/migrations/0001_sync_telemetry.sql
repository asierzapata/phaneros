CREATE TABLE IF NOT EXISTS sync_history (
    id TEXT PRIMARY KEY,
    drive_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    total_duration_ms INTEGER NOT NULL,
    scan_duration_ms INTEGER NOT NULL,
    diff_duration_ms INTEGER NOT NULL,
    upload_tickets_duration_ms INTEGER NOT NULL,
    payload_transfer_duration_ms INTEGER NOT NULL,
    commit_duration_ms INTEGER NOT NULL,
    materialize_duration_ms INTEGER NOT NULL,
    raw_bytes INTEGER NOT NULL,
    wire_bytes INTEGER NOT NULL,
    dedup_bytes INTEGER NOT NULL,
    compression_ratio_pct REAL NOT NULL,
    avg_speed_bps INTEGER NOT NULL,
    peak_speed_bps INTEGER NOT NULL
);
