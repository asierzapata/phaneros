use std::sync::{Arc, RwLock};

use phaneros::blob_store::HttpBlobStore;
use phaneros::node_store::HttpNodeStore;
use phaneros::syncer::Syncer;
use phaneros::watcher::Watcher;

fn main() {
    let watcher = Watcher::new(String::from(
        "/Users/asierzapata/Documents/Projects/phaneros/documentation",
    ));

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

    let syncer = Syncer::new(
        watcher_rx,
        initial_root_hash,
        local_node_store,
        remote_node_store,
        local_blob_store,
        remote_blob_store,
    );

    syncer.run();
}
