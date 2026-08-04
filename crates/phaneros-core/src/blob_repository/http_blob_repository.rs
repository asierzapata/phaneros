use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashSet;

use crate::blob_repository::{
    Blob, BlobRepository, Hash, WritableBlobRepository, repository::BlobRepositoryError,
};

#[derive(Deserialize)]
pub struct HttpBlobRepository {
    base_url: String,
    drive_id: String,
    auth: String,
    #[serde(skip)]
    #[serde(default = "reqwest::Client::new")]
    client: reqwest::Client,
    #[serde(skip)]
    inserted: std::sync::atomic::AtomicUsize,
    #[serde(skip)]
    tracker: std::sync::RwLock<Option<crate::telemetry::ProgressTracker>>,
}

#[derive(Deserialize)]
struct MissingResponse {
    missing: Vec<Hash>,
}

impl HttpBlobRepository {
    pub fn new(base_url: String, drive_id: String, token: String) -> Self {
        Self {
            base_url,
            drive_id,
            auth: format!("Bearer {}", token),
            client: reqwest::Client::new(),
            inserted: std::sync::atomic::AtomicUsize::new(0),
            tracker: std::sync::RwLock::new(None),
        }
    }
}

#[async_trait]
impl BlobRepository for HttpBlobRepository {
    async fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError> {
        let url = format!(
            "{}/api/drives/{}/blobs/{}",
            self.base_url, self.drive_id, hash
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth)
            .send()
            .await
            .map_err(|e| {
                eprintln!("[http-blob] get err={:?}", e);
                BlobRepositoryError::RetrieveFailed(hash.clone())
            })?;

        match resp.status().as_u16() {
            200 => {
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|_| BlobRepositoryError::RetrieveFailed(hash.clone()))?
                    .to_vec();
                Ok(Some(Blob { bytes }))
            }
            404 => Ok(None),
            _ => Err(BlobRepositoryError::RetrieveFailed(hash.clone())),
        }
    }

    async fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError> {
        let url = format!(
            "{}/api/drives/{}/blobs/{}",
            self.base_url, self.drive_id, hash
        );

        let resp = self
            .client
            .head(&url)
            .header("Authorization", &self.auth)
            .send()
            .await
            .map_err(|_| BlobRepositoryError::ExistenceCheckFailed(hash.clone()))?;

        match resp.status().as_u16() {
            200 => Ok(true),
            404 => Ok(false),
            _ => Err(BlobRepositoryError::ExistenceCheckFailed(hash.clone())),
        }
    }

    async fn get_missing(&self, hashes: &[Hash]) -> Result<HashSet<Hash>, BlobRepositoryError> {
        if hashes.is_empty() {
            return Ok(HashSet::new());
        }

        if hashes.len() == 1 {
            let has_it = self.contains(&hashes[0]).await?;
            if has_it {
                return Ok(HashSet::new());
            } else {
                return Ok(HashSet::from([hashes[0].clone()]));
            }
        }

        let url = format!(
            "{}/api/drives/{}/blobs/missing",
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
                eprintln!("[http-blob] get_missing err={:?}", e);
                BlobRepositoryError::ExistenceCheckFailed(hashes[0].clone())
            })?;

        if !resp.status().is_success() {
            return Err(BlobRepositoryError::ExistenceCheckFailed(hashes[0].clone()));
        }

        let body: MissingResponse = resp.json().await.map_err(|e| {
            eprintln!("[http-blob] get_missing parse err={:?}", e);
            BlobRepositoryError::ExistenceCheckFailed(hashes[0].clone())
        })?;

        let mut missing = HashSet::new();
        for h in body.missing {
            missing.insert(h.clone());
        }

        Ok(missing)
    }
}

#[async_trait]
impl WritableBlobRepository for HttpBlobRepository {
    async fn insert(&self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        let url = format!(
            "{}/api/drives/{}/blobs/{}",
            self.base_url, self.drive_id, hash
        );

        let resp = self
            .client
            .put(&url)
            .header("Authorization", &self.auth)
            .header("Content-Type", "application/octet-stream")
            .body(blob.bytes)
            .send()
            .await
            .map_err(|e| {
                eprintln!("[http-blob] insert err={:?}", e);
                BlobRepositoryError::InsertFailed(hash.clone())
            })?;

        if resp.status().is_success() {
            Ok(())
        } else if resp.status() == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
            Err(BlobRepositoryError::UploadRejected {
                hash,
                reason: "Blob too large".to_string(),
            })
        } else {
            Err(BlobRepositoryError::InsertFailed(hash))
        }
    }
}

impl HttpBlobRepository {
    pub fn set_tracker(&mut self, tracker: crate::telemetry::ProgressTracker) {
        *self.tracker.write().unwrap() = Some(tracker);
    }

    pub fn len(&self) -> usize {
        self.inserted.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
