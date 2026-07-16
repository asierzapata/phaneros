use std::{fs, io::Read, path::Path};

use thiserror::Error;

use crate::blob_store::blob::BlobRef;

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
}

impl FileChunker {
    pub fn new(chunk_size: usize) -> Self {
        FileChunker { chunk_size }
    }

    pub fn chunk_file(&self, path: &Path) -> Result<Vec<BlobRef>, FileChunkerError> {
        let file = fs::File::open(path).map_err(|e| FileChunkerError::ReadFileFailed {
            path: path.display().to_string(),
            source: e,
        })?;

        let mut reader = std::io::BufReader::new(file);
        let mut buffer = vec![0; self.chunk_size];
        let mut blobs = Vec::new();

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

            blobs.push(BlobRef::from_bytes(&buffer[..bytes_read]));
        }

        Ok(blobs)
    }
}
