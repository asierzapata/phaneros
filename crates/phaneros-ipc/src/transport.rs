use std::path::Path;

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

use crate::methods::{Notification, Request};
use crate::protocol::{JSONRPC_VERSION, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

pub type IpcFramed = Framed<UnixStream, LinesCodec>;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Codec(#[from] LinesCodecError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("daemon closed the connection")]
    ConnectionClosed,
    #[error("daemon returned an error: {0}")]
    Remote(#[from] JsonRpcError),
}

/// Frames a raw `UnixStream` as newline-delimited JSON. Shared by both the
/// daemon's per-connection accept loop and [`IpcClient`].
pub fn frame(stream: UnixStream) -> IpcFramed {
    Framed::new(stream, LinesCodec::new())
}

/// A client connection to the daemon's control socket. Each call opens a
/// fresh logical request/response exchange over the same connection; the
/// CLI typically opens one `IpcClient` per invocation and drops it when done.
pub struct IpcClient {
    framed: IpcFramed,
    next_id: u64,
}

impl IpcClient {
    pub async fn connect(socket_path: &Path) -> Result<Self, IpcError> {
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self {
            framed: frame(stream),
            next_id: 1,
        })
    }

    /// Sends a request and waits for its matching response, ignoring any
    /// notifications that arrive first (only possible if `events.subscribe`
    /// was already called on this connection).
    pub async fn call(&mut self, request: Request) -> Result<Value, IpcError> {
        let id = json!(self.next_id);
        self.next_id += 1;

        let (method, params) = request.into_parts();
        let wire = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: Some(id.clone()),
            method,
            params,
        };
        self.framed.send(serde_json::to_string(&wire)?).await?;

        loop {
            let line = self
                .framed
                .next()
                .await
                .ok_or(IpcError::ConnectionClosed)??;
            let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) else {
                continue;
            };
            if response.id != id {
                continue;
            }
            return match response.error {
                Some(err) => Err(IpcError::Remote(err)),
                None => Ok(response.result.unwrap_or(Value::Null)),
            };
        }
    }

    /// Calls `events.subscribe`, then hands back `self` for reading a stream
    /// of [`Notification`]s via [`IpcClient::next_notification`].
    pub async fn subscribe(mut self) -> Result<Self, IpcError> {
        self.call(Request::EventsSubscribe).await?;
        Ok(self)
    }

    pub async fn next_notification(&mut self) -> Result<Option<Notification>, IpcError> {
        let Some(line) = self.framed.next().await else {
            return Ok(None);
        };
        let line = line?;
        let raw: JsonRpcRequest = serde_json::from_str(&line)?;
        Ok(Some(Notification::from_parts(&raw.method, raw.params)?))
    }
}
