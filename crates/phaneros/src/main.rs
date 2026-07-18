use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use clap::Parser;

use phaneros::blob_repository::HttpBlobRepository;
use phaneros::node_repository::HttpNodeRepository;
use phaneros::syncer::Syncer;
use phaneros::watcher::Watcher;

/// A command-line utility for synchronizing files and directories across
/// multiple devices.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Directory to watch and sync
    #[arg(value_name = "PATH")]
    path: PathBuf,

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

    let watcher = Watcher::new(cli.path.to_string_lossy().into_owned());

    println!("Watcher started, waiting for changes...");

    // TODO: Handle the error properly instead of unwrapping.
    let (watcher_rx, initial_root_hash, local_node_repository, local_blob_repository) =
        watcher.watch().unwrap();

    let remote_node_repository = Arc::new(RwLock::new(HttpNodeRepository::new(
        // "http://localhost:8080".to_string(),
    )));

    let remote_blob_repository = Arc::new(RwLock::new(HttpBlobRepository::new(
        // "http://localhost:8080".to_string(),
    )));

    let mut syncer = Syncer::new(
        watcher_rx,
        initial_root_hash,
        local_node_repository,
        remote_node_repository,
        local_blob_repository,
        remote_blob_repository,
    );

    if let Some(dump_dir) = cli.dump_store {
        println!(
            "Dumping local store state to {}/ after each sync.",
            dump_dir.display()
        );
        syncer = syncer.with_store_dump(dump_dir);
    }

    syncer.run();
}
