use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use phaneros_sync::node::NodeWire;

use crate::state::AppState;

pub async fn put_node(
    State(state): State<AppState>,
    Path((drive_id, route_hash)): Path<(String, String)>,
    Json(node_wire): Json<NodeWire>,
) -> StatusCode {
    let (body_hash, node) = node_wire.reconstruct();
    if body_hash != route_hash {
        return StatusCode::BAD_REQUEST;
    }
    match state
        .node_service
        .put_node(&drive_id, body_hash, node)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
