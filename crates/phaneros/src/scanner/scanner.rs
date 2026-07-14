use rayon::prelude::*;
use std::path::PathBuf;
use std::{collections::HashMap, fs, sync::atomic::AtomicUsize};
use thiserror::Error;

use crate::folder_tree::{FileChunk, FileIndexTreeNode, FolderIndexTreeNode, IndexTree};
use crate::scanner::file_chunker::{FileChunker, FileChunkerError};
use crate::scanner::file_counter::FileCounter;
use crate::utils::observer::Publisher;

#[derive(Debug)]
pub enum ScannerStatus {
    Idle,     // The scanner is idle and not currently scanning
    Scanning, // The scanner is currently scanning the path
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

#[derive(Debug)]
enum SnapshotStatus {
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug)]
struct ScanSnapshot {
    started_at: std::time::SystemTime,
    completed_at: Option<std::time::SystemTime>,
    status: SnapshotStatus,
    file_count_estimate: usize,
    scan_progress: AtomicUsize,
    scanned_entries: HashMap<EntryKey, EntryRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntryKey {
    pub rel_path: PathBuf, // normalized path relative to root
}

#[derive(Debug)]
pub enum EntryRecord {
    Folder(FolderRecord),
    File(FileRecord),
}

#[derive(Debug)]
pub struct FolderRecord {
    pub metadata_hash: String,
}

#[derive(Debug)]
pub struct FileRecord {
    pub metadata_hash: String,
    pub content_hash: Option<String>,
    pub chunks: Vec<ChunkRecord>,
}

#[derive(Debug, Clone)]
pub struct ChunkRecord {
    pub index: usize,
    pub hash: String,
    pub size: u64,
}

pub struct ScanStats {
    pub files: u64,
    pub folders: u64,
    pub bytes_read: u64,
}

#[derive(Debug)]
enum ScannedEntry {
    Folder(FolderIndexTreeNode, HashMap<EntryKey, EntryRecord>),
    File(FileIndexTreeNode, (EntryKey, EntryRecord)),
}

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
    should_show_progress: bool, // Whether to show progress during scanning
    file_chunker: FileChunker, // The file chunker used for chunking files during scanning
    file_counter: FileCounter, // The file counter used for counting files in the path
    snapshot_buffer_size: usize, // The maximum number of scan snapshots to keep in memory
    scan_snapshots: Vec<ScanSnapshot>, // A list of the last scan snapshots, used for progress reporting and change detection
}

impl Scanner {
    pub fn new(file_path: String, should_show_progress: bool) -> Scanner {
        Scanner {
            file_path,
            should_show_progress,
            status: ScannerStatus::Idle,
            publisher: Publisher::default(),
            file_chunker: FileChunker::new(1024 * 1024), // 1 MB chunk size
            file_counter: FileCounter::new(),
            snapshot_buffer_size: 10, // Default snapshot buffer size
            scan_snapshots: Vec::new(),
        }
    }

    pub fn events(&mut self) -> &mut Publisher<ScannerEvent> {
        &mut self.publisher
    }

    pub fn get_path(&self) -> &str {
        &self.file_path
    }

    fn entry_key(&self, path: &str) -> EntryKey {
        let root_path = std::path::Path::new(&self.file_path);
        let entry_path = std::path::Path::new(path);

        let rel_path = entry_path
            .strip_prefix(root_path)
            .map(|rel| rel.to_path_buf())
            .unwrap_or_else(|_| entry_path.to_path_buf());

        EntryKey { rel_path }
    }

    pub fn scan(&mut self) -> Result<IndexTree, ScannerError> {
        if let ScannerStatus::Scanning = self.status {
            // If the scanner is already scanning, return early
            // TODO: Consider returning an error that a scan is already in progress
            return Err(ScannerError::AlreadyScanning);
        }
        self.status = ScannerStatus::Scanning;

        let is_first_scan = self.scan_snapshots.is_empty();
        let mut new_snapshot = ScanSnapshot {
            started_at: std::time::SystemTime::now(),
            completed_at: None,
            status: SnapshotStatus::InProgress,
            file_count_estimate: 0,
            scan_progress: AtomicUsize::new(0),
            scanned_entries: HashMap::new(),
        };

        new_snapshot.file_count_estimate = if is_first_scan {
            match self.file_counter.count_files_in_path(&self.file_path) {
                Ok(count) => count,
                Err(e) => {
                    println!("Error counting files in path {}: {}", self.file_path, e);
                    self.publisher.notify(
                        &ScannerEvent::Error(format!("Error counting files in path: {}", e)),
                        &self.file_path,
                    );
                    self.status = ScannerStatus::Idle;
                    return Err(ScannerError::CountFilesFailed(format!(
                        "Error counting files in path: {}",
                        e
                    )));
                }
            }
        } else {
            self.scan_snapshots
                .last()
                .map(|snapshot| snapshot.file_count_estimate)
                .unwrap_or(0)
        };

        self.publisher
            .notify(&ScannerEvent::ScanStarted, &self.file_path);

        let file_path = self.file_path.clone();

        let folder_tree = match self.scan_path(&file_path) {
            Ok((index_node, scanned_entries)) => {
                new_snapshot.scanned_entries = scanned_entries;
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
                self.status = ScannerStatus::Idle;
                return Err(e);
            }
        };

        new_snapshot.completed_at = Some(std::time::SystemTime::now());
        new_snapshot.status = SnapshotStatus::Completed;
        new_snapshot.scan_progress.store(
            new_snapshot.scanned_entries.len(),
            std::sync::atomic::Ordering::SeqCst,
        );
        println!(
            "Scan completed for path {} in {}. Root hash: {}. Number of files and directories scanned: {}",
            self.file_path,
            new_snapshot
                .completed_at
                .unwrap()
                .duration_since(new_snapshot.started_at)
                .unwrap_or_default()
                .as_secs(),
            folder_tree.root_hash,
            new_snapshot.scanned_entries.len()
        );
        self.publisher
            .notify(&ScannerEvent::ScanCompleted, &self.file_path);

        self.scan_snapshots.push(new_snapshot);
        if self.scan_snapshots.len() > self.snapshot_buffer_size {
            self.scan_snapshots.remove(0);
        }

        self.status = ScannerStatus::Idle;

        Ok(folder_tree)
    }

    fn scan_path(
        &self,
        path: &str,
    ) -> Result<(FolderIndexTreeNode, HashMap<EntryKey, EntryRecord>), ScannerError> {
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
                .map(|(index_node, scanned_entry)| {
                    let mut scanned_entries = HashMap::new();
                    scanned_entries.insert(scanned_entry.0, scanned_entry.1);

                    (
                        FolderIndexTreeNode::new(base_name(path), vec![], vec![index_node]),
                        scanned_entries,
                    )
                })
        } else {
            Err(ScannerError::GetMetadataFailed {
                path: path.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Path is neither a file nor a directory",
                ),
            })
        }
    }

    fn scan_directory(
        &self,
        path: &str,
    ) -> Result<(FolderIndexTreeNode, HashMap<EntryKey, EntryRecord>), ScannerError> {
        let read_dir = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ScannerError::ReadDirFailed {
                    path: path.to_string(),
                    source: e,
                });
            }
        };

        let entries =
            read_dir
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
                    let (index_node, child_entries) = self.scan_directory(&entry_path_str)?;
                    Ok(ScannedEntry::Folder(index_node, child_entries))
                } else if metadata.is_file() {
                    let (index_node, scanned_entry) = self.scan_file(&entry_path_str, &metadata)?;
                    Ok(ScannedEntry::File(index_node, scanned_entry))
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
        let mut scanned_entry_map: HashMap<EntryKey, EntryRecord> = HashMap::new();

        for scanned_entry in scanned_entries {
            match scanned_entry {
                ScannedEntry::Folder(index_node, child_entries) => {
                    folder_index_nodes.push(index_node);
                    scanned_entry_map.extend(child_entries);
                }
                ScannedEntry::File(index_node, (entry_key, entry_record)) => {
                    file_index_nodes.push(index_node);
                    scanned_entry_map.insert(entry_key, entry_record);
                }
            }
        }

        folder_index_nodes.sort_by(|a, b| a.name.cmp(&b.name));
        file_index_nodes.sort_by(|a, b| a.name.cmp(&b.name));

        let folder_index_node = FolderIndexTreeNode::from_children(
            base_name(path),
            folder_index_nodes,
            file_index_nodes,
        );

        scanned_entry_map.insert(
            self.entry_key(path),
            EntryRecord::Folder(FolderRecord {
                metadata_hash: folder_index_node.hash.clone(),
            }),
        );

        Ok((folder_index_node, scanned_entry_map))
    }

    fn scan_file(
        &self,
        path: &str,
        metadata: &fs::Metadata,
    ) -> Result<(FileIndexTreeNode, (EntryKey, EntryRecord)), ScannerError> {
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
        let entry_key = self.entry_key(path);

        let last_scan_entry = self
            .scan_snapshots
            .last()
            .and_then(|snapshot| snapshot.scanned_entries.get(&entry_key));

        let file_name = base_name(path);

        let index_node = match last_scan_entry {
            Some(EntryRecord::File(last_file_record))
                if last_file_record.metadata_hash == metadata_key.0 =>
            {
                let mut sorted_chunks = last_file_record.chunks.clone();
                sorted_chunks.sort_by_key(|chunk| chunk.index);

                let chunks: Vec<FileChunk> = sorted_chunks
                    .into_iter()
                    .map(|chunk| FileChunk {
                        hash: chunk.hash,
                        size: chunk.size,
                    })
                    .collect();

                let hash = last_file_record.content_hash.clone().unwrap_or_else(|| {
                    FileIndexTreeNode::from_chunks(file_name.clone(), chunks.clone()).hash
                });

                FileIndexTreeNode {
                    name: file_name.clone(),
                    hash,
                    chunks,
                }
            }
            _ => {
                let file_chunks = self.file_chunker.chunk_file(path)?;
                FileIndexTreeNode::from_chunks(file_name.clone(), file_chunks)
            }
        };

        let entry_record = EntryRecord::File(FileRecord {
            metadata_hash: metadata_key.0,
            content_hash: Some(index_node.hash.clone()),
            chunks: index_node
                .chunks
                .iter()
                .enumerate()
                .map(|(index, chunk)| ChunkRecord {
                    index,
                    hash: chunk.hash.clone(),
                    size: chunk.size,
                })
                .collect(),
        });

        self.notify_progress(path);

        Ok((index_node, (entry_key, entry_record)))
    }

    fn notify_progress(&self, _path: &str) {
        if !self.should_show_progress {
            return;
        }

        let progress = self
            .scan_snapshots
            .last()
            .map(|snapshot| {
                snapshot
                    .scan_progress
                    .load(std::sync::atomic::Ordering::SeqCst)
            })
            .unwrap_or(0);
        let total = self
            .scan_snapshots
            .last()
            .map(|snapshot| snapshot.file_count_estimate)
            .unwrap_or(0);

        println!("Scanned {} of {} files and directories. ", progress, total,);
    }
}
