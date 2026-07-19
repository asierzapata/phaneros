-- Current root pointer per drive.
CREATE TABLE IF NOT EXISTS drive_roots (
    drive_id  TEXT PRIMARY KEY,
    root_hash TEXT NOT NULL
);

-- Immutable node with serialized JSON as, addressed by (drive, hash).
CREATE TABLE IF NOT EXISTS nodes (
    drive_id TEXT NOT NULL,
    hash     TEXT NOT NULL,
    data     TEXT NOT NULL,
    PRIMARY KEY (drive_id, hash)
);

-- Append-only log of accepted root flips, one row per version. Newest-first by id.
CREATE TABLE IF NOT EXISTS versions (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    drive_id  TEXT NOT NULL,
    root_hash TEXT NOT NULL,
    at        INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_versions_drive ON versions (drive_id, id DESC);

-- Blob metadata plane. `size` is the client-declared byte length, captured when
-- the upload ticket is minted. It drives pruning decisions.
CREATE TABLE IF NOT EXISTS blob_metadata (
    hash         TEXT PRIMARY KEY,
    size         INTEGER NOT NULL,
    committed_at INTEGER
);
