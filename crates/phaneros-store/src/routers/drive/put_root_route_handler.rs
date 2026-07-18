use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::state::AppState;

pub async fn put_root(State(state): State<AppState>, Path(drive_id): Path<String>) -> StatusCode {
    // TODO: extract {hash, expected} from the request body once the wire format is wired up.
    match state.node_service.put_root(&drive_id, String::new(), None).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_IMPLEMENTED,
    }
}
