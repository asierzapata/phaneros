mod get_node_route_handler;
mod get_root_route_handler;
mod get_versions_route_handler;
mod put_node_route_handler;
mod put_root_route_handler;

use axum::{Router, routing::get};

use get_node_route_handler::get_node;
use get_root_route_handler::get_root;
use get_versions_route_handler::get_versions;
use put_node_route_handler::put_node;
use put_root_route_handler::put_root;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/root", get(get_root).put(put_root))
        .route("/nodes/{hash}", get(get_node).put(put_node))
        .route("/versions", get(get_versions))
}
