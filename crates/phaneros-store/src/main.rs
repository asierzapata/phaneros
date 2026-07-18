use phaneros_store::{config::Config, routers};

#[tokio::main]
async fn main() {
    let config = Config::load().expect("failed to load config");

    let app = routers::router();

    let listener = tokio::net::TcpListener::bind((config.host, config.port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
