use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use clap::Parser;

use phaneros::blob_repository::HttpBlobRepository;
use phaneros::node_repository::HttpNodeRepository;
use phaneros::syncer::Syncer;
use phaneros::syncer::sync_state::DriveSession;
use phaneros::watcher::{WatchHandle, Watcher, spawn_remote_listener};

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

    let drive_session = DriveSession::open(&cli.drive_id, &cli.path)
        .expect("failed to initialize sync state session for this drive/path");

    let watcher = Watcher::new(cli.path.to_string_lossy().into_owned());

    println!("Watcher started, waiting for changes...");

    // TODO: Handle the error properly instead of unwrapping.
    let watch_handle = watcher.watch().unwrap();

    let WatchHandle {
        root_hashes: watcher_root_hashes,
        initial_root_hash,
        node_repository: local_node_repository,
        blob_repository: local_blob_repository,
        rescan,
    } = watch_handle;

    let (sync_trigger_tx, sync_trigger_rx) = std::sync::mpsc::channel();

    let watcher_forward_tx = sync_trigger_tx.clone();
    std::thread::spawn(move || {
        for root_hash in watcher_root_hashes {
            if watcher_forward_tx.send(root_hash).is_err() {
                break;
            }
        }
    });

    let remote_node_repository = Arc::new(RwLock::new(HttpNodeRepository::new(
        &cli.store_url,
        &cli.drive_id,
        &cli.token,
    )));

    let remote_rescan = rescan.clone();
    let remote_trigger_tx = sync_trigger_tx.clone();
    let _remote_listener = spawn_remote_listener(
        cli.store_url.clone(),
        cli.drive_id.clone(),
        cli.token.clone(),
        move |_event| match remote_rescan.rescan() {
            Ok(root_hash) => {
                let _ = remote_trigger_tx.send(root_hash);
            }
            Err(err) => {
                eprintln!("Failed to rescan after remote root-changed event: {err}");
            }
        },
    );

    let remote_blob_repository = Arc::new(RwLock::new(HttpBlobRepository::new(
        &cli.store_url,
        &cli.drive_id,
        &cli.token,
    )));

    let mut syncer = Syncer::new(
        sync_trigger_rx,
        initial_root_hash,
        local_node_repository,
        remote_node_repository,
        local_blob_repository,
        remote_blob_repository,
        drive_session,
        rescan,
    );

    drop(sync_trigger_tx);

    if let Some(dump_dir) = cli.dump_store {
        println!(
            "Dumping local store state to {}/ after each sync.",
            dump_dir.display()
        );
        syncer = syncer.with_store_dump(dump_dir);
    }

    syncer.run();
}
