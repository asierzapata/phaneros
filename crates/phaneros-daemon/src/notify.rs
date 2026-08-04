use phaneros_core::telemetry::DriveStatus;
use phaneros_ipc::Notification;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;

/// Spawns a task that forwards a drive's status transitions onto the
/// daemon's broadcast channel, so any IPC connection that has called
/// `events.subscribe` sees them as `event.drive_status_changed`
/// notifications. Runs until the drive's status channel closes (the engine
/// stopped) or the daemon is shutting down.
pub fn forward_status_changes(
    drive_id: String,
    mut status_rx: watch::Receiver<DriveStatus>,
    broadcast_tx: broadcast::Sender<Notification>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                changed = status_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let status = status_rx.borrow().clone();
                    let _ = broadcast_tx.send(Notification::EventDriveStatusChanged {
                        drive_id: drive_id.clone(),
                        status,
                    });
                }
            }
        }
    });
}
