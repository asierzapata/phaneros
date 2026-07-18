use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn get_node(
    State(state): State<AppState>,
    Path((drive_id, hash)): Path<(String, String)>,
) -> StatusCode {
    match state.node_service.get_node(&drive_id, &hash).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
