pub type Hash = String;

#[derive(Debug, Clone, PartialEq)]
pub struct FileChunk {
    pub hash: Hash, // The hash of the file chunk
    pub size: u64,  // The size of the file chunk
}

impl FileChunk {
    pub fn new(bytes: &[u8]) -> Self {
        FileChunk::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let hash = blake3::hash(bytes).to_hex().to_string();
        FileChunk {
            hash,
            size: bytes.len() as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Folder {
        folders: Vec<Entry>,
        files: Vec<Entry>,
    },
    File {
        chunks: Vec<FileChunk>,
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

    pub fn file(chunks: Vec<FileChunk>) -> (Hash, Node) {
        let mut hasher = blake3::Hasher::new();

        // We add a first byte to the hash to differentiate between files and folders
        // so an empty folder and an empty file don't have the same hash.
        hasher.update(&[1]);

        for chunk in &chunks {
            hasher.update(chunk.hash.as_bytes());
        }

        let hash = hasher.finalize().to_hex().to_string();

        (hash, Node::File { chunks })
    }
}
