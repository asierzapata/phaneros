use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use phaneros_sync::{
    hash::Hash,
    node::{Node, NodeWire},
};
use sqlx::SqlitePool;

use super::repository::{NodeRepository, NodeRepositoryError, Version, VersionEvent};

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
    ) -> Result<VersionEvent, NodeRepositoryError> {
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

        let at = now_unix();
        let inserted =
            sqlx::query("INSERT INTO versions (drive_id, root_hash, at) VALUES (?, ?, ?)")
                .bind(drive_id)
                .bind(&new)
                .bind(at)
                .execute(&mut *tx)
                .await?;

        tx.commit().await?;

        Ok(VersionEvent {
            id: inserted.last_insert_rowid(),
            drive_id: drive_id.to_string(),
            root: new,
            at,
        })
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

    async fn get_missing_nodes(
        &self,
        drive_id: &str,
        hashes: &[Hash],
    ) -> Result<Vec<Hash>, NodeRepositoryError> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = hashes.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT hash FROM nodes WHERE drive_id = ? AND hash IN ({})",
            placeholders
        );

        let mut query = sqlx::query_scalar::<_, String>(&sql).bind(drive_id);
        for hash in hashes {
            query = query.bind(hash.as_str());
        }

        let found: Vec<String> = query.fetch_all(&self.pool).await?;
        let found_set: std::collections::HashSet<_> = found.into_iter().collect();

        Ok(hashes
            .iter()
            .filter(|h| !found_set.contains(h.as_str()))
            .cloned()
            .collect())
    }

    async fn get_nodes_batch(
        &self,
        drive_id: &str,
        hashes: &[Hash],
    ) -> Result<std::collections::HashMap<Hash, Node>, NodeRepositoryError> {
        if hashes.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let placeholders = hashes.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT hash, data FROM nodes WHERE drive_id = ? AND hash IN ({})",
            placeholders
        );

        let mut query = sqlx::query_as::<_, (String, String)>(&sql).bind(drive_id);
        for hash in hashes {
            query = query.bind(hash.as_str());
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut result = std::collections::HashMap::new();
        for (hash_str, json) in rows {
            let (_, node) = serde_json::from_str::<NodeWire>(&json)?.reconstruct();
            result.insert(hash_str.into(), node);
        }

        Ok(result)
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

    async fn max_version_id(&self) -> Result<i64, NodeRepositoryError> {
        let max: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM versions")
            .fetch_one(&self.pool)
            .await?;
        Ok(max)
    }

    async fn list_versions_after(
        &self,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<VersionEvent>, NodeRepositoryError> {
        let rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
            "SELECT id, drive_id, root_hash, at
             FROM versions
             WHERE id > ?
             ORDER BY id ASC
             LIMIT ?",
        )
        .bind(after_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, drive_id, root, at)| VersionEvent {
                id,
                drive_id,
                root,
                at,
            })
            .collect())
    }

    async fn list_drive_versions_after(
        &self,
        drive_id: &str,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<VersionEvent>, NodeRepositoryError> {
        let rows: Vec<(i64, String, i64)> = sqlx::query_as(
            "SELECT id, root_hash, at
             FROM versions
             WHERE drive_id = ? AND id > ?
             ORDER BY id ASC
             LIMIT ?",
        )
        .bind(drive_id)
        .bind(after_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, root, at)| VersionEvent {
                id,
                drive_id: drive_id.to_string(),
                root,
                at,
            })
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

    #[tokio::test]
    async fn put_root_returns_version_event_metadata() {
        let repo = repo().await;

        let version = repo.put_root("drive", "root1".into(), None).await.unwrap();

        assert_eq!(version.id, 1);
        assert_eq!(version.drive_id, "drive");
        assert_eq!(version.root, "root1");
        assert!(version.at > 0);
    }

    #[tokio::test]
    async fn list_versions_after_and_drive_versions_after_are_ordered_and_scoped() {
        let repo = repo().await;

        let drive_a_v1 = repo.put_root("drive-a", "a1".into(), None).await.unwrap();
        let drive_b_v1 = repo.put_root("drive-b", "b1".into(), None).await.unwrap();
        let drive_a_v2 = repo
            .put_root("drive-a", "a2".into(), Some("a1".into()))
            .await
            .unwrap();

        assert_eq!(repo.max_version_id().await.unwrap(), drive_a_v2.id);

        let global = repo.list_versions_after(0, 10).await.unwrap();
        let global_roots: Vec<&str> = global.iter().map(|v| v.root.as_str()).collect();
        assert_eq!(
            global_roots,
            vec![
                drive_a_v1.root.as_str(),
                drive_b_v1.root.as_str(),
                drive_a_v2.root.as_str()
            ]
        );

        let drive_a = repo
            .list_drive_versions_after("drive-a", 0, 10)
            .await
            .unwrap();
        let drive_a_roots: Vec<&str> = drive_a.iter().map(|v| v.root.as_str()).collect();
        assert_eq!(drive_a_roots, vec!["a1", "a2"]);

        let after_first_drive_a = repo
            .list_drive_versions_after("drive-a", drive_a_v1.id, 10)
            .await
            .unwrap();
        assert_eq!(after_first_drive_a.len(), 1);
        assert_eq!(after_first_drive_a[0].id, drive_a_v2.id);
    }

    #[tokio::test]
    async fn get_missing_nodes_returns_only_absent_hashes() {
        let repo = repo().await;
        let (hash1, node1) = Node::file(vec![BlobRef::from_bytes(b"one")]);
        let (hash2, _) = Node::file(vec![BlobRef::from_bytes(b"two")]);
        
        repo.put_node("drive", hash1.clone(), node1).await.unwrap();

        let missing = repo
            .get_missing_nodes("drive", &[hash1.clone(), hash2.clone()])
            .await
            .unwrap();
        
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], hash2);
    }

    #[tokio::test]
    async fn get_nodes_batch_returns_only_present_nodes() {
        let repo = repo().await;
        let (hash1, node1) = Node::file(vec![BlobRef::from_bytes(b"one")]);
        let (hash2, _) = Node::file(vec![BlobRef::from_bytes(b"two")]);
        
        repo.put_node("drive", hash1.clone(), node1.clone()).await.unwrap();

        let batch = repo
            .get_nodes_batch("drive", &[hash1.clone(), hash2.clone()])
            .await
            .unwrap();
        
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.get(&hash1).unwrap(), &node1);
    }
}
