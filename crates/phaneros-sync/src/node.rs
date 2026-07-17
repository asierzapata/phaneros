use crate::{blob::BlobRef, hash::Hash};

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Folder {
        folders: Vec<Entry>,
        files: Vec<Entry>,
    },
    File {
        blobs: Vec<BlobRef>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub hash: Hash,
}

impl Entry {
    pub fn new(name: impl Into<String>, hash: impl Into<Hash>) -> Self {
        Entry {
            name: name.into(),
            hash: hash.into(),
        }
    }
}

impl Node {
    pub fn folder(mut folders: Vec<Entry>, mut files: Vec<Entry>) -> (Hash, Node) {
        folders.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        let mut hasher = blake3::Hasher::new();

        // We add a first byte to the hash to differentiate between files and folders
        // so an empty folder and an empty file don't have the same hash.
        hasher.update(&[0]);

        for folder in &folders {
            hasher.update(&(folder.name.len() as u64).to_le_bytes());
            hasher.update(folder.name.as_bytes());
            hasher.update(folder.hash.as_bytes());
        }

        for file in &files {
            hasher.update(&(file.name.len() as u64).to_le_bytes());
            hasher.update(file.name.as_bytes());
            hasher.update(file.hash.as_bytes());
        }

        let hash = hasher.finalize().to_hex().to_string();

        (hash, Node::Folder { folders, files })
    }

    pub fn file(blobs: Vec<BlobRef>) -> (Hash, Node) {
        let mut hasher = blake3::Hasher::new();

        // We add a first byte to the hash to differentiate between files and folders
        // so an empty folder and an empty file don't have the same hash.
        hasher.update(&[1]);

        for blob in &blobs {
            hasher.update(blob.hash.as_bytes());
        }

        let hash = hasher.finalize().to_hex().to_string();

        (hash, Node::File { blobs })
    }
}
