use phaneros_sync::node::NodeWire;
use serde::{Deserialize, Serialize};

use crate::node_repository::{
    Hash, Node, NodeRepository, WritableNodeRepository, repository::NodeRepositoryError,
};

#[derive(Debug)]
pub struct HttpNodeRepository {
    agent: ureq::Agent,
    base_url: String,
    drive_id: String,
    auth: String,
    /// The root the client believes is currently stored, used as the `expected`
    /// value for compare-and-swap on `set_root`. Seeded from `GET /root` at
    /// construction, advanced on every accepted PUT, and corrected from the
    /// store's response when a PUT loses the race (409).
    cached_root: Option<Hash>,
    /// Count of nodes this session has PUT to the store. The remote total is not
    /// cheaply knowable over HTTP, so this backs `len()` purely for the syncer's
    /// transfer-count logging.
    inserted: usize,
}

#[derive(Deserialize)]
struct RootResponse {
    hash: Option<String>,
}

#[derive(Serialize)]
struct PutRootBody<'a> {
    hash: &'a Hash,
    expected: Option<&'a Hash>,
}

impl HttpNodeRepository {
    pub fn new(
        base_url: impl Into<String>,
        drive_id: impl Into<String>,
        token: impl AsRef<str>,
    ) -> Self {
        let mut repo = Self {
            agent: ureq::Agent::new(),
            base_url: base_url.into(),
            drive_id: drive_id.into(),
            auth: format!("Bearer {}", token.as_ref()),
            cached_root: None,
            inserted: 0,
        };
        // Best-effort seed of the expected root. If the store is unreachable or
        // has no root yet we start from None. Any subsequent 409 corrects it.
        match repo.fetch_root() {
            Ok(root) => repo.cached_root = root,
            Err(err) => eprintln!(
                "HttpNodeRepository: could not seed root from store: {}",
                err
            ),
        }
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

    fn fetch_root(&self) -> Result<Option<Hash>, NodeRepositoryError> {
        match self
            .agent
            .get(&self.root_url())
            .set("Authorization", &self.auth)
            .call()
        {
            Ok(response) => {
                let body: RootResponse = response
                    .into_json()
                    .map_err(|_| NodeRepositoryError::RootRetrieveFailed)?;
                Ok(body.hash)
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(_) => Err(NodeRepositoryError::RootRetrieveFailed),
        }
    }

    pub fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        match self
            .agent
            .put(&self.nodes_url(&hash))
            .set("Authorization", &self.auth)
            .send_json(&node)
        {
            Ok(_) => {
                self.inserted += 1;
                Ok(())
            }
            Err(_) => Err(NodeRepositoryError::InsertFailed(hash)),
        }
    }

    pub fn set_root(&mut self, hash: Hash) -> Result<(), NodeRepositoryError> {
        let body = PutRootBody {
            hash: &hash,
            expected: self.cached_root.as_ref(),
        };
        match self
            .agent
            .put(&self.root_url())
            .set("Authorization", &self.auth)
            .send_json(&body)
        {
            Ok(_) => {
                self.cached_root = Some(hash);
                Ok(())
            }
            Err(ureq::Error::Status(409, response)) => {
                // This error signals a lost race, so we have to adopt the store's
                // expected so the next reconcile can succeed, and surface a
                // current root as the new distinct error so the caller does not retry this PUT.
                let actual = response
                    .into_json::<RootResponse>()
                    .ok()
                    .and_then(|body| body.hash);
                self.cached_root = actual.clone();
                Err(NodeRepositoryError::RootConflict { actual })
            }
            Err(_) => Err(NodeRepositoryError::SetRootFailed(hash)),
        }
    }

    pub fn len(&self) -> usize {
        self.inserted
    }

    pub fn is_empty(&self) -> bool {
        self.inserted == 0
    }
}

impl NodeRepository for HttpNodeRepository {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeRepositoryError> {
        Ok(self.cached_root.as_ref())
    }

    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        match self
            .agent
            .get(&self.nodes_url(hash))
            .set("Authorization", &self.auth)
            .call()
        {
            Ok(response) => {
                let wire: NodeWire = response
                    .into_json()
                    .map_err(|_| NodeRepositoryError::NodeRetrieveFailed(hash.clone()))?;
                let (_, node) = wire.reconstruct();
                Ok(Some(node))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(_) => Err(NodeRepositoryError::NodeRetrieveFailed(hash.clone())),
        }
    }
}

impl WritableNodeRepository for HttpNodeRepository {
    fn insert(&mut self, hash: Hash, node: Node) -> Result<(), NodeRepositoryError> {
        HttpNodeRepository::insert(self, hash, node)
    }

    fn set_root(&mut self, hash: Hash) -> Result<(), NodeRepositoryError> {
        HttpNodeRepository::set_root(self, hash)
    }
}
