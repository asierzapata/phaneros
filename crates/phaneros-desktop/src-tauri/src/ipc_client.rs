use std::path::PathBuf;

use phaneros_ipc::{IpcClient, IpcError, Request};
use serde_json::Value;

/// Default control-plane socket path for a locally running `phanerosd`,
/// matching `phaneros-cli`'s default (`$XDG_DATA_HOME/phaneros/phaneros.sock`
/// or platform equivalent).
pub fn default_socket_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("phaneros").join("phaneros.sock"))
}

/// Connects fresh and makes one request, mirroring `phaneros-cli`'s
/// per-invocation connection pattern. Connection failures (daemon not
/// running) and remote RPC errors are both surfaced as a plain `String` so
/// Tauri commands can hand them straight to the frontend.
pub async fn call(request: Request) -> Result<Value, String> {
    let socket_path = default_socket_path()
        .ok_or_else(|| "Could not determine the phaneros daemon socket path".to_string())?;

    let mut client = IpcClient::connect(&socket_path).await.map_err(|err| {
        format!(
            "Could not connect to the phaneros daemon at {} ({}). Is `phanerosd` running?",
            socket_path.display(),
            err
        )
    })?;

    client.call(request).await.map_err(|err| match err {
        IpcError::Remote(remote) => remote.message,
        other => other.to_string(),
    })
}
