use serde::Deserialize;
use std::collections::HashSet;

use crate::blob_repository::{
    Blob, BlobRepository, Hash, WritableBlobRepository, repository::BlobRepositoryError,
};
use crate::utils::compression::{compress_blob, decompress_blob};

#[derive(Debug)]
pub struct HttpBlobRepository {
    agent: ureq::Agent,
    base_url: String,
    #[allow(dead_code)]
    drive_id: String,
    auth: String,
    /// Count of blobs this session has uploaded. The remote total is not
    /// cheaply knowable over HTTP, so this backs `len()` purely for the
    /// syncer's transfer-count logging.
    inserted: usize,
    progress_tracker: Option<crate::telemetry::ProgressTracker>,
}

#[derive(Deserialize)]
struct TicketResponse {
    url: String,
}

#[derive(Deserialize)]
struct MissingResponse {
    missing: HashSet<Hash>,
}

impl HttpBlobRepository {
    pub fn new(
        base_url: impl Into<String>,
        drive_id: impl Into<String>,
        token: impl AsRef<str>,
    ) -> Self {
        Self {
            agent: ureq::Agent::new(),
            base_url: base_url.into(),
            drive_id: drive_id.into(),
            auth: format!("Bearer {}", token.as_ref()),
            inserted: 0,
            progress_tracker: None,
        }
    }

    pub fn with_tracker(mut self, tracker: crate::telemetry::ProgressTracker) -> Self {
        self.progress_tracker = Some(tracker);
        self
    }

    pub fn set_tracker(&mut self, tracker: crate::telemetry::ProgressTracker) {
        self.progress_tracker = Some(tracker);
    }

    fn blob_url(&self, hash: &Hash) -> String {
        format!("{}/api/blobs/{}", self.base_url, hash)
    }

    fn upload_url(&self, hash: &Hash) -> String {
        format!("{}/api/blobs/{}/upload", self.base_url, hash)
    }

    fn commit_url(&self, hash: &Hash) -> String {
        format!("{}/api/blobs/{}/commit", self.base_url, hash)
    }

    fn download_url(&self, hash: &Hash) -> String {
        format!("{}/api/blobs/{}/download", self.base_url, hash)
    }

    pub fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        let (payload_bytes, compression) = compress_blob(&blob.bytes);
        if let Some(ref tracker) = self.progress_tracker {
            tracker.record_blob_compressed(
                blob.bytes.len() as u64,
                payload_bytes.len() as u64,
                compression == "zstd",
            );
        }
        let size = payload_bytes.len() as i64;

        let ticket_payload = if compression == "zstd" {
            serde_json::json!({
                "size": size,
                "uncompressed_size": blob.bytes.len() as i64,
                "compression": compression,
            })
        } else {
            serde_json::json!({
                "size": size,
                "compression": "none",
            })
        };

        // Step 1: Create upload ticket (or get 204 if already stored).
        let ticket_response = self
            .agent
            .post(&self.upload_url(&hash))
            .set("Authorization", &self.auth)
            .send_json(ticket_payload)
            .map_err(|e| {
                eprintln!(
                    "[http-blob] insert stage=ticket hash={} url={} err={:?}",
                    hash,
                    self.upload_url(&hash),
                    e
                );
                match e {
                    ureq::Error::Status(status, _) => BlobRepositoryError::UploadRejected {
                        hash: hash.clone(),
                        reason: format!("upload ticket returned {}", status),
                    },
                    _ => BlobRepositoryError::InsertFailed(hash.clone()),
                }
            })?;

        // 204 No Content = already stored, skip the rest.
        if ticket_response.status() == 204 {
            if let Some(ref tracker) = self.progress_tracker {
                tracker.record_blob_skipped_dedup(blob.bytes.len() as u64);
            }
            return Ok(());
        }

        let ticket: TicketResponse = ticket_response.into_json().map_err(|e| {
            eprintln!(
                "[http-blob] insert stage=ticket-parse hash={} err={:?}",
                hash, e
            );
            BlobRepositoryError::InsertFailed(hash.clone())
        })?;

        // Step 2: Upload the bytes to the ticket URL with Content-Encoding if compressed.
        let mut request = self
            .agent
            .put(&ticket.url)
            .set("Authorization", &self.auth)
            .set("Content-Type", "application/octet-stream");

        if compression == "zstd" {
            request = request.set("Content-Encoding", "zstd");
        }

        request.send_bytes(&payload_bytes).map_err(|e| {
            eprintln!(
                "[http-blob] insert stage=put-bytes hash={} ticket_url={} size={} err={:?}",
                hash,
                ticket.url,
                payload_bytes.len(),
                e
            );
            match e {
                ureq::Error::Status(status, _) => BlobRepositoryError::UploadRejected {
                    hash: hash.clone(),
                    reason: format!("put bytes returned {}", status),
                },
                _ => BlobRepositoryError::InsertFailed(hash.clone()),
            }
        })?;

        if let Some(ref tracker) = self.progress_tracker {
            tracker.record_bytes_sent(payload_bytes.len() as u64);
        }

        // Step 3: Commit the upload.
        self.agent
            .post(&self.commit_url(&hash))
            .set("Authorization", &self.auth)
            .call()
            .map_err(|e| {
                eprintln!(
                    "[http-blob] insert stage=commit hash={} url={} err={:?}",
                    hash,
                    self.commit_url(&hash),
                    e
                );
                match e {
                    ureq::Error::Status(status, _) => BlobRepositoryError::UploadRejected {
                        hash: hash.clone(),
                        reason: format!("commit returned {}", status),
                    },
                    _ => BlobRepositoryError::InsertFailed(hash.clone()),
                }
            })?;

        self.inserted += 1;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.inserted
    }

    pub fn is_empty(&self) -> bool {
        self.inserted == 0
    }
}

impl BlobRepository for HttpBlobRepository {
    fn get_blob(&self, hash: &Hash) -> Result<Option<Blob>, BlobRepositoryError> {
        // Step 1: Request a download ticket.
        let ticket_response = match self
            .agent
            .post(&self.download_url(hash))
            .set("Authorization", &self.auth)
            .call()
        {
            Ok(resp) => resp,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(e) => {
                eprintln!(
                    "[http-blob] get stage=ticket hash={} url={} err={:?}",
                    hash,
                    self.download_url(hash),
                    e
                );
                return Err(BlobRepositoryError::RetrieveFailed(hash.clone()));
            }
        };

        let ticket: TicketResponse = ticket_response.into_json().map_err(|e| {
            eprintln!(
                "[http-blob] get stage=ticket-parse hash={} err={:?}",
                hash, e
            );
            BlobRepositoryError::RetrieveFailed(hash.clone())
        })?;

        // Step 2: Download bytes from the ticket URL.
        let bytes_response = match self
            .agent
            .get(&ticket.url)
            .set("Authorization", &self.auth)
            .call()
        {
            Ok(resp) => resp,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(e) => {
                eprintln!(
                    "[http-blob] get stage=bytes hash={} ticket_url={} err={:?}",
                    hash, ticket.url, e
                );
                return Err(BlobRepositoryError::RetrieveFailed(hash.clone()));
            }
        };

        let encoding = bytes_response
            .header("Content-Encoding")
            .unwrap_or("none")
            .to_string();

        let mut raw_bytes = Vec::new();
        bytes_response
            .into_reader()
            .read_to_end(&mut raw_bytes)
            .map_err(|e| {
                eprintln!("[http-blob] get stage=read hash={} err={:?}", hash, e);
                BlobRepositoryError::RetrieveFailed(hash.clone())
            })?;

        let bytes = decompress_blob(&raw_bytes, &encoding).map_err(|e| {
            eprintln!(
                "[http-blob] get stage=decompress hash={} encoding={} err={:?}",
                hash, encoding, e
            );
            BlobRepositoryError::RetrieveFailed(hash.clone())
        })?;

        Ok(Some(Blob { bytes }))
    }

    fn contains(&self, hash: &Hash) -> Result<bool, BlobRepositoryError> {
        match self
            .agent
            .head(&self.blob_url(hash))
            .set("Authorization", &self.auth)
            .call()
        {
            Ok(_) => Ok(true),
            Err(ureq::Error::Status(404, _)) => Ok(false),
            Err(e) => {
                eprintln!(
                    "[http-blob] contains hash={} url={} err={:?}",
                    hash,
                    self.blob_url(hash),
                    e
                );
                Err(BlobRepositoryError::ExistenceCheckFailed(hash.clone()))
            }
        }
    }

    fn get_missing(&self, hashes: &[Hash]) -> Result<HashSet<Hash>, BlobRepositoryError> {
        if hashes.is_empty() {
            return Ok(HashSet::new());
        }

        let url = format!("{}/api/blobs/missing", self.base_url);
        let payload = serde_json::json!({ "hashes": hashes });

        let response = self
            .agent
            .post(&url)
            .set("Authorization", &self.auth)
            .send_json(payload)
            .map_err(|e| {
                eprintln!("[http-blob] get_missing err={:?}", e);
                BlobRepositoryError::ExistenceCheckFailed(hashes[0].clone())
            })?;

        let body: MissingResponse = response.into_json().map_err(|e| {
            eprintln!("[http-blob] get_missing parse err={:?}", e);
            BlobRepositoryError::ExistenceCheckFailed(hashes[0].clone())
        })?;

        Ok(body.missing)
    }
}

impl WritableBlobRepository for HttpBlobRepository {
    fn insert(&mut self, hash: Hash, blob: Blob) -> Result<(), BlobRepositoryError> {
        HttpBlobRepository::insert(self, hash, blob)
    }
}
