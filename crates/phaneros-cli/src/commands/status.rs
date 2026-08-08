use phaneros_ipc::methods::{DriveIdParams, DriveStatusResult};
use phaneros_ipc::Request;
use crate::commands::client::{expect_value, IpcCaller};
use crate::ui;

pub async fn handle(client: &impl IpcCaller, drive_id: String) {
    let value = client
        .call(Request::DrivesStatus(DriveIdParams { drive_id }))
        .await;
    let status: DriveStatusResult = expect_value(value);
    render(&status);
}

fn render(status: &DriveStatusResult) {
    let mut items = vec![
        ("Drive", status.summary.drive_id.clone()),
        ("Path", status.summary.path.clone()),
        ("Store URL", status.summary.store_url.clone()),
        ("Enabled", status.summary.enabled.to_string()),
        ("Status", status.summary.status.to_string()),
    ];
    if let Some(root) = &status.summary.last_synced_root {
        items.push(("Last root", root.clone()));
    }
    if let Some(progress) = &status.progress {
        items.push((
            "Progress",
            format!(
                "{:?} ({}/{} blobs, {} sent)",
                progress.phase,
                progress.blobs_completed,
                progress.blobs_total,
                ui::format_bytes(progress.bytes_sent)
            ),
        ));
    }
    ui::render_key_values(&items.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::client::MockIpcCaller;
    use serde_json::json;

    #[tokio::test]
    async fn test_status_command() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::DrivesStatus(_)));
            json!({
                "drive_id": "main-vault",
                "path": "/vault",
                "store_url": "http://localhost:8080",
                "enabled": true,
                "status": "Idle",
                "last_synced_root": null,
                "last_error": null,
                "progress": null
            })
        });

        handle(&mock, "main-vault".to_string()).await;
    }
}
