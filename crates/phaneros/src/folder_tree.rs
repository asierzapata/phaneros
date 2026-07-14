#[derive(Debug)]
pub struct IndexTree {
    pub root_hash: String,                 // The root hash of the tree
    pub folders: Vec<FolderIndexTreeNode>, // The folders of the node
    pub files: Vec<FileIndexTreeNode>,     // The files of the node
}

#[derive(Debug, Clone)]
pub struct FolderIndexTreeNode {
    pub name: String, // The name of the file or directory represented by the node
    pub hash: String, // The hash of the file or directory represented by the node
    pub folders: Vec<FolderIndexTreeNode>, // The folders of the node
    pub files: Vec<FileIndexTreeNode>, // The files of the node
}

impl FolderIndexTreeNode {
    pub fn new(
        name: String,
        folders: Vec<FolderIndexTreeNode>,
        files: Vec<FileIndexTreeNode>,
    ) -> Self {
        FolderIndexTreeNode::from_children(name, folders, files)
    }

    pub fn from_children(
        name: String,
        folders: Vec<FolderIndexTreeNode>,
        files: Vec<FileIndexTreeNode>,
    ) -> Self {
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

        FolderIndexTreeNode {
            name,
            hash: hasher.finalize().to_hex().to_string(),
            folders,
            files,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileIndexTreeNode {
    pub name: String,           // The name of the file represented by the node
    pub hash: String,           // The hash of the file represented by the node
    pub chunks: Vec<FileChunk>, // The chunks of the file represented by the node
}

impl FileIndexTreeNode {
    pub fn new(name: String, chunks: Vec<FileChunk>) -> Self {
        FileIndexTreeNode::from_chunks(name, chunks)
    }

    pub fn from_chunks(name: String, chunks: Vec<FileChunk>) -> Self {
        // To compute a file hash we get the hashes of all the chunks and concatenate them, then hash the result.
        let mut hasher = blake3::Hasher::new();
        // We add a first byte to the hash to differentiate between files and folders
        // so an empty folder and an empty file don't have the same hash.
        hasher.update(&[1]);
        for chunk in &chunks {
            hasher.update(chunk.hash.as_bytes());
        }

        FileIndexTreeNode {
            name,
            hash: hasher.finalize().to_hex().to_string(),
            chunks,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileChunk {
    pub hash: String, // The hash of the file chunk
    pub size: u64,    // The size of the file chunk
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
