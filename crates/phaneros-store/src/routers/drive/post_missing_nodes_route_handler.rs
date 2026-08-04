use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use phaneros_sync::hash::Hash;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct MissingNodesRequest {
    hashes: Vec<Hash>,
}

#[derive(Serialize)]
pub struct MissingNodesResponse {
    missing: Vec<Hash>,
}

// TODO: migrate to QUERY method once axum supports it (tracking: axum#3801)
pub async fn post_missing_nodes(
    State(state): State<AppState>,
    Path(drive_id): Path<String>,
    Json(payload): Json<MissingNodesRequest>,
) -> Response {
    if payload.hashes.len() > 10_000 {
        return (StatusCode::BAD_REQUEST, "Too many hashes").into_response();
    }

    match state
        .node_service
        .get_missing_nodes(&drive_id, &payload.hashes)
        .await
    {
        Ok(missing) => (StatusCode::OK, Json(MissingNodesResponse { missing })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
