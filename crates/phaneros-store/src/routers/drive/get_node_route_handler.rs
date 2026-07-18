use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

pub async fn get_node(
    State(state): State<AppState>,
    Path((drive_id, hash)): Path<(String, String)>,
) -> Response {
    match state.node_service.get_node(&drive_id, &hash).await {
        Ok(Some(node)) => (StatusCode::OK, Json(node)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
