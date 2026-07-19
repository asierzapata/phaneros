use std::sync::Arc;

use phaneros_store::{
    config::Config,
    db, routers,
    services::{
        blob::{BlobService, FsBlobBytesRepository, SqliteBlobMetadataRepository},
        node::{NodeService, SqliteNodeRepository},
    },
    state::AppState,
};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() {
    fmt()
        .pretty()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug")),
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
    };

    let app = routers::router(state);

    let listener = tokio::net::TcpListener::bind((config.host, config.port))
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
