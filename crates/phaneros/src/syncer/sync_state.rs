use fs2::FileExt;
use phaneros_sync::hash::Hash;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncState {
    pub schema_version: u32, // always 1 for now
    pub drive_id: String,
    pub local_path: String, // canonical absolute path
    pub last_synced_root: Option<Hash>,
}

impl SyncState {
    pub fn state_path(file_hash: &str) -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::env::temp_dir());
        path.push("phaneros");
        path.push("sync_state");
        path.push(file_hash);
        path.set_extension("json");
        path
    }

    pub fn lock_path(file_hash: &str) -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::env::temp_dir());
        path.push("phaneros");
        path.push("sync_state");
        path.push(file_hash);
        path.set_extension("lock");
        path
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SyncStateError {
    #[error("failed to parse state file: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("failed to open state file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("unknown state schema version: {0}")]
    UnknownSchemaVersion(u32),
    #[error(
        "state identity mismatch: expected drive_id={expected_drive_id} local_path={expected_local_path}, found drive_id={found_drive_id} local_path={found_local_path}"
    )]
    StateIdentityMismatch {
        expected_drive_id: String,
        expected_local_path: String,
        found_drive_id: String,
        found_local_path: String,
    },
}

pub struct DriveSession {
    pub state: SyncState,
    state_path: std::path::PathBuf,
    _lock: DriveLock, // keep alive to keep lock held
}

impl DriveSession {
    pub fn new(state: SyncState, state_path: std::path::PathBuf, lock: DriveLock) -> Self {
        Self {
            state,
            state_path,
            _lock: lock,
        }
    }

    pub fn open(drive_id: &str, local_path: &std::path::Path) -> Result<Self, SyncStateError> {
        // We try to acquire a lock on drive state (drive_id + local_path) to prevent concurrent syncs on the same drive.
        let canonical_path = local_path.canonicalize()?.to_string_lossy().to_string();
        let file_hash = blake3::hash(format!("{}:{}", drive_id, canonical_path).as_bytes())
            .to_hex()
            .to_string();
        let lock = DriveLock::acquire(&SyncState::lock_path(&file_hash))?;
        let state_path = SyncState::state_path(&file_hash);

        // With the lock, we can safely read the state file (if it exists) and update the sync state.
        if state_path.exists() {
            let data = std::fs::read(&state_path)?;
            let state: SyncState = serde_json::from_slice(&data)?;
            // If the state file has a different schema version, we return an error.
            // In the future, we could implement a migration path.
            if state.schema_version != 1 {
                return Err(SyncStateError::UnknownSchemaVersion(state.schema_version));
            }
            if state.drive_id != drive_id || state.local_path != canonical_path {
                return Err(SyncStateError::StateIdentityMismatch {
                    expected_drive_id: drive_id.to_string(),
                    expected_local_path: canonical_path,
                    found_drive_id: state.drive_id,
                    found_local_path: state.local_path,
                });
            }
            Ok(Self::new(state, state_path, lock))
        } else {
            let drive_session = Self::new(
                SyncState {
                    schema_version: 1,
                    drive_id: drive_id.to_string(),
                    local_path: canonical_path,
                    last_synced_root: None,
                },
                state_path,
                lock,
            );

            drive_session.persist()?;

            Ok(drive_session)
        }
    }
    pub fn persist(&self) -> Result<(), SyncStateError> {
        let data = serde_json::to_vec_pretty(&self.state)?;
        std::fs::create_dir_all(self.state_path.parent().unwrap())?;
        // we do a write to a temp file and then rename to avoid partial writes
        let temp_path = self.state_path.with_extension("json.tmp");
        std::fs::write(&temp_path, data)?;
        std::fs::rename(temp_path, &self.state_path)?;
        Ok(())
    }

    pub fn set_last_synced_root(&mut self, root: Option<Hash>) {
        self.state.last_synced_root = root;
    }
}

pub struct DriveLock {
    _file: std::fs::File,
    _path: std::path::PathBuf,
}

impl DriveLock {
    pub fn acquire(path: &std::path::Path) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(path.parent().unwrap())?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.try_lock_exclusive()?;
        Ok(Self {
            _file: file,
            _path: path.to_path_buf(),
        })
    }
}
