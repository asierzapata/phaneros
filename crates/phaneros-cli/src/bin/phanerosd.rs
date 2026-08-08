#[tokio::main]
async fn main() {
    phaneros_daemon::run_daemon().await;
}
