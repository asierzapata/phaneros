use std::{fs, path::Path, sync::Arc};

use fastcdc::v2020::FastCDC;
use thiserror::Error;

use crate::blob_repository::{Blob, BlobRef, InMemoryBlobRepository};
use crate::node_repository::{Hash, Node};

pub const DEFAULT_MIN_CHUNK_SIZE: usize = 256 * 1024; // 256 KB
pub const DEFAULT_AVG_CHUNK_SIZE: usize = 1024 * 1024; // 1 MB
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileChunkerConfig {
    pub min_size: usize,
    pub avg_size: usize,
    pub max_size: usize,
}

impl Default for FileChunkerConfig {
    fn default() -> Self {
        Self {
            min_size: DEFAULT_MIN_CHUNK_SIZE,
            avg_size: DEFAULT_AVG_CHUNK_SIZE,
            max_size: DEFAULT_MAX_CHUNK_SIZE,
        }
    }
}

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
    config: FileChunkerConfig,
    pub blob_repository: Arc<InMemoryBlobRepository>,
}

impl FileChunker {
    pub fn new(
        config: FileChunkerConfig,
        blob_repository: Arc<InMemoryBlobRepository>,
    ) -> Self {
        FileChunker {
            config,
            blob_repository,
        }
    }

    pub fn chunk_file(&self, path: &Path) -> Result<Vec<BlobRef>, FileChunkerError> {
        read_chunks(path, self.config, |blob_ref, bytes| {
            self.blob_repository
                .insert_internal(
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
pub fn file_node_hash(path: &Path, config: FileChunkerConfig) -> Result<Hash, FileChunkerError> {
    let blob_refs = read_chunks(path, config, |_, _| {})?;
    let (hash, _) = Node::file(blob_refs);
    Ok(hash)
}

fn read_chunks<F>(
    path: &Path,
    config: FileChunkerConfig,
    mut on_chunk: F,
) -> Result<Vec<BlobRef>, FileChunkerError>
where
    F: FnMut(&BlobRef, &[u8]),
{
    let file_bytes = fs::read(path).map_err(|e| FileChunkerError::ReadFileFailed {
        path: path.display().to_string(),
        source: e,
    })?;

    let chunker = FastCDC::new(
        &file_bytes,
        config.min_size as u32,
        config.avg_size as u32,
        config.max_size as u32,
    );
    let mut blob_refs = Vec::new();

    for chunk in chunker {
        let chunk_bytes = &file_bytes[chunk.offset..chunk.offset + chunk.length];
        let blob_ref = BlobRef::from_bytes(chunk_bytes);
        on_chunk(&blob_ref, chunk_bytes);
        blob_refs.push(blob_ref);
    }

    Ok(blob_refs)
}
