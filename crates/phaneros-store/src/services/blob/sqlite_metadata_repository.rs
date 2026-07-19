use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use phaneros_sync::hash::Hash;
use sqlx::SqlitePool;

use super::metadata_repository::{BlobMetadataRepository, BlobMetadataRepositoryError};

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

    async fn declare(&self, hash: &Hash, size: i64) -> Result<(), BlobMetadataRepositoryError> {
        sqlx::query(
            "INSERT INTO blob_metadata (hash, size) VALUES (?, ?) ON CONFLICT(hash) DO NOTHING",
        )
        .bind(hash)
        .bind(size)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn declared_size(
        &self,
        hash: &Hash,
    ) -> Result<Option<i64>, BlobMetadataRepositoryError> {
        let size: Option<i64> = sqlx::query_scalar("SELECT size FROM blob_metadata WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;
        Ok(size)
    }

    async fn mark_committed(&self, hash: &Hash) -> Result<(), BlobMetadataRepositoryError> {
        sqlx::query("UPDATE blob_metadata SET committed_at = ? WHERE hash = ?")
            .bind(now_unix())
            .bind(hash)
            .execute(&self.pool)
            .await?;
        Ok(())
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
        repo.declare(&hash, 5885).await.unwrap();
        assert_eq!(repo.declared_size(&hash).await.unwrap(), Some(5885));
        assert!(!repo.exists(&hash).await.unwrap());

        // Committed: the bytes have landed, so the store now reports it as held.
        repo.mark_committed(&hash).await.unwrap();
        assert!(repo.exists(&hash).await.unwrap());
    }

    #[tokio::test]
    async fn declared_size_is_none_for_unknown_blob() {
        let repo = repo().await;
        assert_eq!(
            repo.declared_size(&"missing".into()).await.unwrap(),
            None
        );
    }
}
