use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
};

use crate::state::AppState;

pub async fn upload_blob_direct(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let compression = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .filter(|v| *v == "zstd")
        .map(|v| v.to_string());

    let uncompressed_size = headers
        .get("x-uncompressed-size")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());

    match state
        .blob_service
        .put_bytes_direct(&hash, body, uncompressed_size, compression)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
