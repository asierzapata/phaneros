mod commit_blob_route_handler;
mod download_blob_bytes_route_handler;
mod download_blob_route_handler;
mod head_or_get_blob_route_handler;
mod upload_blob_bytes_route_handler;
mod upload_blob_direct_route_handler;
mod upload_blob_route_handler;
mod post_missing_blobs_route_handler;

use axum::{
    Router,
    routing::{head, post, put},
};

use commit_blob_route_handler::commit_blob;
use download_blob_bytes_route_handler::download_blob_bytes;
use download_blob_route_handler::download_blob;
use head_or_get_blob_route_handler::head_or_get_blob;
use upload_blob_bytes_route_handler::upload_blob_bytes;
use upload_blob_direct_route_handler::upload_blob_direct;
use upload_blob_route_handler::upload_blob;
use post_missing_blobs_route_handler::post_missing_blobs;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // TODO: migrate to QUERY method once axum supports it (tracking: axum#3801)
        .route("/missing", post(post_missing_blobs))
        .route(
            "/{hash}",
            head(head_or_get_blob)
                .get(download_blob_bytes)
                .put(upload_blob_direct),
        )
        .route("/{hash}/upload", post(upload_blob))
        .route("/{hash}/commit", post(commit_blob))
        .route("/{hash}/download", post(download_blob))
        .route(
            "/{hash}/bytes",
            put(upload_blob_bytes).get(download_blob_bytes),
        )
}
