use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::blob_repository::{BlobRepository, BlobRepositoryError};
use crate::node_repository::{Hash, Node, NodeRepository, NodeRepositoryError};
use crate::scanner::file_chunker::{DEFAULT_CHUNK_SIZE, FileChunkerError, file_node_hash};
use crate::utils::filesystem_write::{is_internal_entry, move_to_trash, write_atomic};

#[derive(Debug, Error)]
pub enum MaterializeError {
    #[error("tree references node {hash} that is not in the local store")]
    MissingNode { hash: Hash },
    #[error("tree references blob {hash} that is not in the local store")]
    MissingBlob { hash: Hash },
    #[error("expected {hash} to be a folder node")]
    NotAFolder { hash: Hash },
    #[error("expected {hash} to be a file node")]
    NotAFile { hash: Hash },
    #[error("filesystem error at {path}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to hash local file {path}")]
    HashLocalFile {
        path: String,
        #[source]
        source: FileChunkerError,
    },
    #[error(transparent)]
    NodeRepository(#[from] NodeRepositoryError),
    #[error(transparent)]
    BlobRepository(#[from] BlobRepositoryError),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MaterializeStats {
    pub files_written: usize,
    pub folders_created: usize,
    pub entries_trashed: usize,
}

impl MaterializeStats {
    pub fn touched_nothing(&self) -> bool {
        *self == MaterializeStats::default()
    }
}

pub fn materialize(
    node_repository: &impl NodeRepository,
    blob_repository: &impl BlobRepository,
    vault_path: &Path,
    trash_batch: &Path,
    from_root: Option<&Hash>,
    to_root: &Hash,
) -> Result<MaterializeStats, MaterializeError> {
    let mut materializer = Materializer {
        node_repository,
        blob_repository,
        vault_path,
        trash_batch,
        stats: MaterializeStats::default(),
    };

    materializer.sync_folder(from_root, to_root, Path::new(""))?;

    Ok(materializer.stats)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryKind {
    Folder,
    File,
}

#[derive(Clone, Debug)]
struct EntryRef {
    hash: Hash,
    kind: EntryKind,
}

struct Materializer<'a, N: NodeRepository, B: BlobRepository> {
    node_repository: &'a N,
    blob_repository: &'a B,
    vault_path: &'a Path,
    trash_batch: &'a Path,
    stats: MaterializeStats,
}

impl<N: NodeRepository, B: BlobRepository> Materializer<'_, N, B> {
    fn sync_folder(
        &mut self,
        from: Option<&Hash>,
        to: &Hash,
        rel_dir: &Path,
    ) -> Result<(), MaterializeError> {
        if from == Some(to) {
            return Ok(());
        }

        self.create_dir(&self.vault_path.join(rel_dir))?;

        let from_entries = match from {
            Some(hash) => self.folder_entries(hash)?,
            None => HashMap::new(),
        };
        let to_entries = self.folder_entries(to)?;

        let mut names = BTreeSet::new();
        names.extend(from_entries.keys().cloned());
        names.extend(to_entries.keys().cloned());

        for name in names {
            if is_internal_entry(&name) {
                continue;
            }

            let rel_path = rel_dir.join(&name);
            let path = self.vault_path.join(&rel_path);
            let from_entry = from_entries.get(&name);
            let to_entry = to_entries.get(&name);

            match (from_entry, to_entry) {
                (from_entry, Some(to_entry)) => {
                    let same = from_entry.is_some_and(|from_entry| {
                        from_entry.kind == to_entry.kind && from_entry.hash == to_entry.hash
                    });
                    if same {
                        continue;
                    }

                    if from_entry.is_some_and(|from_entry| from_entry.kind != to_entry.kind) {
                        self.trash_path(&path, &rel_path)?;
                    }

                    match to_entry.kind {
                        EntryKind::Folder => {
                            let from_folder = from_entry
                                .filter(|entry| entry.kind == EntryKind::Folder)
                                .map(|entry| &entry.hash);
                            self.sync_folder(from_folder, &to_entry.hash, &rel_path)?;
                        }
                        EntryKind::File => {
                            let from_file = from_entry
                                .filter(|entry| entry.kind == EntryKind::File)
                                .map(|entry| &entry.hash);
                            self.write_file(&to_entry.hash, from_file, &path, &rel_path)?;
                        }
                    }
                }
                (Some(_), None) => self.trash_path(&path, &rel_path)?,
                (None, None) => unreachable!("name came from one of the two entry maps"),
            }
        }

        Ok(())
    }

    fn write_file(
        &mut self,
        to_hash: &Hash,
        from_hash: Option<&Hash>,
        path: &Path,
        rel_path: &Path,
    ) -> Result<(), MaterializeError> {
        // Whatever is on disk is only safe to overwrite if it still holds the
        // content `from` claims.
        if path.exists() && !self.get_disk_matches_hash(path, from_hash)? {
            if self.get_disk_matches_hash(path, Some(to_hash))? {
                return Ok(());
            }
            self.trash_path(path, rel_path)?;
        }

        let bytes = self.file_bytes(to_hash)?;
        write_atomic(path, &bytes).map_err(|source| MaterializeError::Io {
            path: path.display().to_string(),
            source,
        })?;
        self.stats.files_written += 1;

        Ok(())
    }

    fn get_disk_matches_hash(
        &self,
        path: &Path,
        from_hash: Option<&Hash>,
    ) -> Result<bool, MaterializeError> {
        let Some(from_hash) = from_hash else {
            return Ok(false);
        };

        let metadata = std::fs::metadata(path).map_err(|source| MaterializeError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if metadata.is_dir() {
            return Ok(false);
        }

        let Some(expected_size) = self.file_size(from_hash)? else {
            return Ok(false);
        };
        if metadata.len() != expected_size {
            return Ok(false);
        }

        let hash = file_node_hash(path, DEFAULT_CHUNK_SIZE).map_err(|source| {
            MaterializeError::HashLocalFile {
                path: path.display().to_string(),
                source,
            }
        })?;

        Ok(&hash == from_hash)
    }

    fn trash_path(&mut self, path: &Path, rel_path: &Path) -> Result<(), MaterializeError> {
        if !path.exists() {
            return Ok(());
        }

        move_to_trash(path, self.trash_batch, rel_path).map_err(|source| MaterializeError::Io {
            path: path.display().to_string(),
            source,
        })?;
        self.stats.entries_trashed += 1;

        Ok(())
    }

    fn create_dir(&mut self, path: &PathBuf) -> Result<(), MaterializeError> {
        if path.is_dir() {
            return Ok(());
        }

        std::fs::create_dir_all(path).map_err(|source| MaterializeError::Io {
            path: path.display().to_string(),
            source,
        })?;
        self.stats.folders_created += 1;

        Ok(())
    }

    fn folder_entries(&self, hash: &Hash) -> Result<HashMap<String, EntryRef>, MaterializeError> {
        let node = self
            .node_repository
            .get_node(hash)?
            .ok_or_else(|| MaterializeError::MissingNode { hash: hash.clone() })?;

        let Node::Folder { folders, files } = node else {
            return Err(MaterializeError::NotAFolder { hash: hash.clone() });
        };

        let mut entries = HashMap::with_capacity(folders.len() + files.len());
        for entry in folders {
            entries.insert(
                entry.name,
                EntryRef {
                    hash: entry.hash,
                    kind: EntryKind::Folder,
                },
            );
        }
        for entry in files {
            entries.insert(
                entry.name,
                EntryRef {
                    hash: entry.hash,
                    kind: EntryKind::File,
                },
            );
        }

        Ok(entries)
    }

    fn file_bytes(&self, hash: &Hash) -> Result<Vec<u8>, MaterializeError> {
        let blobs = self.file_blobs(hash)?;
        let total: u64 = blobs.iter().map(|blob_ref| blob_ref.size).sum();

        let mut bytes = Vec::with_capacity(total as usize);
        for blob_ref in blobs {
            let blob = self
                .blob_repository
                .get_blob(&blob_ref.hash)?
                .ok_or_else(|| MaterializeError::MissingBlob {
                    hash: blob_ref.hash.clone(),
                })?;
            bytes.extend_from_slice(&blob.bytes);
        }

        Ok(bytes)
    }

    fn file_size(&self, hash: &Hash) -> Result<Option<u64>, MaterializeError> {
        match self.node_repository.get_node(hash)? {
            Some(Node::File { blobs }) => Ok(Some(blobs.iter().map(|blob| blob.size).sum())),
            _ => Ok(None),
        }
    }

    fn file_blobs(
        &self,
        hash: &Hash,
    ) -> Result<Vec<crate::blob_repository::BlobRef>, MaterializeError> {
        let node = self
            .node_repository
            .get_node(hash)?
            .ok_or_else(|| MaterializeError::MissingNode { hash: hash.clone() })?;

        match node {
            Node::File { blobs } => Ok(blobs),
            Node::Folder { .. } => Err(MaterializeError::NotAFile { hash: hash.clone() }),
        }
    }
}
