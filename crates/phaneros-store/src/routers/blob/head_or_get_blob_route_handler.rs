use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn head_or_get_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> StatusCode {
    match state.blob_service.exists(&hash).await {
        Ok(true) => StatusCode::OK,
        Ok(false) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
