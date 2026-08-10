use async_trait::async_trait;
use phaneros_sync::node::NodeWire;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::node_repository::{
    Hash, Node, NodeRepository, WritableNodeRepository, repository::NodeRepositoryError,
};

#[derive(Debug)]
pub struct HttpNodeRepository {
    client: reqwest::Client,
    base_url: String,
    drive_id: String,
    auth: String,
    cached_root: std::sync::RwLock<Option<Hash>>,
    inserted: std::sync::atomic::AtomicUsize,
}

#[derive(Deserialize)]
struct RootResponse {
    hash: Option<String>,
}

#[derive(Deserialize)]
struct MissingResponse {
    missing: HashSet<Hash>,
}

#[derive(Deserialize)]
struct BatchResponse {
    nodes: HashMap<Hash, NodeWire>,
}

#[derive(Serialize)]
struct PutRootBody<'a> {
    hash: &'a Hash,
    expected: Option<&'a Hash>,
}

impl HttpNodeRepository {
    pub async fn new(
        base_url: impl Into<String>,
        drive_id: impl Into<String>,
        token: impl AsRef<str>,
    ) -> Self {
        let repo = Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            drive_id: drive_id.into(),
            auth: format!("Bearer {}", token.as_ref()),
            cached_root: std::sync::RwLock::new(None),
            inserted: std::sync::atomic::AtomicUsize::new(0),
        };
        let _ = repo.refresh_root().await;
        repo
    }

    fn nodes_url(&self, hash: &Hash) -> String {
        format!(
            "{}/api/drives/{}/nodes/{}",
            self.base_url, self.drive_id, hash
        )
    }

    fn root_url(&self) -> String {
        format!("{}/api/drives/{}/root", self.base_url, self.drive_id)
    }

    pub async fn refresh_root(&self) -> Result<Option<String>, NodeRepositoryError> {
        match self.fetch_root().await {
            Ok(root) => {
                *self.cached_root.write().unwrap() = root.clone();
                Ok(root)
            }
            Err(err) => Err(err),
        }
    }

    async fn fetch_root(&self) -> Result<Option<Hash>, NodeRepositoryError> {
        let resp = self
            .client
            .get(&self.root_url())
            .header("Authorization", &self.auth)
            .send()
            .await
            .map_err(|_| NodeRepositoryError::RootRetrieveFailed)?;

        match resp.status().as_u16() {
            200 => {
                let body: RootResponse = resp
                    .json()
                    .await
                    .map_err(|_| NodeRepositoryError::RootRetrieveFailed)?;
                Ok(body.hash)
            }
            404 => Ok(None),
            _ => Err(NodeRepositoryError::RootRetrieveFailed),
        }
    }

    pub async fn insert_internal(&self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        let resp = self
            .client
            .put(&self.nodes_url(&hash))
            .header("Authorization", &self.auth)
            .json(&node)
            .send()
            .await
            .map_err(|_| NodeRepositoryError::InsertFailed(hash.clone()))?;

        if resp.status().is_success() {
            self.inserted
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        } else {
            Err(NodeRepositoryError::InsertFailed(hash))
        }
    }

    pub async fn set_root_internal(&self, hash: Hash) -> Result<(), NodeRepositoryError> {
        let cached = self.cached_root.read().unwrap().clone();
        let body = PutRootBody {
            hash: &hash,
            expected: cached.as_ref(),
        };
        let resp = self
            .client
            .put(&self.root_url())
            .header("Authorization", &self.auth)
            .json(&body)
            .send()
            .await
            .map_err(|_| NodeRepositoryError::SetRootFailed(hash.clone()))?;

        match resp.status().as_u16() {
            200 | 201 | 204 => {
                *self.cached_root.write().unwrap() = Some(hash);
                Ok(())
            }
            409 => {
                let actual = resp
                    .json::<RootResponse>()
                    .await
                    .ok()
                    .and_then(|body| body.hash);
                *self.cached_root.write().unwrap() = actual.clone();
                Err(NodeRepositoryError::RootConflict { actual })
            }
            _ => Err(NodeRepositoryError::SetRootFailed(hash)),
        }
    }

    pub fn len(&self) -> usize {
        self.inserted.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl NodeRepository for HttpNodeRepository {
    async fn root_hash(&self) -> Result<Option<Hash>, NodeRepositoryError> {
        Ok(self.cached_root.read().unwrap().clone())
    }

    async fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        let resp = self
            .client
            .get(&self.nodes_url(hash))
            .header("Authorization", &self.auth)
            .send()
            .await
            .map_err(|_| NodeRepositoryError::NodeRetrieveFailed(hash.clone()))?;

        match resp.status().as_u16() {
            200 => {
                let wire: NodeWire = resp
                    .json()
                    .await
                    .map_err(|_| NodeRepositoryError::NodeRetrieveFailed(hash.clone()))?;
                let (_, node) = wire.reconstruct();
                Ok(Some(node))
            }
            404 => Ok(None),
            _ => Err(NodeRepositoryError::NodeRetrieveFailed(hash.clone())),
        }
    }

    async fn get_missing(&self, hashes: &[Hash]) -> Result<HashSet<Hash>, NodeRepositoryError> {
        if hashes.is_empty() {
            return Ok(HashSet::new());
        }

        let url = format!(
            "{}/api/drives/{}/nodes/missing",
            self.base_url, self.drive_id
        );
        let payload = serde_json::json!({ "hashes": hashes });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(drive_id = %self.drive_id, error = ?e, "http node get_missing failed");
                NodeRepositoryError::NodeRetrieveFailed(hashes[0].clone())
            })?;

        if !resp.status().is_success() {
            return Err(NodeRepositoryError::NodeRetrieveFailed(hashes[0].clone()));
        }

        let body: MissingResponse = resp.json().await.map_err(|e| {
            tracing::error!(drive_id = %self.drive_id, error = ?e, "http node get_missing parse failed");
            NodeRepositoryError::NodeRetrieveFailed(hashes[0].clone())
        })?;

        Ok(body.missing)
    }

    async fn get_nodes_batch(
        &self,
        hashes: &[Hash],
    ) -> Result<HashMap<Hash, Node>, NodeRepositoryError> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }

        let url = format!("{}/api/drives/{}/nodes/batch", self.base_url, self.drive_id);
        let payload = serde_json::json!({ "hashes": hashes });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(drive_id = %self.drive_id, error = ?e, "http node get_nodes_batch failed");
                NodeRepositoryError::NodeRetrieveFailed(hashes[0].clone())
            })?;

        if !resp.status().is_success() {
            return Err(NodeRepositoryError::NodeRetrieveFailed(hashes[0].clone()));
        }

        let body: BatchResponse = resp.json().await.map_err(|e| {
            tracing::error!(drive_id = %self.drive_id, error = ?e, "http node get_nodes_batch parse failed");
            NodeRepositoryError::NodeRetrieveFailed(hashes[0].clone())
        })?;

        let mut nodes = HashMap::new();
        for (hash, wire) in body.nodes {
            let (_, node) = wire.reconstruct();
            nodes.insert(hash, node);
        }

        Ok(nodes)
    }
}

#[async_trait]
impl WritableNodeRepository for HttpNodeRepository {
    async fn insert(&self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        self.insert_internal(hash, node).await
    }

    async fn set_root(&self, hash: Hash) -> Result<(), NodeRepositoryError> {
        self.set_root_internal(hash).await
    }
}
