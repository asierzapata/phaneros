use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{services::node::Version, state::AppState};

#[derive(serde::Serialize)]
pub struct VersionWire {
    root: String,
    at: i64,
}

impl From<Version> for VersionWire {
    fn from(v: Version) -> Self {
        Self {
            root: v.root,
            at: v.at,
        }
    }
}

#[derive(serde::Serialize)]
pub struct VersionsResponse {
    pub versions: Vec<VersionWire>,
}

pub async fn get_versions(State(state): State<AppState>, Path(drive_id): Path<String>) -> Response {
    match state.node_service.list_versions(&drive_id).await {
        Ok(versions) => (
            StatusCode::OK,
            Json(VersionsResponse {
                versions: versions.into_iter().map(VersionWire::from).collect(),
            }),
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
