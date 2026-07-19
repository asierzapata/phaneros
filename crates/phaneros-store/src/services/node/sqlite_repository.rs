use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use phaneros_sync::{
    hash::Hash,
    node::{Node, NodeWire},
};
use sqlx::SqlitePool;

use super::repository::{NodeRepository, NodeRepositoryError, Version};

pub struct SqliteNodeRepository {
    pool: SqlitePool,
}

impl SqliteNodeRepository {
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
impl NodeRepository for SqliteNodeRepository {
    async fn get_root(&self, drive_id: &str) -> Result<Option<Hash>, NodeRepositoryError> {
        let root: Option<String> =
            sqlx::query_scalar("SELECT root_hash FROM drive_roots WHERE drive_id = ?")
                .bind(drive_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(root)
    }

    async fn put_root(
        &self,
        drive_id: &str,
        new: Hash,
        expected: Option<Hash>,
    ) -> Result<(), NodeRepositoryError> {
        let mut tx = self.pool.begin().await?;

        let current: Option<String> =
            sqlx::query_scalar("SELECT root_hash FROM drive_roots WHERE drive_id = ?")
                .bind(drive_id)
                .fetch_optional(&mut *tx)
                .await?;

        if current != expected {
            return Err(NodeRepositoryError::RootMismatch {
                expected,
                actual: current,
            });
        }

        sqlx::query(
            "INSERT INTO drive_roots (drive_id, root_hash) VALUES (?, ?)
             ON CONFLICT(drive_id) DO UPDATE SET root_hash = excluded.root_hash",
        )
        .bind(drive_id)
        .bind(&new)
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO versions (drive_id, root_hash, at) VALUES (?, ?, ?)")
            .bind(drive_id)
            .bind(&new)
            .bind(now_unix())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_node(
        &self,
        drive_id: &str,
        hash: &Hash,
    ) -> Result<Option<Node>, NodeRepositoryError> {
        let data: Option<String> =
            sqlx::query_scalar("SELECT data FROM nodes WHERE drive_id = ? AND hash = ?")
                .bind(drive_id)
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;

        match data {
            Some(json) => {
                let (_, node) = serde_json::from_str::<NodeWire>(&json)?.reconstruct();
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    async fn put_node(
        &self,
        drive_id: &str,
        hash: Hash,
        node: Node,
    ) -> Result<(), NodeRepositoryError> {
        let json = serde_json::to_string(&node)?;
        sqlx::query(
            "INSERT INTO nodes (drive_id, hash, data) VALUES (?, ?, ?)
             ON CONFLICT(drive_id, hash) DO NOTHING",
        )
        .bind(drive_id)
        .bind(&hash)
        .bind(&json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_versions(&self, drive_id: &str) -> Result<Vec<Version>, NodeRepositoryError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT root_hash, at FROM versions WHERE drive_id = ? ORDER BY id DESC",
        )
        .bind(drive_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(root, at)| Version { root, at })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phaneros_sync::{blob::BlobRef, node::Entry};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn repo() -> SqliteNodeRepository {
        let options = SqliteConnectOptions::new().in_memory(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        SqliteNodeRepository::new(pool)
    }

    #[tokio::test]
    async fn node_round_trips_through_storage() {
        let repo = repo().await;
        let (hash, node) = Node::file(vec![BlobRef::from_bytes(b"hello")]);

        assert_eq!(repo.get_node("drive", &hash).await.unwrap(), None);
        repo.put_node("drive", hash.clone(), node.clone())
            .await
            .unwrap();
        assert_eq!(repo.get_node("drive", &hash).await.unwrap(), Some(node));
    }

    #[tokio::test]
    async fn nodes_are_scoped_per_drive() {
        let repo = repo().await;
        let (hash, node) = Node::folder(vec![Entry::new("sub", "abc")], vec![]);
        repo.put_node("drive-a", hash.clone(), node).await.unwrap();

        // Same hash, different drive: absent.
        assert_eq!(repo.get_node("drive-b", &hash).await.unwrap(), None);
    }

    #[tokio::test]
    async fn root_cas_flips_only_on_matching_expected() {
        let repo = repo().await;

        // First set: the drive is empty, so `expected` must be None.
        repo.put_root("drive", "root1".into(), None).await.unwrap();
        assert_eq!(repo.get_root("drive").await.unwrap(), Some("root1".into()));

        // Stale expected: rejected, root unchanged, actual reported.
        let err = repo
            .put_root("drive", "root2".into(), Some("wrong".into()))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            NodeRepositoryError::RootMismatch { actual: Some(a), .. } if a == "root1"
        ));
        assert_eq!(repo.get_root("drive").await.unwrap(), Some("root1".into()));

        // Correct expected: flips.
        repo.put_root("drive", "root2".into(), Some("root1".into()))
            .await
            .unwrap();
        assert_eq!(repo.get_root("drive").await.unwrap(), Some("root2".into()));
    }

    #[tokio::test]
    async fn root_cas_rejects_non_null_expected_on_empty_drive() {
        let repo = repo().await;
        let err = repo
            .put_root("drive", "root1".into(), Some("ghost".into()))
            .await
            .unwrap_err();
        // No current root to report, so `actual` is None rather than a bogus hash.
        assert!(matches!(
            err,
            NodeRepositoryError::RootMismatch { actual: None, .. }
        ));
        assert_eq!(repo.get_root("drive").await.unwrap(), None);
    }

    #[tokio::test]
    async fn versions_are_logged_newest_first() {
        let repo = repo().await;
        repo.put_root("drive", "root1".into(), None).await.unwrap();
        repo.put_root("drive", "root2".into(), Some("root1".into()))
            .await
            .unwrap();

        let versions = repo.list_versions("drive").await.unwrap();
        let roots: Vec<&str> = versions.iter().map(|v| v.root.as_str()).collect();
        assert_eq!(roots, vec!["root2", "root1"]);
    }
}
