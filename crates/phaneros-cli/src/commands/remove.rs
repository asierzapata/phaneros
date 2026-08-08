use phaneros_ipc::methods::DriveIdParams;
use phaneros_ipc::Request;
use crate::commands::client::IpcCaller;

pub async fn handle(client: &impl IpcCaller, drive_id: String) {
    client
        .call(Request::DrivesRemove(DriveIdParams {
            drive_id: drive_id.clone(),
        }))
        .await;
    println!("Removed drive '{}'.", drive_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::client::MockIpcCaller;
    use serde_json::json;

    #[tokio::test]
    async fn test_remove_command() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::DrivesRemove(_)));
            json!(true)
        });

        handle(&mock, "test-drive".to_string()).await;
    }
}
