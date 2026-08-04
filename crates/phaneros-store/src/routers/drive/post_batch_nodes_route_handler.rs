use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use phaneros_sync::hash::Hash;
use phaneros_sync::node::Node;
use std::collections::HashMap;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct BatchNodesRequest {
    hashes: Vec<Hash>,
}

#[derive(Serialize)]
pub struct BatchNodesResponse {
    nodes: HashMap<Hash, Node>,
}

// TODO: migrate to QUERY method once axum supports it (tracking: axum#3801)
pub async fn post_batch_nodes(
    State(state): State<AppState>,
    Path(drive_id): Path<String>,
    Json(payload): Json<BatchNodesRequest>,
) -> Response {
    if payload.hashes.len() > 10_000 {
        return (StatusCode::BAD_REQUEST, "Too many hashes").into_response();
    }

    match state
        .node_service
        .get_nodes_batch(&drive_id, &payload.hashes)
        .await
    {
        Ok(nodes) => (StatusCode::OK, Json(BatchNodesResponse { nodes })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
