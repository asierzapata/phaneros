use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{services::blob::UploadTicket, state::AppState};

#[derive(serde::Deserialize)]
pub struct UploadRequestBody {
    size: i64,
}

pub async fn upload_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    Json(body): Json<UploadRequestBody>,
) -> Response {
    match state.blob_service.create_ticket(&hash, body.size).await {
        Ok(UploadTicket::Upload(ticket)) => (StatusCode::OK, Json(ticket)).into_response(),
        // Already stored, the client can skip the upload entirely.
        Ok(UploadTicket::AlreadyStored) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
