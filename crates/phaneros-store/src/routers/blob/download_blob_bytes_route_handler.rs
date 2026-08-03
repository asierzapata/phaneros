use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::state::AppState;

pub async fn download_blob_bytes(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Response {
    let metadata = match state.blob_service.get_metadata(&hash).await {
        Ok(Some(meta)) => meta,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match state.blob_service.get_bytes(&hash).await {
        Ok(Some(bytes)) => {
            let mut headers = HeaderMap::new();
            if metadata.compression == "zstd" {
                headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("zstd"));
            }
            (headers, bytes).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
