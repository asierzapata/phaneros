use crate::hash::Hash;

#[derive(Debug, Clone, PartialEq)]
pub struct Blob {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlobRef {
    pub hash: Hash,
    pub size: u64,
}

impl BlobRef {
    pub fn new(bytes: &[u8]) -> Self {
        BlobRef::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        BlobRef {
            hash: blake3::hash(bytes).to_hex().to_string(),
            size: bytes.len() as u64,
        }
    }
}
