use std::path::PathBuf;
use phaneros_ipc::methods::AddDriveParams;
use phaneros_ipc::Request;
use crate::commands::client::IpcCaller;

pub fn build_add_drive_params(
    drive_id: String,
    path: PathBuf,
    store_url: Option<String>,
    token: Option<String>,
    disabled: bool,
) -> Result<AddDriveParams, phaneros_ipc::DriveIdError> {
    let drive_id = phaneros_ipc::normalize_drive_id(&drive_id)?;
    Ok(AddDriveParams {
        drive_id,
        path: path.to_string_lossy().to_string(),
        token,
        store_url,
        enabled: !disabled,
    })
}

pub async fn handle(
    client: &impl IpcCaller,
    drive_id: String,
    path: PathBuf,
    store_url: Option<String>,
    token: Option<String>,
    disabled: bool,
) {
    let params = build_add_drive_params(drive_id, path, store_url, token, disabled)
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            std::process::exit(1);
        });
    let drive_id = params.drive_id.clone();
    client.call(Request::DrivesAdd(params)).await;
    println!("Added drive '{}'.", drive_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::client::MockIpcCaller;
    use serde_json::json;

    #[tokio::test]
    async fn test_add_command() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::DrivesAdd(_)));
            json!(true)
        });

        handle(
            &mock,
            "my_drive".to_string(),
            PathBuf::from("/tmp/docs"),
            None,
            None,
            false,
        )
        .await;
    }
}
