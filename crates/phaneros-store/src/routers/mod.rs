pub mod blob;
pub mod drive;

use axum::Router;

pub fn router() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .nest("/blobs", blob::router())
            .nest("/drives/{drive_id}", drive::router()),
    )
}
