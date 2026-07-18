use axum::{extract::Path, http::StatusCode};

pub async fn upload_blob(Path(hash): Path<String>) -> StatusCode {
    let _ = hash;
    StatusCode::NOT_IMPLEMENTED
}
