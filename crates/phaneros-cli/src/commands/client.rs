use std::path::{Path, PathBuf};
use phaneros_ipc::{IpcClient, IpcError, Request};
use serde_json::Value;

#[allow(async_fn_in_trait)]
pub trait IpcCaller: Send + Sync {
    async fn call(&self, request: Request) -> Value;
}

pub struct SocketIpcCaller<'a> {
    pub socket_path: &'a Path,
}

impl<'a> SocketIpcCaller<'a> {
    pub fn new(socket_path: &'a Path) -> Self {
        Self { socket_path }
    }
}

impl<'a> IpcCaller for SocketIpcCaller<'a> {
    async fn call(&self, request: Request) -> Value {
        call(self.socket_path, request).await
    }
}

#[cfg(test)]
pub struct MockIpcCaller<F> {
    handler: F,
}

#[cfg(test)]
impl<F> MockIpcCaller<F>
where
    F: Fn(Request) -> Value + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[cfg(test)]
impl<F> IpcCaller for MockIpcCaller<F>
where
    F: Fn(Request) -> Value + Send + Sync,
{
    async fn call(&self, request: Request) -> Value {
        (self.handler)(request)
    }
}

pub fn default_socket_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("phaneros").join("phaneros.sock"))
}

pub async fn connect(socket_path: &Path) -> IpcClient {
    match IpcClient::connect(socket_path).await {
        Ok(client) => client,
        Err(err) => {
            eprintln!(
                "Could not connect to the phaneros daemon at {} ({}).",
                socket_path.display(),
                err
            );
            eprintln!(
                "Is `phanerosd` running? Start it, or pass --socket to point at a different instance."
            );
            std::process::exit(1);
        }
    }
}

/// Connects fresh, makes one request, and exits the process with a clear
/// error message on any transport or daemon-side failure.
pub async fn call(socket_path: &Path, request: Request) -> Value {
    let mut client = connect(socket_path).await;
    match client.call(request).await {
        Ok(value) => value,
        Err(IpcError::Remote(err)) => {
            eprintln!("Error: {}", err.message);
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("Error talking to phanerosd: {}", err);
            std::process::exit(1);
        }
    }
}

pub fn expect_value<T: serde::de::DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("Received an unexpected response from phanerosd: {}", err);
            std::process::exit(1);
        }
    }
}
