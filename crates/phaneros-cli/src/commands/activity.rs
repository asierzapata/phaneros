use phaneros_ipc::methods::ActivityListParams;
use phaneros_ipc::Request;
use serde_json::Value;
use crate::commands::client::IpcCaller;
use crate::ui;

pub async fn handle(client: &impl IpcCaller, drive_id: Option<String>, limit: usize, json: bool) {
    let value = client
        .call(Request::ActivityList(ActivityListParams {
            drive_id: drive_id.clone(),
            limit,
        }))
        .await;
    if json {
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    } else {
        render(&value);
    }
}

fn render(value: &Value) {
    let sessions = value.as_array().cloned().unwrap_or_default();
    if sessions.is_empty() {
        println!("No sync activity recorded.");
        return;
    }
    let mut table = ui::Table::with_columns(vec![
        ui::Column::new("DRIVE"),
        ui::Column::new("TIMESTAMP"),
        ui::Column::right_aligned("WIRE BYTES"),
        ui::Column::right_aligned("AVG SPEED"),
    ]);
    for session in &sessions {
        let drive_id = session["drive_id"].as_str().unwrap_or("?");
        let timestamp = session["timestamp_epoch_sec"].as_u64().unwrap_or(0);
        let wire_bytes = session["transfer"]["wire_bytes_sent"].as_u64().unwrap_or(0);
        let avg_speed = session["avg_upload_rate_bps"].as_u64().unwrap_or(0);
        table.add_row(vec![
            drive_id.to_string(),
            ui::format_timestamp(timestamp),
            ui::format_bytes(wire_bytes),
            ui::format_speed(avg_speed),
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
    async fn test_activity_command() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::ActivityList(_)));
            json!([
                {
                    "drive_id": "notes",
                    "timestamp_epoch_sec": 1786210714,
                    "transfer": { "wire_bytes_sent": 2048 },
                    "avg_upload_rate_bps": 1024
                }
            ])
        });

        handle(&mock, Some("notes".to_string()), 20, false).await;
    }
}
