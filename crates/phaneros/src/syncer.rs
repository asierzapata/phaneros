use std::sync::{Arc, RwLock};

use crate::node_store::{Hash, InMemoryNodeStore};

pub struct Syncer {
    watcher_rx: std::sync::mpsc::Receiver<Hash>,
    initial_root_hash: Hash,
    node_store: Arc<RwLock<InMemoryNodeStore>>,
}

impl Syncer {
    pub fn new(
        watcher_rx: std::sync::mpsc::Receiver<Hash>,
        initial_root_hash: Hash,
        node_store: Arc<RwLock<InMemoryNodeStore>>,
    ) -> Self {
        Syncer {
            watcher_rx,
            initial_root_hash,
            node_store,
        }
    }

    pub fn run(&self) {
        println!(
            "Syncer started with initial root hash: {}",
            self.initial_root_hash
        );
        for updated_root_hash in &self.watcher_rx {
            let node_count = self.node_store.read().unwrap().len();
            println!(
                "Syncer received updated root hash: {} ({} nodes in store)",
                updated_root_hash, node_count
            );
        }
    }
}
