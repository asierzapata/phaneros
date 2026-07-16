use notify::RecursiveMode;
use notify_debouncer_full::new_debouncer;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;
use thiserror::Error;

use std::sync::{Arc, RwLock};

use crate::blob_store::InMemoryBlobStore;
use crate::node_store::{Hash, InMemoryNodeStore};
use crate::scanner::Scanner;

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Path watch error: {0}")]
    PathWachError(#[from] notify::Error),
    #[error("Scanner error: {0}")]
    Scanner(#[from] crate::scanner::ScannerError),
}

pub struct Watcher {
    pub scanner: Scanner,
}

/// What `watch` hands to the caller: a receiver of root hashes (one per
/// completed rescan), the initial root hash, and the node store the hashes
/// resolve against.
pub type WatchHandle = (
    Receiver<Hash>,
    Hash,
    Arc<RwLock<InMemoryNodeStore>>,
    Arc<RwLock<InMemoryBlobStore>>,
);

impl Watcher {
    pub fn new(path: String) -> Self {
        let scanner = Scanner::new(path, false);
        Watcher { scanner }
    }

    pub fn watch(mut self) -> Result<WatchHandle, WatcherError> {
        let (notify_tx, notify_rx) = channel();
        let (watcher_tx, watcher_rx) = channel();

        let node_store = self.scanner.get_store();
        let blob_store = self.scanner.get_blob_store().clone();

        let mut debouncer = new_debouncer(Duration::from_secs(5), None, notify_tx)?;

        let path = self.scanner.get_path().to_path_buf();
        let debounce_watch_result = debouncer.watch(&path, RecursiveMode::Recursive);

        if let Err(error) = debounce_watch_result {
            println!("Error watching path: {:?}", error);
            return Err(WatcherError::PathWachError(error));
        }

        // We do a first scan to return alongside the watcher receiver, so the caller can have an initial state of the folder tree.
        let scanner_results = self.scanner.scan();
        let initial_root_hash = match scanner_results {
            Ok(root_hash) => root_hash,
            Err(error) => {
                println!("Error scanning path: {:?}", error);
                return Err(WatcherError::Scanner(error));
            }
        };

        std::thread::spawn(move || {
            // Keep the debouncer alive for the lifetime of the watch loop.
            let _debouncer = debouncer;

            for result in notify_rx {
                match result {
                    Ok(_) => {
                        let scanner_results = self.scanner.scan();
                        // TODO: What we do with the error here? Right now we drop it
                        if let Ok(root_hash) = scanner_results {
                            println!("Folder tree updated, sending to syncer...");
                            watcher_tx.send(root_hash).unwrap();
                        }
                    }
                    Err(errors) => errors.iter().for_each(|error| {
                        println!("Error: {:?}", error);
                    }),
                }
            }
        });

        Ok((watcher_rx, initial_root_hash, node_store, blob_store))
    }
}
