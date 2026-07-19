use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use bytes::Bytes;
use phaneros_sync::hash::Hash;
use tokio::io::AsyncWriteExt;

use super::bytes_repository::{BlobBytesRepository, BlobBytesRepositoryError};

// Distinguishes concurrent temp files for the same blob; only needs to be unique.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Content-addressed blob bytes on the local filesystem. Files live under `root`,
/// sharded by the first two hex characters of the hash so no single directory grows unbounded.
pub struct FsBlobBytesRepository {
    root: PathBuf,
}

impl FsBlobBytesRepository {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Map a hash to its on-disk path. The hash is validated as hex first: it is
    /// attacker-influenced (it comes straight from the URL), and without this a
    /// value like `../../etc/passwd` would escape `root`.
    fn path_for(&self, hash: &Hash) -> Result<PathBuf, BlobBytesRepositoryError> {
        let valid = hash.len() >= 3 && hash.bytes().all(|b| b.is_ascii_hexdigit());
        if !valid {
            return Err(BlobBytesRepositoryError::InvalidHash);
        }
        let (shard, rest) = hash.split_at(2);
        Ok(self.root.join(shard).join(rest))
    }

    fn temp_path(final_path: &Path) -> PathBuf {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name: OsString = final_path.into();
        name.push(format!(".tmp.{}.{}", std::process::id(), n));
        PathBuf::from(name)
    }
}

#[async_trait]
impl BlobBytesRepository for FsBlobBytesRepository {
    async fn put_bytes(&self, hash: &Hash, bytes: Bytes) -> Result<(), BlobBytesRepositoryError> {
        let path = self.path_for(hash)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to a unique temp file, fsync, then atomically rename into place.
        // A crash mid-write leaves only the temp file, so `has`/`get_bytes` never
        // observe a partially written blob under its real name.
        let tmp = Self::temp_path(&path);
        {
            let mut file = tokio::fs::File::create(&tmp).await?;
            file.write_all(&bytes).await?;
            file.sync_all().await?;
        }
        tokio::fs::rename(&tmp, &path).await?;
        Ok(())
    }

    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobBytesRepositoryError> {
        let path = self.path_for(hash)?;
        match tokio::fs::read(&path).await {
            Ok(data) => Ok(Some(Bytes::from(data))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn has(&self, hash: &Hash) -> Result<bool, BlobBytesRepositoryError> {
        let path = self.path_for(hash)?;
        match tokio::fs::metadata(&path).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_root() -> PathBuf {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("phaneros-blob-test-{}-{}", std::process::id(), n))
    }

    #[tokio::test]
    async fn round_trips_bytes() {
        let root = unique_temp_root();
        let repo = FsBlobBytesRepository::new(root.clone());
        let hash: Hash = "a".repeat(64);

        assert!(!repo.has(&hash).await.unwrap());
        assert_eq!(repo.get_bytes(&hash).await.unwrap(), None);

        repo.put_bytes(&hash, Bytes::from_static(b"hello"))
            .await
            .unwrap();

        assert!(repo.has(&hash).await.unwrap());
        assert_eq!(
            repo.get_bytes(&hash).await.unwrap().unwrap(),
            Bytes::from_static(b"hello")
        );

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn rejects_hashes_that_could_escape_the_root() {
        let repo = FsBlobBytesRepository::new(unique_temp_root());
        let bad: Hash = "../../etc/passwd".into();

        assert!(matches!(
            repo.get_bytes(&bad).await,
            Err(BlobBytesRepositoryError::InvalidHash)
        ));
        assert!(matches!(
            repo.has(&bad).await,
            Err(BlobBytesRepositoryError::InvalidHash)
        ));
        assert!(matches!(
            repo.put_bytes(&bad, Bytes::from_static(b"x")).await,
            Err(BlobBytesRepositoryError::InvalidHash)
        ));
    }
}
