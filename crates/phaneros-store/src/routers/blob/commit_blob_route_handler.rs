use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::{services::blob::BlobServiceError, state::AppState};

pub async fn commit_blob(State(state): State<AppState>, Path(hash): Path<String>) -> StatusCode {
    match state.blob_service.confirm_upload(&hash).await {
        Ok(()) => StatusCode::NO_CONTENT,
        // No ticket was ever minted for this hash so nothing to commit.
        Err(BlobServiceError::Unregistered) => StatusCode::NOT_FOUND,
        // Ticket exists but the bytes never arrived.
        Err(BlobServiceError::BytesMissing) => StatusCode::CONFLICT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
