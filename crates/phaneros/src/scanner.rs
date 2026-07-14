use rayon::prelude::*;
use std::{collections::HashMap, fs, sync::atomic::AtomicUsize};
use thiserror::Error;

use crate::folder_tree::{FolderTree, FolderTreeNode};
use crate::utils::observer::Publisher;

#[derive(Debug)]
pub enum ScannerStatus {
    Idle,          // The scanner is idle and not currently scanning
    Scanning,      // The scanner is currently scanning the path
    Error(String), // An error occurred during scanning, with an error message
}

/// Events that can be emitted by the scanner to notify observers of changes in its state or progress.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScannerEvent {
    ScanStarted,   // The scanner has started scanning the path
    ScanCompleted, // The scanner has completed scanning the path

    SyncStarted, // The scanner has started syncing the local representation with the remote representation
    SyncCompleted, // The scanner has completed syncing the local representation with the remote representation
    Error(String), // An error occurred during scanning or syncing, with an error message
}

impl Default for ScannerEvent {
    fn default() -> Self {
        ScannerEvent::ScanStarted
    }
}

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("IO error: {path}")]
    GetMetadataFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Error reading directory: {path}")]
    ReadDirFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Error reading file: {path}")]
    ReadFileFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Error hashing file: {path}")]
    HashFileFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Error hashing data: {path}")]
    HashDataFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Scanner is already scanning")]
    AlreadyScanning,

    #[error("Scanner failed to count files in path: {0}")]
    CountFilesFailed(String),
}

#[derive(Debug, Clone, PartialEq)]
struct MetadataKey(String);

impl MetadataKey {
    fn new(size: u64, last_modified: std::time::SystemTime) -> Self {
        let last_modified_timestamp = last_modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        MetadataKey(format!("{}-{}", size, last_modified_timestamp))
    }
}

/// Scanner is reponsible for maintaning a local representation of a given path and its contents for efficient change detection and reconciliation with a remote representation of the same path and its contents.
#[derive(Debug)]
pub struct Scanner {
    file_path: String,     // The path to the file or directory being scanned
    status: ScannerStatus, // The current status of the scanner
    publisher: Publisher<ScannerEvent>, // The publisher for scanner events
    file_count_estimate: usize, // An estimate of the number of files and directories in the path, used for progress reporting
    scan_progress: AtomicUsize, // The number of files and directories scanned so far, used for progress reporting
    last_scan_file_metadata_hash_map: HashMap<String, (MetadataKey, FolderTreeNode)>, // A map of file paths to their metadata keys and tree nodes from the last scan
    last_scan_time: Option<std::time::SystemTime>, // The time of the last scan
    last_scan_duration: Option<std::time::Duration>, // The duration of the last scan
    should_show_progress: bool,                    // Whether to show progress during scanning
}

impl Scanner {
    pub fn new(file_path: String, should_show_progress: bool) -> Scanner {
        Scanner {
            file_path,
            should_show_progress,
            status: ScannerStatus::Idle,
            publisher: Publisher::default(),
            file_count_estimate: 0,
            scan_progress: AtomicUsize::new(0),
            last_scan_file_metadata_hash_map: HashMap::new(),
            last_scan_time: None,
            last_scan_duration: None,
        }
    }

    pub fn events(&mut self) -> &mut Publisher<ScannerEvent> {
        &mut self.publisher
    }

    pub fn get_path(&self) -> &str {
        &self.file_path
    }

    pub fn scan(&mut self) -> Result<FolderTree, ScannerError> {
        if let ScannerStatus::Scanning = self.status {
            // If the scanner is already scanning, return early
            // TODO: Consider returning an error that a scan is already in progress
            return Err(ScannerError::AlreadyScanning);
        }
        self.status = ScannerStatus::Scanning;
        self.last_scan_time = Some(std::time::SystemTime::now());

        self.file_count_estimate = if self.last_scan_file_metadata_hash_map.is_empty() {
            match self.count_files_in_path(&self.file_path) {
                Ok(count) => count,
                Err(e) => {
                    println!("Error counting files in path {}: {}", self.file_path, e);
                    self.publisher.notify(
                        &ScannerEvent::Error(format!("Error counting files in path: {}", e)),
                        &self.file_path,
                    );
                    self.status =
                        ScannerStatus::Error(format!("Error counting files in path: {}", e));
                    return Err(ScannerError::CountFilesFailed(format!(
                        "Error counting files in path: {}",
                        e
                    )));
                }
            }
        } else {
            self.last_scan_file_metadata_hash_map.len()
        };

        self.publisher
            .notify(&ScannerEvent::ScanStarted, &self.file_path);

        let file_path = self.file_path.clone();

        let folder_tree = match self.scan_path(&file_path) {
            Ok((tree_node, metadata_keys)) => {
                self.last_scan_file_metadata_hash_map.clear();
                for (path, metadata_key, tree_node) in metadata_keys {
                    self.last_scan_file_metadata_hash_map
                        .insert(path, (metadata_key, tree_node));
                }
                FolderTree {
                    root_hash: tree_node.hash.clone(),
                    nodes: vec![tree_node],
                }
            }
            Err(e) => {
                println!("Error scanning path {}: {}", self.file_path, e);
                self.publisher.notify(
                    &ScannerEvent::Error(format!("Error scanning path: {}", e)),
                    &self.file_path,
                );
                self.status = ScannerStatus::Error(format!("Error scanning path: {}", e));
                return Err(e);
            }
        };

        self.last_scan_duration = self.last_scan_time.and_then(|start| start.elapsed().ok());
        println!(
            "Scan completed for path {} in {}. Root hash: {}. Number of files and directories scanned: {}",
            self.file_path,
            self.last_scan_duration.unwrap_or_default().as_secs_f64(),
            folder_tree.root_hash,
            self.last_scan_file_metadata_hash_map.len()
        );
        self.publisher
            .notify(&ScannerEvent::ScanCompleted, &self.file_path);

        self.status = ScannerStatus::Idle;

        Ok(folder_tree)
    }

    fn scan_path(
        &self,
        path: &str,
    ) -> Result<(FolderTreeNode, Vec<(String, MetadataKey, FolderTreeNode)>), ScannerError> {
        // We use fs::metadata to do a performant scan of the path, without reading
        // the entire contents into memory. We will only read files, and just to compute
        // their hashes if they have changed since the last scan based on their metadata.

        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(ScannerError::GetMetadataFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        if metadata.is_dir() {
            let entries = match fs::read_dir(path) {
                Ok(entries) => entries,
                Err(e) => {
                    return Err(ScannerError::ReadDirFailed {
                        path: path.to_string(),
                        source: e,
                    });
                }
            };

            let scan_path_results = entries
                .par_bridge()
                .filter(|entry| entry.is_ok())
                .map(|entry| {
                    let entry = entry.unwrap();
                    let entry_path = entry.path();
                    let entry_path_str = entry_path.to_string_lossy().to_string();
                    self.scan_path(&entry_path_str)
                })
                .collect::<Result<
                    Vec<(FolderTreeNode, Vec<(String, MetadataKey, FolderTreeNode)>)>,
                    ScannerError,
                >>()?;

            let (mut entries_tree_nodes, keys): (
                Vec<FolderTreeNode>,
                Vec<Vec<(String, MetadataKey, FolderTreeNode)>>,
            ) = scan_path_results.into_iter().par_bridge().unzip();
            let vector_of_metadata_keys: Vec<(String, MetadataKey, FolderTreeNode)> =
                keys.into_iter().flatten().par_bridge().collect();

            entries_tree_nodes.sort_by(|a, b| a.name.cmp(&b.name));

            let tree_node = FolderTreeNode {
                name: path.to_string(),
                hash: self.hash_iterator_of_data(entries_tree_nodes.iter().map(|node| &node.hash)),
                children: entries_tree_nodes,
            };

            return Ok((tree_node, vector_of_metadata_keys));
        } else if metadata.is_file() {
            let file_size = metadata.len();

            let last_time_modified = match metadata.modified() {
                Ok(time) => time,
                Err(e) => {
                    return Err(ScannerError::GetMetadataFailed {
                        path: path.to_string(),
                        source: e,
                    });
                }
            };

            let metadata_key = MetadataKey::new(file_size, last_time_modified);

            let last_scan_entry = self.last_scan_file_metadata_hash_map.get(path);

            let scan_response = match last_scan_entry {
                Some((last_metadata_key, metadata_key_tree_node)) => {
                    if *last_metadata_key == metadata_key {
                        (
                            metadata_key_tree_node.clone(),
                            vec![(
                                path.to_string(),
                                metadata_key.clone(),
                                metadata_key_tree_node.clone(),
                            )],
                        )
                    } else {
                        let tree_node = FolderTreeNode {
                            name: path.to_string(),
                            hash: self.hash_file(path)?,
                            children: Vec::new(),
                        };

                        (
                            tree_node.clone(),
                            vec![(path.to_string(), metadata_key.clone(), tree_node)],
                        )
                    }
                }
                None => {
                    let tree_node = FolderTreeNode {
                        name: path.to_string(),
                        hash: self.hash_file(path)?,
                        children: Vec::new(),
                    };

                    (
                        tree_node.clone(),
                        vec![(path.to_string(), metadata_key.clone(), tree_node)],
                    )
                }
            };

            self.notify_progress(path);

            return Ok(scan_response);
        } else {
            return Err(ScannerError::GetMetadataFailed {
                path: path.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Path is neither a file nor a directory",
                ),
            });
        }
    }

    fn notify_progress(&self, _path: &str) {
        if !self.should_show_progress {
            return;
        }

        let progress = self
            .scan_progress
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        let total = self.file_count_estimate;

        println!("Scanned {} of {} files and directories. ", progress, total,);
    }

    fn count_files_in_path(&self, path: &str) -> Result<usize, ScannerError> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(ScannerError::GetMetadataFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        if metadata.is_dir() {
            let entries = match fs::read_dir(path) {
                Ok(entries) => entries,
                Err(e) => {
                    return Err(ScannerError::ReadDirFailed {
                        path: path.to_string(),
                        source: e,
                    });
                }
            };

            let count = entries
                .par_bridge()
                .filter(|entry| entry.is_ok())
                .map(|entry| {
                    let entry = entry.unwrap();
                    let entry_path = entry.path();
                    let entry_path_str = entry_path.to_string_lossy().to_string();
                    self.count_files_in_path(&entry_path_str)
                })
                .collect::<Result<Vec<usize>, ScannerError>>()?
                .into_iter()
                .sum();

            return Ok(count);
        } else if metadata.is_file() {
            return Ok(1);
        } else {
            return Err(ScannerError::GetMetadataFailed {
                path: path.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Path is neither a file nor a directory",
                ),
            });
        }
    }

    fn hash_file(&self, path: &str) -> Result<String, ScannerError> {
        let file_contents = match fs::read(path) {
            Ok(contents) => contents,
            Err(e) => {
                println!("Error reading file {}: {}", path, e);
                self.publisher.notify(
                    &ScannerEvent::Error(format!("Error reading file: {}", e)),
                    path,
                );
                return Err(ScannerError::ReadFileFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        return Ok(self.hash_data(&file_contents));
    }

    fn hash_data(&self, data: &[u8]) -> String {
        return blake3::hash(data).to_hex().to_string();
    }

    fn hash_iterator_of_data<'a>(&self, hashes: impl Iterator<Item = &'a String>) -> String {
        let mut hasher = blake3::Hasher::new();
        for hash in hashes {
            hasher.update(hash.as_bytes());
        }
        return hasher.finalize().to_hex().to_string();
    }
}
