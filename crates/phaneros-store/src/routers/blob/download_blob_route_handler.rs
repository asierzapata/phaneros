use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn download_blob(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> StatusCode {
    // TODO: mint and return a real ticket once the data plane exists.
    match state.blob_service.exists(&hash).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
