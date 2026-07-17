use std::{
    fs,
    io::Read,
    path::Path,
    sync::{Arc, RwLock},
};

use thiserror::Error;

use crate::blob_store::{Blob, BlobRef, InMemoryBlobStore};

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
    pub blob_store: Arc<RwLock<InMemoryBlobStore>>,
}

impl FileChunker {
    pub fn new(chunk_size: usize, blob_store: Arc<RwLock<InMemoryBlobStore>>) -> Self {
        FileChunker {
            chunk_size,
            blob_store,
        }
    }

    pub fn chunk_file(&self, path: &Path) -> Result<Vec<BlobRef>, FileChunkerError> {
        let file = fs::File::open(path).map_err(|e| FileChunkerError::ReadFileFailed {
            path: path.display().to_string(),
            source: e,
        })?;

        let mut reader = std::io::BufReader::new(file);
        let mut buffer = vec![0; self.chunk_size];
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

            let blob = Blob {
                bytes: buffer[..bytes_read].to_vec(),
            };

            self.blob_store
                .write()
                .unwrap()
                .insert(blob_ref.hash.clone(), blob)
                .expect("in-memory blob store insert is infallible");

            blob_refs.push(blob_ref);
        }

        Ok(blob_refs)
    }
}
