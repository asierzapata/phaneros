use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

pub async fn download_blob_bytes(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Response {
    match state.blob_service.get_bytes(&hash).await {
        Ok(Some(bytes)) => bytes.into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::NOT_IMPLEMENTED.into_response(),
    }
}
