use axum::{extract::Path, http::StatusCode};

pub async fn download_blob_bytes(Path(hash): Path<String>) -> StatusCode {
    let _ = hash;
    StatusCode::NOT_IMPLEMENTED
}
