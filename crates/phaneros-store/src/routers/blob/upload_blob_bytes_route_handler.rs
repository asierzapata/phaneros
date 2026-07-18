use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn upload_blob_bytes(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    body: Bytes,
) -> StatusCode {
    match state.blob_service.put_bytes(&hash, body).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
