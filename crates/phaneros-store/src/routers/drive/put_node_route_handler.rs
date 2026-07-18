use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn put_node(
    State(state): State<AppState>,
    Path((drive_id, hash)): Path<(String, String)>,
) -> StatusCode {
    // TODO: extract the JSON node from the request body once the wire format is wired up.
    match state
        .node_service
        .put_node(&drive_id, hash, phaneros_sync::node::Node::folder(vec![], vec![]).1)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
