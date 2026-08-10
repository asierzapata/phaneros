use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;
use thiserror::Error;

use std::sync::Arc;

use crate::blob_repository::InMemoryBlobRepository;
use crate::node_repository::{Hash, InMemoryNodeRepository};
use crate::scanner::Scanner;

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Path watch error: {0}")]
    PathWachError(#[from] notify::Error),
    #[error("Scanner error: {0}")]
    Scanner(#[from] crate::scanner::ScannerError),
}

#[derive(Error, Debug)]
pub enum RescanError {
    #[error("watcher is no longer running")]
    WatcherStopped,
    #[error("Scanner error: {0}")]
    Scanner(#[from] crate::scanner::ScannerError),
}

pub struct Watcher {
    pub scanner: Scanner,
}

enum WatchEvent {
    FsChanged,
    Rescan(Sender<Result<Hash, crate::scanner::ScannerError>>),
    Stop,
}

#[derive(Clone)]
pub struct RescanHandle {
    events: Sender<WatchEvent>,
}

impl RescanHandle {
    pub fn rescan(&self) -> Result<Hash, RescanError> {
        let (reply_tx, reply_rx) = channel();

        self.events
            .send(WatchEvent::Rescan(reply_tx))
            .map_err(|_| RescanError::WatcherStopped)?;

        reply_rx
            .recv()
            .map_err(|_| RescanError::WatcherStopped)?
            .map_err(RescanError::Scanner)
    }

    /// Tells the watcher thread to stop, which drops its filesystem watch
    /// and closes the root-hash channel the watcher forwards into. Needed
    /// for a clean shutdown: the thread otherwise blocks forever on its
    /// event channel (it holds one of its own senders via the debouncer
    /// callback, so it can never observe that channel as closed on its own).
    pub fn stop(&self) {
        let _ = self.events.send(WatchEvent::Stop);
    }
}

/// What `watch` hands to the caller: a receiver of root hashes (one per
/// completed rescan), the initial root hash, the stores the hashes resolve
/// against, and a handle to request a scan.
pub struct WatchHandle {
    pub root_hashes: Receiver<Hash>,
    pub initial_root_hash: Hash,
    pub node_repository: Arc<InMemoryNodeRepository>,
    pub blob_repository: Arc<InMemoryBlobRepository>,
    pub rescan: RescanHandle,
}

impl Watcher {
    pub fn new(path: String) -> Self {
        let scanner = Scanner::new(path, false);
        Watcher { scanner }
    }

    pub fn watch(mut self) -> Result<WatchHandle, WatcherError> {
        let (event_tx, event_rx) = channel::<WatchEvent>();
        let (watcher_tx, watcher_rx) = channel();

        let node_repository = self.scanner.get_store();
        let blob_repository = self.scanner.get_blob_repository().clone();

        let debouncer_tx = event_tx.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(300),
            None,
            move |result: DebounceEventResult| match result {
                Ok(_) => {
                    let _ = debouncer_tx.send(WatchEvent::FsChanged);
                }
                Err(errors) => errors.iter().for_each(|error| {
                    println!("Error: {:?}", error);
                }),
            },
        )?;

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

        let initial_root_hash_clone = initial_root_hash.clone();
        std::thread::spawn(move || {
            // Keep the debouncer alive for the lifetime of the watch loop.
            let _debouncer = debouncer;
            let mut last_root_hash = initial_root_hash_clone;

            for event in event_rx {
                match event {
                    WatchEvent::FsChanged => {
                        if let Ok(root_hash) = self.scanner.scan() {
                            if root_hash != last_root_hash {
                                println!("Folder tree updated, sending to syncer...");
                                last_root_hash = root_hash.clone();
                                if watcher_tx.send(root_hash).is_err() {
                                    // Nobody is syncing anymore.
                                    break;
                                }
                            }
                        }
                    }
                    WatchEvent::Rescan(reply) => {
                        let res = self.scanner.scan();
                        if let Ok(ref root_hash) = res {
                            last_root_hash = root_hash.clone();
                        }
                        let _ = reply.send(res);
                    }
                    WatchEvent::Stop => break,
                }
            }
        });

        Ok(WatchHandle {
            root_hashes: watcher_rx,
            initial_root_hash,
            node_repository,
            blob_repository,
            rescan: RescanHandle { events: event_tx },
        })
    }
}
