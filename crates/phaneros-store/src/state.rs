use crate::services::{blob::BlobService, node::NodeService};

#[derive(Clone)]
pub struct AppState {
    pub node_service: NodeService,
    pub blob_service: BlobService,
}
