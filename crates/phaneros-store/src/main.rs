use std::sync::Arc;

use phaneros_store::{
    config::Config,
    db, routers,
    services::{
        blob::{BlobService, SqliteBlobMetadataRepository, UnimplementedBlobBytesRepository},
        node::{NodeService, SqliteNodeRepository},
    },
    state::AppState,
};

#[tokio::main]
async fn main() {
    let config = Config::load().expect("failed to load config");

    let pool = db::connect(&config.database_path)
        .await
        .expect("failed to open database");

    let state = AppState {
        node_service: NodeService::new(Arc::new(SqliteNodeRepository::new(pool.clone()))),
        blob_service: BlobService::new(
            Arc::new(SqliteBlobMetadataRepository::new(pool.clone())),
            Arc::new(UnimplementedBlobBytesRepository),
            config.public_url.clone(),
        ),
    };

    let app = routers::router(state);

    let listener = tokio::net::TcpListener::bind((config.host, config.port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
