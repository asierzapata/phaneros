use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

pub async fn download_blob(State(state): State<AppState>, Path(hash): Path<String>) -> Response {
    match state.blob_service.create_download_ticket(&hash).await {
        Ok(Some(ticket)) => (StatusCode::OK, Json(ticket)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
