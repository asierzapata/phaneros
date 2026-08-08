use std::path::Path;
use phaneros_ipc::Notification;
use crate::commands::client::connect;

pub async fn handle(socket_path: &Path, drive_id_filter: Option<String>) {
    let client = connect(socket_path).await;
    let mut client = match client.subscribe().await {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Failed to subscribe to daemon events: {}", err);
            std::process::exit(1);
        }
    };

    println!("Watching for sync events... (Ctrl-C to stop)");
    loop {
        match client.next_notification().await {
            Ok(Some(Notification::EventProgress { drive_id, event })) => {
                if drive_id_filter.as_deref().is_some_and(|f| f != drive_id) {
                    continue;
                }
                println!(
                    "[{}] {:?} - {}/{} blobs, {} B sent",
                    drive_id,
                    event.phase,
                    event.blobs_completed,
                    event.blobs_total,
                    event.bytes_sent
                );
            }
            Ok(Some(Notification::EventDriveStatusChanged { drive_id, status })) => {
                if drive_id_filter.as_deref().is_some_and(|f| f != drive_id) {
                    continue;
                }
                println!("[{}] status changed: {}", drive_id, status);
            }
            Ok(None) => {
                println!("Connection closed by daemon.");
                break;
            }
            Err(err) => {
                eprintln!("Error reading events: {}", err);
                break;
            }
        }
    }
}
