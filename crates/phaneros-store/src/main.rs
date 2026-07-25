use std::{sync::Arc, time::Duration};

use phaneros_store::{
    config::Config,
    db, routers,
    services::{
        blob::{BlobService, FsBlobBytesRepository, SqliteBlobMetadataRepository},
        node::{NodeService, SqliteNodeRepository},
        sync::SyncService,
    },
    state::AppState,
};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() {
    fmt()
        .pretty()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")),
        )
        .init();

    let config = Config::load().expect("failed to load config");

    let pool = db::connect(&config.database_path)
        .await
        .expect("failed to open database");

    let state = AppState {
        node_service: NodeService::new(Arc::new(SqliteNodeRepository::new(pool.clone()))),
        blob_service: BlobService::new(
            Arc::new(SqliteBlobMetadataRepository::new(pool.clone())),
            Arc::new(FsBlobBytesRepository::new(config.blob_storage_path.clone())),
            config.public_url.clone(),
        ),
        sync_service: SyncService::default(),
    };

    let poller_node_service = state.node_service.clone();
    let poller_sync_service = state.sync_service.clone();
    tokio::spawn(async move {
        poller_sync_service
            .run_versions_poller(poller_node_service, Duration::from_millis(500), 500)
            .await;
    });

    let app = routers::router(state);

    let listener = tokio::net::TcpListener::bind((config.host, config.port))
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
