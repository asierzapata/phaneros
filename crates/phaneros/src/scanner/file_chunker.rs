use std::{fs, io::Read};

use thiserror::Error;

use crate::folder_tree::FileChunk;

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

    pub fn chunk_file(&self, path: &str) -> Result<Vec<FileChunk>, FileChunkerError> {
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
