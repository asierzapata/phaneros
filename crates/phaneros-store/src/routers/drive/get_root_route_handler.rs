use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct GetRootResponse {
    hash: String,
}

pub async fn get_root(State(state): State<AppState>, Path(drive_id): Path<String>) -> Response {
    match state.node_service.get_root(&drive_id).await {
        Ok(Some(root_node)) => (
            StatusCode::OK,
            axum::Json(GetRootResponse { hash: root_node }),
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
