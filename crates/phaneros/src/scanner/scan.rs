use rayon::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use std::{fs, time::SystemTime};
use thiserror::Error;

use crate::blob_store::InMemoryBlobStore;
use crate::node_store::{Entry, Hash, InMemoryNodeStore, Node, NodeStore};
use crate::scanner::file_chunker::{FileChunker, FileChunkerError};
use crate::utils::observer::Publisher;

#[derive(Debug)]
pub enum ScannerStatus {
    Idle,
    Scanning,
}

/// Events that can be emitted by the scanner to notify observers of changes in its state or progress.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScannerEvent {
    ScanStarted,
    ScanCompleted,
    SyncStarted,
    SyncCompleted,
    Error(String),
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

    #[error("Chunking file failed: {0}")]
    ChunkingFileFailed(#[from] FileChunkerError),
}

#[derive(Debug, Clone, PartialEq)]
struct MetadataKey(String);

impl MetadataKey {
    fn new(metadata: &fs::Metadata) -> Self {
        let size = metadata.len();
        let last_modified_timestamp = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        MetadataKey(format!("{}-{}", size, last_modified_timestamp))
    }
}

#[derive(Debug)]
enum SnapshotStatus {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug)]
struct ScanSnapshot {
    started_at: SystemTime,
    completed_at: Option<SystemTime>,
    status: SnapshotStatus,
    file_count_estimate: usize,
    scan_progress: AtomicUsize,
    scanned_entries: HashMap<EntryKey, EntryRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntryKey {
    pub rel_path: PathBuf,
}

#[derive(Debug)]
pub enum EntryRecord {
    Folder(FolderRecord),
    File(FileRecord),
}

#[derive(Debug)]
pub struct FolderRecord {
    pub hash: Hash,
}

/// Chunks are not stored here: on a cache hit they are recovered from the
/// node store via `content_hash`, which retains file nodes across scans.
#[derive(Debug)]
pub struct FileRecord {
    pub metadata_hash: String,
    pub content_hash: Hash,
}

/// What scanning a folder yields: the edge to it from its parent, every node
/// discovered beneath it (to be committed to the store), and the metadata
/// records for the scan snapshot cache.
type ScannedFolder = (Entry, Vec<(Hash, Node)>, HashMap<EntryKey, EntryRecord>);

/// What scanning a file yields: the edge to it, its node if it wasn't already
/// in the store (cache hit), and its snapshot record.
type ScannedFile = (Entry, Option<(Hash, Node)>, (EntryKey, EntryRecord));

#[derive(Debug)]
enum ScannedEntry {
    Folder {
        entry: Entry,
        nodes: Vec<(Hash, Node)>,
        records: HashMap<EntryKey, EntryRecord>,
    },
    File {
        entry: Entry,
        node: Option<(Hash, Node)>,
        record: (EntryKey, EntryRecord),
    },
}

fn base_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// Scanner is responsible for maintaining a local representation of a given path and its contents
/// for efficient change detection and reconciliation with a remote representation.
#[derive(Debug)]
pub struct Scanner {
    file_path: PathBuf,
    status: ScannerStatus,
    publisher: Publisher<ScannerEvent>,
    should_show_progress: bool,
    file_chunker: FileChunker,
    snapshot_buffer_size: usize,
    current_snapshot: Option<ScanSnapshot>,
    scan_snapshots: VecDeque<ScanSnapshot>,
    node_store: Arc<RwLock<InMemoryNodeStore>>,
}

impl Scanner {
    pub fn new(file_path: impl Into<PathBuf>, should_show_progress: bool) -> Scanner {
        Scanner {
            file_path: file_path.into(),
            should_show_progress,
            status: ScannerStatus::Idle,
            publisher: Publisher::new(),
            file_chunker: FileChunker::new(
                1024 * 1024, // 1 MB chunk size
                Arc::new(RwLock::new(InMemoryBlobStore::new())),
            ),
            snapshot_buffer_size: 10,
            current_snapshot: None,
            scan_snapshots: VecDeque::new(),
            node_store: Arc::new(RwLock::new(InMemoryNodeStore::new())),
        }
    }

    pub fn events(&mut self) -> &mut Publisher<ScannerEvent> {
        &mut self.publisher
    }

    pub fn get_store(&self) -> Arc<RwLock<InMemoryNodeStore>> {
        Arc::clone(&self.node_store)
    }

    pub fn get_blob_store(&self) -> Arc<RwLock<InMemoryBlobStore>> {
        Arc::clone(&self.file_chunker.blob_store)
    }

    pub fn get_path(&self) -> &Path {
        &self.file_path
    }

    fn entry_key(&self, path: &Path) -> EntryKey {
        let rel_path = path
            .strip_prefix(&self.file_path)
            .map(|rel| rel.to_path_buf())
            .unwrap_or_else(|_| path.to_path_buf());

        EntryKey { rel_path }
    }

    fn create_snapshot(&mut self) {
        let file_count_estimate = self
            .scan_snapshots
            .back()
            .map(|snapshot| snapshot.scanned_entries.len())
            .unwrap_or(0);

        self.current_snapshot = Some(ScanSnapshot {
            started_at: SystemTime::now(),
            completed_at: None,
            status: SnapshotStatus::InProgress,
            file_count_estimate,
            scan_progress: AtomicUsize::new(0),
            scanned_entries: HashMap::new(),
        });
    }

    fn complete_snapshot(&mut self, scanned_entries: HashMap<EntryKey, EntryRecord>) {
        if let Some(mut snapshot) = self.current_snapshot.take() {
            snapshot.completed_at = Some(SystemTime::now());
            snapshot.status = SnapshotStatus::Completed;
            snapshot
                .scan_progress
                .store(scanned_entries.len(), std::sync::atomic::Ordering::SeqCst);
            snapshot.scanned_entries = scanned_entries;

            println!(
                "Scan completed for path {} in {}s. Number of entries scanned: {}",
                self.file_path.display(),
                snapshot
                    .completed_at
                    .unwrap()
                    .duration_since(snapshot.started_at)
                    .unwrap_or_default()
                    .as_secs(),
                snapshot.scanned_entries.len()
            );

            self.scan_snapshots.push_back(snapshot);
            if self.scan_snapshots.len() > self.snapshot_buffer_size {
                self.scan_snapshots.pop_front();
            }
        }
    }

    fn fail_snapshot(&mut self) {
        if let Some(mut snapshot) = self.current_snapshot.take() {
            snapshot.completed_at = Some(SystemTime::now());
            snapshot.status = SnapshotStatus::Failed;
            // Failed snapshots are not stored in the buffer
        }
    }

    pub fn scan(&mut self) -> Result<Hash, ScannerError> {
        if let ScannerStatus::Scanning = self.status {
            return Err(ScannerError::AlreadyScanning);
        }
        self.status = ScannerStatus::Scanning;
        self.create_snapshot();

        self.publisher.notify(
            &ScannerEvent::ScanStarted,
            &self.file_path.display().to_string(),
        );

        let file_path = self.file_path.clone();

        let root_hash = match self.scan_path(&file_path) {
            Ok((entry, nodes, scanned_entries)) => {
                // Commit the whole scan in one write lock so readers never see
                // a root whose nodes aren't all present yet.
                {
                    let mut store = self.node_store.write().unwrap();
                    for (hash, node) in nodes {
                        store.insert(hash, node);
                    }
                    store.set_root(entry.hash.clone());
                }
                self.complete_snapshot(scanned_entries);
                entry.hash
            }
            Err(e) => {
                let error_msg = format!("Error scanning path: {}", e);
                self.publisher.notify(
                    &ScannerEvent::Error(error_msg.clone()),
                    &self.file_path.display().to_string(),
                );
                self.fail_snapshot();
                self.status = ScannerStatus::Idle;
                return Err(e);
            }
        };

        self.publisher.notify(
            &ScannerEvent::ScanCompleted,
            &self.file_path.display().to_string(),
        );

        self.status = ScannerStatus::Idle;

        Ok(root_hash)
    }

    fn scan_path(&self, path: &Path) -> Result<ScannedFolder, ScannerError> {
        let metadata = fs::metadata(path).map_err(|e| ScannerError::GetMetadataFailed {
            path: path.display().to_string(),
            source: e,
        })?;

        if metadata.is_dir() {
            self.scan_directory(path)
        } else if metadata.is_file() {
            // When the root is a file, wrap it in a synthetic folder so the
            // tree always has a folder root.
            self.scan_file(path, &metadata).map(
                |(file_entry, file_node, (entry_key, entry_record))| {
                    let mut scanned_entries = HashMap::new();
                    scanned_entries.insert(entry_key, entry_record);

                    let (root_hash, root_node) = Node::folder(vec![], vec![file_entry]);

                    let mut nodes: Vec<(Hash, Node)> = file_node.into_iter().collect();
                    nodes.push((root_hash.clone(), root_node));

                    (
                        Entry::new(base_name(path), root_hash),
                        nodes,
                        scanned_entries,
                    )
                },
            )
        } else {
            Err(ScannerError::GetMetadataFailed {
                path: path.display().to_string(),
                source: std::io::Error::other("Path is neither a file nor a directory"),
            })
        }
    }

    fn scan_directory(&self, path: &Path) -> Result<ScannedFolder, ScannerError> {
        // TODO: Integrate the `ignore` crate to respect .gitignore rules and skip
        // directories like .git/, node_modules/, target/
        let read_dir = fs::read_dir(path).map_err(|e| ScannerError::ReadDirFailed {
            path: path.display().to_string(),
            source: e,
        })?;

        let entries =
            read_dir
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| ScannerError::ReadDirFailed {
                    path: path.display().to_string(),
                    source: e,
                })?;

        let scanned_entries = entries
            .par_iter()
            .map(|entry| {
                let entry_path = entry.path();

                // NOTE: fs::metadata follows symlinks. Symlink cycles could cause infinite
                // recursion. We should consider tracking visited inodes or limiting depth.
                let metadata =
                    fs::metadata(&entry_path).map_err(|e| ScannerError::GetMetadataFailed {
                        path: entry_path.display().to_string(),
                        source: e,
                    })?;

                if metadata.is_dir() {
                    let (entry, nodes, records) = self.scan_directory(&entry_path)?;
                    Ok(ScannedEntry::Folder {
                        entry,
                        nodes,
                        records,
                    })
                } else if metadata.is_file() {
                    let (entry, node, record) = self.scan_file(&entry_path, &metadata)?;
                    Ok(ScannedEntry::File {
                        entry,
                        node,
                        record,
                    })
                } else {
                    Err(ScannerError::GetMetadataFailed {
                        path: entry_path.display().to_string(),
                        source: std::io::Error::other("Path is neither a file nor a directory"),
                    })
                }
            })
            .collect::<Result<Vec<ScannedEntry>, ScannerError>>()?;

        let mut folder_entries = Vec::new();
        let mut file_entries = Vec::new();
        let mut nodes: Vec<(Hash, Node)> = Vec::new();
        let mut scanned_entry_map: HashMap<EntryKey, EntryRecord> = HashMap::new();

        for scanned_entry in scanned_entries {
            match scanned_entry {
                ScannedEntry::Folder {
                    entry,
                    nodes: child_nodes,
                    records,
                } => {
                    folder_entries.push(entry);
                    nodes.extend(child_nodes);
                    scanned_entry_map.extend(records);
                }
                ScannedEntry::File {
                    entry,
                    node,
                    record: (entry_key, entry_record),
                } => {
                    file_entries.push(entry);
                    nodes.extend(node);
                    scanned_entry_map.insert(entry_key, entry_record);
                }
            }
        }

        // Node::folder sorts entries internally, so the hash is canonical
        // regardless of the order read_dir returned them in.
        let (folder_hash, folder_node) = Node::folder(folder_entries, file_entries);
        nodes.push((folder_hash.clone(), folder_node));

        scanned_entry_map.insert(
            self.entry_key(path),
            EntryRecord::Folder(FolderRecord {
                hash: folder_hash.clone(),
            }),
        );

        Ok((
            Entry::new(base_name(path), folder_hash),
            nodes,
            scanned_entry_map,
        ))
    }

    fn scan_file(&self, path: &Path, metadata: &fs::Metadata) -> Result<ScannedFile, ScannerError> {
        let metadata_key = MetadataKey::new(metadata);
        let entry_key = self.entry_key(path);

        // Cache hit: metadata unchanged and the file node is already in the
        // store from a previous scan, so there is nothing to hash or insert.
        let cached_hash = self
            .scan_snapshots
            .back()
            .and_then(|snapshot| snapshot.scanned_entries.get(&entry_key))
            .and_then(|record| match record {
                EntryRecord::File(file_record) if file_record.metadata_hash == metadata_key.0 => {
                    Some(file_record.content_hash.clone())
                }
                _ => None,
            })
            .filter(|hash| self.node_store.read().unwrap().get_node(hash).is_some());

        let (content_hash, node) = match cached_hash {
            Some(hash) => (hash, None),
            None => {
                let file_blobs = self.file_chunker.chunk_file(path)?;
                let (hash, node) = Node::file(file_blobs);
                (hash.clone(), Some((hash, node)))
            }
        };

        let entry_record = EntryRecord::File(FileRecord {
            metadata_hash: metadata_key.0,
            content_hash: content_hash.clone(),
        });

        self.notify_progress();

        Ok((
            Entry::new(base_name(path), content_hash),
            node,
            (entry_key, entry_record),
        ))
    }

    fn notify_progress(&self) {
        if !self.should_show_progress {
            return;
        }

        let (progress, total) = match &self.current_snapshot {
            Some(snapshot) => (
                snapshot
                    .scan_progress
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                snapshot.file_count_estimate,
            ),
            None => (0, 0),
        };

        println!(
            "Scanned {} of ~{} files and directories.",
            progress + 1,
            total
        );
    }
}
