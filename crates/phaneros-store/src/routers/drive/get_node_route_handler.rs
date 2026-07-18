use axum::{extract::Path, http::StatusCode};

pub async fn get_node(Path((drive_id, hash)): Path<(String, String)>) -> StatusCode {
    let _ = (drive_id, hash);
    StatusCode::NOT_IMPLEMENTED
}
