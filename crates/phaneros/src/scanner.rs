use std::{collections::HashMap, fs};
use thiserror::Error;

use crate::utils::observer::Publisher;

/// A merkle tree representation of a file or directory, where each node is a hash of its contents and its children.
#[derive(Debug)]
pub struct Tree {
    root_hash: String,    // The root hash of the tree
    nodes: Vec<TreeNode>, // The nodes of the tree
}

/// A node in the merkle tree, representing a file or directory and its hash.
#[derive(Debug, Clone)]
pub struct TreeNode {
    name: String, // The path of the file or directory represented by the node. Just for debugging, delete later
    hash: String, // The hash of the node
    children: Vec<TreeNode>, // The children of the node
}

#[derive(Debug)]
pub enum ScannerStatus {
    Idle,          // The scanner is idle and not currently scanning
    Scanning,      // The scanner is currently scanning the path
    Syncing, // The scanner is currently syncing the local representation with the remote representation
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
}

#[derive(Debug, PartialEq)]
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
    tree: Tree,            // The tree representing the contents of the file or directory
    publisher: Publisher<ScannerEvent>, // The publisher for scanner events
    last_scan_path_metadata_hash_map: HashMap<String, (MetadataKey, TreeNode)>, // A map of the last scan's path metadata keys and their corresponding tree nodes
    last_scan_time: Option<std::time::SystemTime>, // The time of the last scan
    last_scan_duration: Option<std::time::Duration>, // The duration of the last scan
}

impl Scanner {
    pub fn new(file_path: String) -> Self {
        Scanner {
            file_path,
            status: ScannerStatus::Idle,
            tree: Tree {
                root_hash: String::new(),
                nodes: Vec::new(),
            },
            publisher: Publisher::default(),
            last_scan_path_metadata_hash_map: HashMap::new(),
            last_scan_time: None,
            last_scan_duration: None,
        }
    }

    pub fn events(&mut self) -> &mut Publisher<ScannerEvent> {
        &mut self.publisher
    }

    pub fn scan(&mut self) {
        if let ScannerStatus::Scanning = self.status {
            // If the scanner is already scanning, return early
            // TODO: Consider returning an error that a scan is already in progress
            return;
        }
        self.status = ScannerStatus::Scanning;
        self.last_scan_time = Some(std::time::SystemTime::now());

        self.publisher
            .notify(&ScannerEvent::ScanStarted, &self.file_path);

        let file_path = self.file_path.clone();

        match self.scan_path(&file_path) {
            Ok(tree_node) => {
                self.tree.root_hash = tree_node.hash.clone();
                self.tree.nodes = vec![tree_node];
            }
            Err(e) => {
                println!("Error scanning path {}: {}", self.file_path, e);
                self.publisher.notify(
                    &ScannerEvent::Error(format!("Error scanning path: {}", e)),
                    &self.file_path,
                );
                self.status = ScannerStatus::Error(format!("Error scanning path: {}", e));
                return;
            }
        }

        println!("{:#?}", self.tree);

        self.last_scan_duration = self.last_scan_time.and_then(|start| start.elapsed().ok());
        println!(
            "Scan completed for path {} in {}. Root hash: {}. Number of files and directories scanned: {}",
            self.file_path,
            self.last_scan_duration.unwrap_or_default().as_secs_f64(),
            self.tree.root_hash,
            self.last_scan_path_metadata_hash_map.len()
        );
        self.publisher
            .notify(&ScannerEvent::ScanCompleted, &self.file_path);

        self.status = ScannerStatus::Idle;
    }

    fn scan_path(&mut self, path: &str) -> Result<TreeNode, ScannerError> {
        // We use fs::metadata to do a performant scan of the path, without reading
        // the entire contents into memory. We will only read files, and just to compute
        // their hashes if they have changed since the last scan, based on their metadata.

        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(e) => {
                println!("Error reading metadata for {}: {}", path, e);
                self.publisher.notify(
                    &ScannerEvent::Error(format!("Error reading metadata: {}", e)),
                    path,
                );
                self.status = ScannerStatus::Error(format!("Error reading metadata: {}", e));
                return Err(ScannerError::GetMetadataFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        let file_size = metadata.len();

        let last_time_modified = match metadata.modified() {
            Ok(time) => time,
            Err(e) => {
                println!("Error getting modified time for {}: {}", path, e);
                self.publisher.notify(
                    &ScannerEvent::Error(format!("Error getting modified time: {}", e)),
                    path,
                );
                self.status = ScannerStatus::Error(format!("Error getting modified time: {}", e));
                return Err(ScannerError::GetMetadataFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        let metadata_key = MetadataKey::new(file_size, last_time_modified);

        if let Some((last_metadata_key, metadata_key_tree_node)) =
            self.last_scan_path_metadata_hash_map.get(path)
        {
            if *last_metadata_key == metadata_key {
                // If the metadata has not changed since the last scan, we can skip this path
                return Ok(metadata_key_tree_node.clone());
            }
        }

        if metadata.is_dir() {
            // If the path is a directory, we will scan its contents recursively
            let entries = match fs::read_dir(path) {
                Ok(entries) => entries,
                Err(e) => {
                    println!("Error reading directory {}: {}", path, e);
                    self.publisher.notify(
                        &ScannerEvent::Error(format!("Error reading directory: {}", e)),
                        path,
                    );
                    self.status = ScannerStatus::Error(format!("Error reading directory: {}", e));
                    return Err(ScannerError::ReadDirFailed {
                        path: path.to_string(),
                        source: e,
                    });
                }
            };

            let entries_tree_nodes = entries
                .filter(|entry| entry.is_ok())
                .map(|entry| {
                    let entry = entry.unwrap();
                    let entry_path = entry.path();
                    let entry_path_str = entry_path.to_string_lossy().to_string();
                    self.scan_path(&entry_path_str)
                })
                .collect::<Result<Vec<TreeNode>, ScannerError>>()?;

            let tree_node = TreeNode {
                name: path.to_string(),
                hash: self.hash_vector_of_hashes(
                    &entries_tree_nodes
                        .iter()
                        .map(|node| node.hash.clone())
                        .collect::<Vec<String>>(),
                ),
                children: entries_tree_nodes.clone(),
            };

            self.last_scan_path_metadata_hash_map
                .insert(path.to_string(), (metadata_key, tree_node.clone()));

            return Ok(tree_node);
        } else if metadata.is_file() {
            // If the path is a file, we will compute its hash and update the tree
            let tree_node = TreeNode {
                name: path.to_string(),
                hash: self.hash_file(path)?,
                children: Vec::new(),
            };

            self.last_scan_path_metadata_hash_map
                .insert(path.to_string(), (metadata_key, tree_node.clone()));

            return Ok(tree_node);
        } else {
            // If the path is neither a file nor a directory, we will return an error
            self.publisher.notify(
                &ScannerEvent::Error(format!("Path is neither a file nor a directory: {}", path)),
                path,
            );
            self.status =
                ScannerStatus::Error(format!("Path is neither a file nor a directory: {}", path));
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

        return Ok(self.hash_vector(&file_contents));
    }

    fn hash_vector(&self, data: &[u8]) -> String {
        return blake3::hash(data).to_hex().to_string();
    }

    fn hash_vector_of_hashes(&self, hashes: &[String]) -> String {
        let mut concatenated_hashes = Vec::new();
        for hash in hashes {
            concatenated_hashes.extend_from_slice(hash.as_bytes());
        }
        return self.hash_vector(&concatenated_hashes);
    }
}
