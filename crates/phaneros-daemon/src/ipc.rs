use std::path::{Path, PathBuf};

use futures::{SinkExt, StreamExt};
use phaneros_ipc::protocol::{JSONRPC_VERSION, JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use phaneros_ipc::{Notification, Request, frame};
use serde_json::Value;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::state::Command;

#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("failed to bind IPC socket at {path}: {source}")]
    Bind {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Binds the control-plane unix socket, removing a stale socket file left
/// behind by a daemon that didn't shut down cleanly.
pub async fn bind(socket_path: &Path) -> Result<UnixListener, ListenError> {
    if let Some(parent) = socket_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match UnixListener::bind(socket_path) {
        Ok(listener) => Ok(listener),
        Err(_) => {
            // Bind failed, possibly because a previous daemon left its socket
            // file behind. Remove it and retry once; if another daemon is
            // actually still listening, the retry will fail too (address in
            // use), which is the correct outcome.
            let _ = std::fs::remove_file(socket_path);
            UnixListener::bind(socket_path).map_err(|source| ListenError::Bind {
                path: socket_path.to_path_buf(),
                source,
            })
        }
    }
}

pub async fn serve(
    listener: UnixListener,
    command_tx: mpsc::Sender<Command>,
    broadcast_tx: broadcast::Sender<Notification>,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _addr)) => {
                        let command_tx = command_tx.clone();
                        let broadcast_tx = broadcast_tx.clone();
                        let cancel = cancel.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, command_tx, broadcast_tx, cancel).await;
                        });
                    }
                    Err(err) => {
                        tracing::warn!("failed to accept IPC connection: {}", err);
                    }
                }
            }
            _ = cancel.cancelled() => break,
        }
    }
}

async fn handle_connection(
    stream: UnixStream,
    command_tx: mpsc::Sender<Command>,
    broadcast_tx: broadcast::Sender<Notification>,
    cancel: CancellationToken,
) {
    let mut framed = frame(stream);
    let mut broadcast_rx = broadcast_tx.subscribe();
    let mut subscribed = false;

    loop {
        tokio::select! {
            maybe_line = framed.next() => {
                let Some(Ok(line)) = maybe_line else { break; };
                let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&line) else { continue; };
                if request.is_notification() {
                    continue;
                }
                let id = request.id.clone().unwrap();

                let response = match Request::from_parts(&request.method, request.params.clone()) {
                    Ok(Request::EventsSubscribe) => {
                        subscribed = true;
                        JsonRpcResponse::success(id, Value::Object(Default::default()))
                    }
                    Ok(parsed) => match dispatch(parsed, &command_tx).await {
                        Ok(value) => JsonRpcResponse::success(id, value),
                        Err(err) => JsonRpcResponse::failure(id, err),
                    },
                    Err(_) => JsonRpcResponse::failure(
                        id,
                        JsonRpcError::new(
                            JsonRpcError::METHOD_NOT_FOUND,
                            format!("unknown method '{}'", request.method),
                        ),
                    ),
                };

                let Ok(text) = serde_json::to_string(&response) else { break; };
                if framed.send(text).await.is_err() {
                    break;
                }
            }
            notification = broadcast_rx.recv(), if subscribed => {
                match notification {
                    Ok(notif) => {
                        let (method, params) = notif.into_parts();
                        let wire = JsonRpcRequest {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            id: None,
                            method,
                            params,
                        };
                        let Ok(text) = serde_json::to_string(&wire) else { continue; };
                        if framed.send(text).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = cancel.cancelled() => break,
        }
    }
}

async fn dispatch(request: Request, command_tx: &mpsc::Sender<Command>) -> Result<Value, JsonRpcError> {
    let (reply_tx, reply_rx) = oneshot::channel();

    let command = match request {
        Request::DaemonPing => Command::Ping(reply_tx),
        Request::DaemonShutdown => Command::Shutdown(reply_tx),
        Request::DrivesList => Command::ListDrives(reply_tx),
        Request::DrivesStatus(p) => Command::DriveStatus {
            drive_id: p.drive_id,
            reply: reply_tx,
        },
        Request::DrivesStart(p) => Command::StartDrive {
            drive_id: p.drive_id,
            reply: reply_tx,
        },
        Request::DrivesStop(p) => Command::StopDrive {
            drive_id: p.drive_id,
            reply: reply_tx,
        },
        Request::DrivesAdd(params) => Command::AddDrive {
            params,
            reply: reply_tx,
        },
        Request::DrivesRemove(p) => Command::RemoveDrive {
            drive_id: p.drive_id,
            reply: reply_tx,
        },
        Request::DrivesTriggerSync(p) => Command::TriggerSync {
            drive_id: p.drive_id,
            reply: reply_tx,
        },
        Request::ConfigReload => Command::ReloadConfig(reply_tx),
        Request::StatsAggregate(p) => Command::StatsAggregate {
            drive_id: p.drive_id,
            reply: reply_tx,
        },
        Request::EventsSubscribe => {
            unreachable!("events.subscribe is handled before dispatch")
        }
    };

    command_tx.send(command).await.map_err(|_| {
        JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, "daemon actor is not running")
    })?;

    reply_rx.await.map_err(|_| {
        JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, "daemon actor dropped the reply")
    })?
}
