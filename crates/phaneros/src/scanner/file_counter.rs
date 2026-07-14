use std::{fs, sync::atomic::AtomicUsize};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileCounterError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct FileCounter {
    count: AtomicUsize,
}

impl FileCounter {
    pub fn new() -> Self {
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
