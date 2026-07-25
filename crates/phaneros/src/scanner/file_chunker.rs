use std::{
    fs,
    io::Read,
    path::Path,
    sync::{Arc, RwLock},
};

use thiserror::Error;

use crate::blob_repository::{Blob, BlobRef, InMemoryBlobRepository};
use crate::node_repository::{Hash, Node};

pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

#[derive(Error, Debug)]
pub enum FileChunkerError {
    #[error("Error reading file: {path}")]
    ReadFileFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub struct FileChunker {
    chunk_size: usize,
    pub blob_repository: Arc<RwLock<InMemoryBlobRepository>>,
}

impl FileChunker {
    pub fn new(chunk_size: usize, blob_repository: Arc<RwLock<InMemoryBlobRepository>>) -> Self {
        FileChunker {
            chunk_size,
            blob_repository,
        }
    }

    pub fn chunk_file(&self, path: &Path) -> Result<Vec<BlobRef>, FileChunkerError> {
        read_chunks(path, self.chunk_size, |blob_ref, bytes| {
            self.blob_repository
                .write()
                .unwrap()
                .insert(
                    blob_ref.hash.clone(),
                    Blob {
                        bytes: bytes.to_vec(),
                    },
                )
                .expect("in-memory blob store insert is infallible");
        })
    }
}

/// The file node hash `path` would get if it were scanned right now, without storing its chunks anywhere.
///
/// Used to tell whether a file on disk still holds the content a tree claims it does, so we can spot local edits before overwriting them.
pub fn file_node_hash(path: &Path, chunk_size: usize) -> Result<Hash, FileChunkerError> {
    let blob_refs = read_chunks(path, chunk_size, |_, _| {})?;
    let (hash, _) = Node::file(blob_refs);
    Ok(hash)
}

fn read_chunks<F>(
    path: &Path,
    chunk_size: usize,
    mut on_chunk: F,
) -> Result<Vec<BlobRef>, FileChunkerError>
where
    F: FnMut(&BlobRef, &[u8]),
{
    let file = fs::File::open(path).map_err(|e| FileChunkerError::ReadFileFailed {
        path: path.display().to_string(),
        source: e,
    })?;

    let mut reader = std::io::BufReader::new(file);
    let mut buffer = vec![0; chunk_size];
    let mut blob_refs = Vec::new();

    loop {
        let bytes_read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                return Err(FileChunkerError::ReadFileFailed {
                    path: path.display().to_string(),
                    source: e,
                });
            }
        };

        let blob_ref = BlobRef::from_bytes(&buffer[..bytes_read]);
        on_chunk(&blob_ref, &buffer[..bytes_read]);
        blob_refs.push(blob_ref);
    }

    Ok(blob_refs)
}
