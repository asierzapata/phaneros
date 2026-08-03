use std::path::PathBuf;

use clap::Parser;
use phaneros_core::{EngineConfig, SyncEngine};

/// A command-line utility for synchronizing files and directories across
/// multiple devices.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
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
    let cli = Cli::parse();

    let config = EngineConfig::new(
        cli.path,
        cli.store_url,
        cli.drive_id,
        cli.token,
        cli.dump_store,
    );

    let engine = SyncEngine::new(config);
    if let Err(err) = engine.run() {
        eprintln!("Phaneros engine error: {err}");
        std::process::exit(1);
    }
}
