use std::path::PathBuf;

use clap::Parser;
use phaneros_core::{EngineConfig, SyncEngine};

/// Phaneros background synchronization daemon.
#[derive(Parser)]
#[command(version, about)]
struct DaemonCli {
    /// Directory to watch and sync
    #[arg(value_name = "PATH")]
    path: PathBuf,

    /// Base URL of the remote phaneros-store (e.g. http://localhost:8080)
    #[arg(long, default_value = "http://localhost:8080")]
    store_url: String,

    /// Drive identifier on the remote store
    #[arg(long, default_value = "default")]
    drive_id: String,

    /// Bearer token for authenticating with the remote store
    #[arg(long, default_value = "")]
    token: String,

    /// Debug: dump the local store state to DIR/local_store_dump.txt after
    /// every sync
    #[arg(
        long,
        value_name = "DIR",
        num_args = 0..=1,
        default_missing_value = "target"
    )]
    dump_store: Option<PathBuf>,
}

fn main() {
    let args = DaemonCli::parse();

    println!("Starting Phaneros daemon (phanerosd)...");

    let config = EngineConfig::new(
        args.path,
        args.store_url,
        args.drive_id,
        args.token,
        args.dump_store,
    );

    let engine = SyncEngine::new(config);
    if let Err(err) = engine.run() {
        eprintln!("Phaneros daemon error: {err}");
        std::process::exit(1);
    }
}
