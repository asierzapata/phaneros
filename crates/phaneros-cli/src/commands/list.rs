use phaneros_ipc::methods::DriveSummary;
use phaneros_ipc::Request;
use crate::commands::client::{expect_value, IpcCaller};
use crate::ui;

pub async fn handle(client: &impl IpcCaller) {
    let value = client.call(Request::DrivesList).await;
    let drives: Vec<DriveSummary> = expect_value(value);
    render(&drives);
}

fn render(drives: &[DriveSummary]) {
    if drives.is_empty() {
        println!("No drives configured.");
        return;
    }
    let mut table = ui::Table::with_columns(vec![
        ui::Column::new("DRIVE"),
        ui::Column::new("ENABLED"),
        ui::Column::new("STATUS"),
        ui::Column::new("PATH"),
    ]);
    for drive in drives {
        table.add_row(vec![
            drive.drive_id.clone(),
            drive.enabled.to_string(),
            drive.status.to_string(),
            drive.path.clone(),
        ]);
    }
    table.print();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::client::MockIpcCaller;
    use serde_json::json;

    #[tokio::test]
    async fn test_list_command() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::DrivesList));
            json!([
                {
                    "drive_id": "test-drive",
                    "path": "/tmp/test",
                    "store_url": "http://localhost",
                    "enabled": true,
                    "status": "Idle"
                }
            ])
        });

        handle(&mock).await;
    }
}
