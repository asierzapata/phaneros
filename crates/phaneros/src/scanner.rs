use rayon::prelude::*;
use std::io::Read;
use std::{collections::HashMap, fs, sync::atomic::AtomicUsize};
use thiserror::Error;

use crate::folder_tree::{FileChunk, FileIndexTreeNode, FolderIndexTreeNode, IndexTree};
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

    #[error("Chunking file failed: {0}")]
    ChunkingFileFailed(#[from] FileChunkerError),
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

/// What a single `read_dir` entry turned into, so that a parallel scan of a directory
/// can be partitioned back into the two homogeneous lists a folder node is built from.
enum ScannedEntry {
    Folder(
        FolderIndexTreeNode,
        Vec<(String, MetadataKey, FileIndexTreeNode)>,
    ),
    File(FileIndexTreeNode, (String, MetadataKey, FileIndexTreeNode)),
}

/// The name a node is known by inside its parent. Never the full path: a node's hash
/// must describe its contents, not where the tree happens to be mounted.
fn base_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Scanner is reponsible for maintaning a local representation of a given path and its contents for efficient change detection and reconciliation with a remote representation of the same path and its contents.
#[derive(Debug)]
pub struct Scanner {
    file_path: String,     // The path to the file or directory being scanned
    status: ScannerStatus, // The current status of the scanner
    publisher: Publisher<ScannerEvent>, // The publisher for scanner events
    file_count_estimate: usize, // An estimate of the number of files and directories in the path, used for progress reporting
    scan_progress: AtomicUsize, // The number of files and directories scanned so far, used for progress reporting
    last_scan_file_metadata_hash_map: HashMap<String, (MetadataKey, FileIndexTreeNode)>, // A map of file paths to their metadata keys and tree nodes from the last scan
    last_scan_time: Option<std::time::SystemTime>, // The time of the last scan
    last_scan_duration: Option<std::time::Duration>, // The duration of the last scan
    should_show_progress: bool,                    // Whether to show progress during scanning
    file_chunker: FileChunker, // The file chunker used for chunking files during scanning
    file_counter: FileCounter, // The file counter used for counting files in the path
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
            file_chunker: FileChunker::new(1024 * 1024), // 1 MB chunk size
            file_counter: FileCounter::new(),
        }
    }

    pub fn events(&mut self) -> &mut Publisher<ScannerEvent> {
        &mut self.publisher
    }

    pub fn get_path(&self) -> &str {
        &self.file_path
    }

    pub fn scan(&mut self) -> Result<IndexTree, ScannerError> {
        if let ScannerStatus::Scanning = self.status {
            // If the scanner is already scanning, return early
            // TODO: Consider returning an error that a scan is already in progress
            return Err(ScannerError::AlreadyScanning);
        }
        self.status = ScannerStatus::Scanning;
        self.last_scan_time = Some(std::time::SystemTime::now());

        self.file_count_estimate = if self.last_scan_file_metadata_hash_map.is_empty() {
            match self.file_counter.count_files_in_path(&self.file_path) {
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
            Ok((index_node, metadata_keys)) => {
                self.last_scan_file_metadata_hash_map.clear();
                for (path, metadata_key, index_node) in metadata_keys {
                    self.last_scan_file_metadata_hash_map
                        .insert(path, (metadata_key, index_node));
                }
                // The root node's own name is deliberately dropped: a node's name belongs
                // to its parent, and the root has no parent. Its hash is unaffected by it.
                IndexTree {
                    root_hash: index_node.hash,
                    folders: index_node.folders,
                    files: index_node.files,
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
    ) -> Result<
        (
            FolderIndexTreeNode,
            Vec<(String, MetadataKey, FileIndexTreeNode)>,
        ),
        ScannerError,
    > {
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
            self.scan_directory(path)
        } else if metadata.is_file() {
            self.scan_file(path, &metadata)
                .map(|(index_node, metadata_key)| {
                    (
                        FolderIndexTreeNode::new(base_name(path), vec![], vec![index_node]),
                        vec![metadata_key],
                    )
                })
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

    fn scan_directory(
        &self,
        path: &str,
    ) -> Result<
        (
            FolderIndexTreeNode,
            Vec<(String, MetadataKey, FileIndexTreeNode)>,
        ),
        ScannerError,
    > {
        let read_dir = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ScannerError::ReadDirFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        let entries = read_dir
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ScannerError::ReadDirFailed {
                path: path.to_string(),
                source: e,
            })?;

        let scanned_entries = entries
            .par_iter()
            .map(|entry| {
                let entry_path = entry.path();
                let entry_path_str = entry_path.to_string_lossy().to_string();

                // fs::metadata resolves symlinks, so a symlink to a file is scanned as
                // the file it points at.
                let metadata =
                    fs::metadata(&entry_path_str).map_err(|e| ScannerError::GetMetadataFailed {
                        path: entry_path_str.clone(),
                        source: e,
                    })?;

                if metadata.is_dir() {
                    let (index_node, metadata_keys) = self.scan_directory(&entry_path_str)?;
                    Ok(ScannedEntry::Folder(index_node, metadata_keys))
                } else if metadata.is_file() {
                    let (index_node, metadata_key) = self.scan_file(&entry_path_str, &metadata)?;
                    Ok(ScannedEntry::File(index_node, metadata_key))
                } else {
                    Err(ScannerError::GetMetadataFailed {
                        path: entry_path_str.clone(),
                        source: std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Path is neither a file nor a directory",
                        ),
                    })
                }
            })
            .collect::<Result<Vec<ScannedEntry>, ScannerError>>()?;

        let mut folder_index_nodes = Vec::new();
        let mut file_index_nodes = Vec::new();
        let mut metadata_keys = Vec::new();

        for scanned_entry in scanned_entries {
            match scanned_entry {
                ScannedEntry::Folder(index_node, entry_metadata_keys) => {
                    folder_index_nodes.push(index_node);
                    metadata_keys.extend(entry_metadata_keys);
                }
                ScannedEntry::File(index_node, metadata_key) => {
                    file_index_nodes.push(index_node);
                    metadata_keys.push(metadata_key);
                }
            }
        }

        // read_dir yields entries in an arbitrary order, so both lists have to be
        // sorted for the folder hash to be stable across scans and across machines.
        folder_index_nodes.sort_by(|a, b| a.name.cmp(&b.name));
        file_index_nodes.sort_by(|a, b| a.name.cmp(&b.name));

        Ok((
            FolderIndexTreeNode::from_children(base_name(path), folder_index_nodes, file_index_nodes),
            metadata_keys,
        ))
    }

    fn scan_file(
        &self,
        path: &str,
        metadata: &fs::Metadata,
    ) -> Result<(FileIndexTreeNode, (String, MetadataKey, FileIndexTreeNode)), ScannerError> {
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
            Some((last_metadata_key, metadata_key_index_node)) => {
                if *last_metadata_key == metadata_key {
                    (
                        metadata_key_index_node.clone(),
                        (
                            path.to_string(),
                            metadata_key.clone(),
                            metadata_key_index_node.clone(),
                        ),
                    )
                } else {
                    let file_chunks = self.file_chunker.chunk_file(path)?;
                    let index_node =
                        FileIndexTreeNode::from_chunks(base_name(path), file_chunks);

                    (
                        index_node.clone(),
                        (path.to_string(), metadata_key.clone(), index_node),
                    )
                }
            }
            None => {
                let file_chunks = self.file_chunker.chunk_file(path)?;
                let file_name = match std::path::Path::new(path).file_name() {
                    Some(name) => name.to_string_lossy().to_string(),
                    None => path.to_string(),
                };

                let index_node = FileIndexTreeNode::from_chunks(file_name, file_chunks);

                (
                    index_node.clone(),
                    (path.to_string(), metadata_key.clone(), index_node),
                )
            }
        };

        self.notify_progress(path);

        Ok(scan_response)
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
}

#[derive(Error, Debug)]
enum FileCounterError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
struct FileCounter {
    count: AtomicUsize,
}

impl FileCounter {
    fn new() -> Self {
        FileCounter {
            count: AtomicUsize::new(0),
        }
    }

    fn increment(&self) {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn get_count(&self) -> usize {
        self.count.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn count_files_in_path(&self, path: &str) -> Result<usize, FileCounterError> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(FileCounterError::IoError(e));
            }
        };

        if metadata.is_dir() {
            let entries = match fs::read_dir(path) {
                Ok(entries) => entries,
                Err(e) => {
                    return Err(FileCounterError::IoError(e));
                }
            };

            for entry in entries {
                let entry = entry.map_err(FileCounterError::IoError)?;
                let entry_path = entry.path();
                let entry_path_str = entry_path.to_string_lossy().to_string();
                self.count_files_in_path(&entry_path_str)?;
            }
        } else if metadata.is_file() {
            self.increment();
        }

        Ok(self.get_count())
    }
}

#[derive(Error, Debug)]
enum FileChunkerError {
    #[error("Error reading file: {path}")]
    ReadFileFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug)]
struct FileChunker {
    chunk_size: usize,
}

impl FileChunker {
    fn new(chunk_size: usize) -> Self {
        FileChunker { chunk_size }
    }

    fn chunk_file(&self, path: &str) -> Result<Vec<FileChunk>, FileChunkerError> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) => {
                println!("Error opening file {}: {}", path, e);
                return Err(FileChunkerError::ReadFileFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        let mut reader = std::io::BufReader::new(file);
        let mut buffer = vec![0; self.chunk_size];
        let mut chunks = Vec::new();

        loop {
            let bytes_read = match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    println!("Error reading file {}: {}", path, e);
                    return Err(FileChunkerError::ReadFileFailed {
                        path: path.to_string(),
                        source: e,
                    });
                }
            };

            chunks.push(buffer[..bytes_read].to_vec());
        }

        let chunk_index_nodes = chunks
            .into_iter()
            .enumerate()
            .map(|(_, bytes)| FileChunk::from_bytes(&bytes))
            .collect();

        Ok(chunk_index_nodes)
    }
}
