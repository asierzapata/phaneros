use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use phaneros_sync::hash::Hash;
use sqlx::{FromRow, SqlitePool};

use super::metadata_repository::{
    BlobMetadataInfo, BlobMetadataRepository, BlobMetadataRepositoryError,
};

pub struct SqliteBlobMetadataRepository {
    pool: SqlitePool,
}

impl SqliteBlobMetadataRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(FromRow)]
struct BlobMetadataRow {
    hash: String,
    size: i64,
    uncompressed_size: Option<i64>,
    compression: Option<String>,
    committed_at: Option<i64>,
}

#[async_trait]
impl BlobMetadataRepository for SqliteBlobMetadataRepository {
    async fn exists(&self, hash: &Hash) -> Result<bool, BlobMetadataRepositoryError> {
        let found: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM blob_metadata WHERE hash = ? AND committed_at IS NOT NULL",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }

    async fn declare(
        &self,
        hash: &Hash,
        size: i64,
        uncompressed_size: Option<i64>,
        compression: Option<&str>,
    ) -> Result<(), BlobMetadataRepositoryError> {
        let comp = compression.unwrap_or("none");
        sqlx::query(
            "INSERT INTO blob_metadata (hash, size, uncompressed_size, compression) VALUES (?, ?, ?, ?) ON CONFLICT(hash) DO NOTHING",
        )
        .bind(hash)
        .bind(size)
        .bind(uncompressed_size)
        .bind(comp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn declared_size(&self, hash: &Hash) -> Result<Option<i64>, BlobMetadataRepositoryError> {
        let size: Option<i64> = sqlx::query_scalar("SELECT size FROM blob_metadata WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;
        Ok(size)
    }

    async fn get_metadata(
        &self,
        hash: &Hash,
    ) -> Result<Option<BlobMetadataInfo>, BlobMetadataRepositoryError> {
        let row: Option<BlobMetadataRow> =
            sqlx::query_as("SELECT hash, size, uncompressed_size, compression, committed_at FROM blob_metadata WHERE hash = ?")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| BlobMetadataInfo {
            hash: r.hash,
            size: r.size,
            uncompressed_size: r.uncompressed_size,
            compression: r.compression.unwrap_or_else(|| "none".to_string()),
            committed_at: r.committed_at,
        }))
    }

    async fn mark_committed(&self, hash: &Hash) -> Result<(), BlobMetadataRepositoryError> {
        sqlx::query("UPDATE blob_metadata SET committed_at = ? WHERE hash = ?")
            .bind(now_unix())
            .bind(hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_missing(&self, hashes: &[Hash]) -> Result<Vec<Hash>, BlobMetadataRepositoryError> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }

        let json = serde_json::to_string(hashes)?;

        let missing: Vec<String> = sqlx::query_scalar(
            "SELECT value FROM json_each(?)
             EXCEPT
             SELECT hash FROM blob_metadata WHERE hash IN (SELECT value FROM json_each(?)) AND committed_at IS NOT NULL",
        )
        .bind(&json)
        .bind(&json)
        .fetch_all(&self.pool)
        .await?;

        Ok(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn repo() -> SqliteBlobMetadataRepository {
        let options = SqliteConnectOptions::new().in_memory(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        SqliteBlobMetadataRepository::new(pool)
    }

    #[tokio::test]
    async fn declare_then_commit_flips_existence() {
        let repo = repo().await;
        let hash: Hash = "abc123".into();

        // Declared: size is known, but the store does not yet hold the bytes.
        assert!(!repo.exists(&hash).await.unwrap());
        repo.declare(&hash, 5885, Some(10000), Some("zstd"))
            .await
            .unwrap();
        assert_eq!(repo.declared_size(&hash).await.unwrap(), Some(5885));

        let meta = repo.get_metadata(&hash).await.unwrap().unwrap();
        assert_eq!(meta.uncompressed_size, Some(10000));
        assert_eq!(meta.compression, "zstd");

        assert!(!repo.exists(&hash).await.unwrap());

        // Committed: the bytes have landed, so the store now reports it as held.
        repo.mark_committed(&hash).await.unwrap();
        assert!(repo.exists(&hash).await.unwrap());
    }

    #[tokio::test]
    async fn declared_size_is_none_for_unknown_blob() {
        let repo = repo().await;
        assert_eq!(repo.declared_size(&"missing".into()).await.unwrap(), None);
    }

    #[tokio::test]
    async fn get_missing_returns_only_absent_or_uncommitted_blobs() {
        let repo = repo().await;
        let hash1: Hash = "hash1".into();
        let hash2: Hash = "hash2".into();
        let hash3: Hash = "hash3".into();

        // hash1 is declared and committed (should not be missing)
        repo.declare(&hash1, 10, None, None).await.unwrap();
        repo.mark_committed(&hash1).await.unwrap();

        // hash2 is only declared, not committed (should be missing)
        repo.declare(&hash2, 10, None, None).await.unwrap();

        // hash3 is not even declared (should be missing)
        
        let missing = repo.get_missing(&[hash1.clone(), hash2.clone(), hash3.clone()]).await.unwrap();

        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&hash2));
        assert!(missing.contains(&hash3));
    }
}
