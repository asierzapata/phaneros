use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{services::blob::BlobServiceError, state::AppState};

pub async fn upload_blob_bytes(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    body: Bytes,
) -> StatusCode {
    match state.blob_service.put_bytes(&hash, body).await {
        Ok(()) => StatusCode::NO_CONTENT,
        // Client-caused: bytes with no ticket, or bytes that don't match the
        // declared size.
        Err(BlobServiceError::Unregistered | BlobServiceError::SizeMismatch { .. }) => {
            StatusCode::BAD_REQUEST
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
