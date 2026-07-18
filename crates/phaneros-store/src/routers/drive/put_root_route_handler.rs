use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{services::node::NodeRepositoryError::RootMismatch, state::AppState};

#[derive(serde::Deserialize)]
pub struct PutRootRequestBody {
    hash: String,
    expected: Option<String>,
}

#[derive(serde::Serialize)]
pub struct PutRootConflictResponse {
    hash: String,
}

pub async fn put_root(
    State(state): State<AppState>,
    Path(drive_id): Path<String>,
    Json(body): Json<PutRootRequestBody>,
) -> Response {
    let PutRootRequestBody { hash, expected } = body;
    match state.node_service.put_root(&drive_id, hash, expected).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(RootMismatch {
            expected: _,
            actual,
        }) => (
            StatusCode::CONFLICT,
            Json(PutRootConflictResponse { hash: actual }),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
