use axum::{extract::Path, http::StatusCode};

pub async fn head_or_get_blob(Path(hash): Path<String>) -> StatusCode {
    let _ = hash;
    StatusCode::NOT_IMPLEMENTED
}
