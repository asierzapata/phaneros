use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn upload_blob_direct(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    body: Bytes,
) -> StatusCode {
    match state.blob_service.put_bytes_direct(&hash, body).await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
