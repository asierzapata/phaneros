mod get_events_route_handler;
mod get_node_route_handler;
mod get_root_route_handler;
mod get_versions_route_handler;
mod put_node_route_handler;
mod put_root_route_handler;
mod post_missing_nodes_route_handler;
mod post_batch_nodes_route_handler;

use axum::{Router, routing::{get, post}};

use get_events_route_handler::get_events;
use get_node_route_handler::get_node;
use get_root_route_handler::get_root;
use get_versions_route_handler::get_versions;
use put_node_route_handler::put_node;
use put_root_route_handler::put_root;
use post_missing_nodes_route_handler::post_missing_nodes;
use post_batch_nodes_route_handler::post_batch_nodes;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/root", get(get_root).put(put_root))
        .route("/nodes/{hash}", get(get_node).put(put_node))
        // TODO: migrate to QUERY method once axum supports it (tracking: axum#3801)
        .route("/nodes/missing", post(post_missing_nodes))
        // TODO: migrate to QUERY method once axum supports it (tracking: axum#3801)
        .route("/nodes/batch", post(post_batch_nodes))
        .route("/versions", get(get_versions))
        .route("/events", get(get_events))
}
