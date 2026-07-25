use crate::services::{blob::BlobService, node::NodeService, sync::SyncService};

#[derive(Clone)]
pub struct AppState {
    pub node_service: NodeService,
    pub blob_service: BlobService,
    pub sync_service: SyncService,
}
