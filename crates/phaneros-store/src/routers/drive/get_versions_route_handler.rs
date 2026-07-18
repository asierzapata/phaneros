use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn get_versions(
    State(state): State<AppState>,
    Path(drive_id): Path<String>,
) -> StatusCode {
    match state.node_service.list_versions(&drive_id).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
