pub mod blob;
pub mod drive;

use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest(
            "/api",
            Router::new()
                .nest("/blobs", blob::router())
                .nest("/drives/{drive_id}", drive::router()),
        )
        .with_state(state)
}
