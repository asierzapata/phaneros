use serde::{Deserialize, Serialize};

/// Lifecycle status of a single drive's sync engine, as observed from outside
/// (e.g. by a daemon supervising multiple drives, or over the IPC control
/// plane). Distinct from `SyncPhase`, which tracks progress *within* one
/// reconcile pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DriveStatus {
    Idle,
    Reconciling,
    Materializing,
    Stopped,
    Error(String),
}

impl std::fmt::Display for DriveStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveStatus::Idle => write!(f, "Idle"),
            DriveStatus::Reconciling => write!(f, "Reconciling"),
            DriveStatus::Materializing => write!(f, "Materializing"),
            DriveStatus::Stopped => write!(f, "Stopped"),
            DriveStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}
