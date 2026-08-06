pub mod blob;
pub mod drive;

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::state::AppState;

/// Blobs are content-defined chunks capped at
/// `phaneros_core::scanner::file_chunker::DEFAULT_MAX_CHUNK_SIZE` (4 MB); 5 MB
/// leaves headroom for HTTP overhead while still rejecting anything
/// unexpectedly large.
const MAX_BLOB_UPLOAD_BYTES: usize = 5 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest(
            "/api",
            Router::new()
                .nest("/blobs", blob::router())
                .nest("/drives/{drive_id}", drive::router()),
        )
        .layer(
            TraceLayer::new_for_http()
                .on_request(())
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(DefaultBodyLimit::max(MAX_BLOB_UPLOAD_BYTES))
        .with_state(state)
}
