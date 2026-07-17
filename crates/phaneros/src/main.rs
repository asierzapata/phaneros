use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use clap::Parser;

use phaneros::blob_store::HttpBlobStore;
use phaneros::node_store::HttpNodeStore;
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
    let (watcher_rx, initial_root_hash, local_node_store, local_blob_store) =
        watcher.watch().unwrap();

    let remote_node_store = Arc::new(RwLock::new(HttpNodeStore::new(
        // "http://localhost:8080".to_string(),
    )));

    let remote_blob_store = Arc::new(RwLock::new(HttpBlobStore::new(
        // "http://localhost:8080".to_string(),
    )));

    let mut syncer = Syncer::new(
        watcher_rx,
        initial_root_hash,
        local_node_store,
        remote_node_store,
        local_blob_store,
        remote_blob_store,
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
