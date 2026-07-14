use notify::RecursiveMode;
use notify_debouncer_full::new_debouncer;
use std::sync::mpsc::{Receiver, channel};
use std::{path::Path, time::Duration};
use thiserror::Error;

use crate::folder_tree::IndexTree;
use crate::scanner::scanner::Scanner;

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Path watch error: {0}")]
    PathWachError(#[from] notify::Error),
    #[error("Scanner error: {0}")]
    Scanner(#[from] crate::scanner::scanner::ScannerError),
}

pub struct Watcher {
    pub scanner: Scanner,
}

impl Watcher {
    pub fn new(path: String) -> Self {
        let scanner = Scanner::new(path, false);
        Watcher { scanner }
    }

    pub fn watch(mut self) -> Result<(Receiver<IndexTree>, IndexTree), WatcherError> {
        let (notify_tx, notify_rx) = channel();
        let (watcher_tx, watcher_rx) = channel();

        let mut debouncer = new_debouncer(Duration::from_secs(5), None, notify_tx)?;

        let scanner_path = self.scanner.get_path();
        let path = Path::new(&scanner_path);
        let debounce_watch_result = debouncer.watch(&path, RecursiveMode::Recursive);

        if let Err(error) = debounce_watch_result {
            println!("Error watching path: {:?}", error);
            return Err(WatcherError::PathWachError(error).into());
        }

        // We do a first scan to return alongside the watcher receiver, so the caller can have an initial state of the folder tree.
        let scanner_results = self.scanner.scan();
        let initial_folder_tree = match scanner_results {
            Ok(folder_tree) => folder_tree,
            Err(error) => {
                println!("Error scanning path: {:?}", error);
                return Err(WatcherError::Scanner(error).into());
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
                        if let Ok(folder_tree) = scanner_results {
                            println!("Folder tree updated, sending to syncer...");
                            watcher_tx.send(folder_tree).unwrap();
                        }
                    }
                    Err(errors) => errors.iter().for_each(|error| {
                        println!("Error: {:?}", error);
                    }),
                }
            }
        });

        Ok((watcher_rx, initial_folder_tree))
    }
}
