use crate::blob_store::blob::{Blob, Hash};

pub trait BlobStore {
    fn get_blob(&self, hash: &Hash) -> Option<Blob>;
    fn contains(&self, hash: &Hash) -> bool;
}

/// A blob store that can also be written to. The syncer reads both sides
/// through `BlobStore` and pushes missing blobs through this.
pub trait WritableBlobStore: BlobStore {
    fn insert(&mut self, hash: Hash, blob: Blob);
}
