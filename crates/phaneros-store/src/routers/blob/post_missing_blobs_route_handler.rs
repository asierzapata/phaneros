use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use phaneros_sync::hash::Hash;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct MissingBlobsRequest {
    hashes: Vec<Hash>,
}

#[derive(Serialize)]
pub struct MissingBlobsResponse {
    missing: Vec<Hash>,
}

// TODO: migrate to QUERY method once axum supports it (tracking: axum#3801)
pub async fn post_missing_blobs(
    State(state): State<AppState>,
    Json(payload): Json<MissingBlobsRequest>,
) -> Response {
    if payload.hashes.len() > 10_000 {
        return (StatusCode::BAD_REQUEST, "Too many hashes").into_response();
    }

    match state.blob_service.get_missing(&payload.hashes).await {
        Ok(missing) => (StatusCode::OK, Json(MissingBlobsResponse { missing })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
