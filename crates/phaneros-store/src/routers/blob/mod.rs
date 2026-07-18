mod download_blob_bytes_route_handler;
mod download_blob_route_handler;
mod head_or_get_blob_route_handler;
mod upload_blob_bytes_route_handler;
mod upload_blob_route_handler;

use axum::{
    Router,
    routing::{head, post, put},
};

use download_blob_bytes_route_handler::download_blob_bytes;
use download_blob_route_handler::download_blob;
use head_or_get_blob_route_handler::head_or_get_blob;
use upload_blob_bytes_route_handler::upload_blob_bytes;
use upload_blob_route_handler::upload_blob;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{hash}", head(head_or_get_blob))
        .route("/{hash}/upload", post(upload_blob))
        .route("/{hash}/download", post(download_blob))
        .route(
            "/{hash}/bytes",
            put(upload_blob_bytes).get(download_blob_bytes),
        )
}
