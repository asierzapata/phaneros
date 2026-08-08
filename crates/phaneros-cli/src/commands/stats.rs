use phaneros_ipc::methods::StatsParams;
use phaneros_ipc::Request;
use serde_json::Value;
use crate::commands::client::IpcCaller;
use crate::ui;

pub async fn handle(client: &impl IpcCaller, drive_id: Option<String>, json: bool) {
    let value = client
        .call(Request::StatsAggregate(StatsParams {
            drive_id: drive_id.clone(),
        }))
        .await;
    if json {
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    } else {
        render(drive_id.as_deref(), &value);
    }
}

fn render(drive_id: Option<&str>, value: &Value) {
    let total_syncs = value["total_syncs"].as_u64().unwrap_or(0);
    let total_raw_bytes = value["total_raw_bytes"].as_u64().unwrap_or(0);
    let total_wire_bytes = value["total_wire_bytes"].as_u64().unwrap_or(0);
    let total_dedup_bytes = value["total_dedup_bytes"].as_u64().unwrap_or(0);
    let compression_pct = value["overall_compression_ratio_pct"]
        .as_f64()
        .unwrap_or(0.0);
    let avg_speed = value["avg_upload_rate_bps"].as_u64().unwrap_or(0);

    ui::render_view_header("Phaneros Sync Telemetry Insights");
    let mut items = Vec::new();
    if let Some(did) = drive_id {
        items.push(("Filter Drive ID", did.to_string()));
    }
    items.push(("Total Sync Sessions", total_syncs.to_string()));
    items.push(("Logical Data Processed", ui::format_bytes(total_raw_bytes)));
    items.push(("Wire Bytes Transferred", ui::format_bytes(total_wire_bytes)));
    items.push(("Deduplicated Bytes Saved", ui::format_bytes(total_dedup_bytes)));
    items.push(("Compression Savings", format!("{:.2}%", compression_pct)));
    items.push(("Average Upload Speed", ui::format_speed(avg_speed)));

    ui::render_key_values(&items.iter().map(|(k, v)| (*k, v.clone())).collect::<Vec<_>>());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::client::MockIpcCaller;
    use serde_json::json;

    #[tokio::test]
    async fn test_stats_command() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::StatsAggregate(_)));
            json!({
                "total_syncs": 42,
                "total_raw_bytes": 1048576,
                "total_wire_bytes": 524288,
                "total_dedup_bytes": 262144,
                "overall_compression_ratio_pct": 50.0,
                "avg_upload_rate_bps": 1024
            })
        });

        handle(&mock, None, false).await;
    }
}
