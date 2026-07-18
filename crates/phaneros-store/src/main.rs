use std::sync::Arc;

use phaneros_store::{
    config::Config,
    routers,
    services::{
        blob::{BlobService, UnimplementedBlobBytesRepository, UnimplementedBlobMetadataRepository},
        node::{NodeService, UnimplementedNodeRepository},
    },
    state::AppState,
};

#[tokio::main]
async fn main() {
    let config = Config::load().expect("failed to load config");

    let state = AppState {
        node_service: NodeService::new(Arc::new(UnimplementedNodeRepository)),
        blob_service: BlobService::new(
            Arc::new(UnimplementedBlobMetadataRepository),
            Arc::new(UnimplementedBlobBytesRepository),
        ),
    };

    let app = routers::router(state);

    let listener = tokio::net::TcpListener::bind((config.host, config.port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
