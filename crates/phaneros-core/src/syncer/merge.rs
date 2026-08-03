use std::collections::{BTreeSet, HashMap, HashSet};

use phaneros_sync::hash::Hash;

use crate::{
    blob_repository::{BlobRepository, WritableBlobRepository},
    node_repository::{Entry, Node, NodeRepository, WritableNodeRepository},
    syncer::{SyncError, diff::compute_unidirectional_diff},
};

#[derive(Default)]
pub struct MergePlan {
    /// sourced from remote repo, written to local
    to_local_nodes: HashSet<Hash>,
    to_local_blobs: HashSet<Hash>,
    /// sourced from local repo, written to remote
    to_remote_nodes: HashSet<Hash>,
    to_remote_blobs: HashSet<Hash>,
    /// freshly built merged folders — insert into BOTH sides (child-before-parent order)
    built_nodes: Vec<(Hash, Node)>,
    /// dedupe companion for built_nodes while preserving Vec insertion order
    built_node_hashes: HashSet<Hash>,
}

impl MergePlan {
    fn push_built_node(&mut self, hash: Hash, node: Node) {
        if self.built_node_hashes.insert(hash.clone()) {
            self.built_nodes.push((hash, node));
        }
    }

    fn reconciled_nodes(&self) -> usize {
        self.to_local_nodes.len() + self.to_remote_nodes.len() + self.built_nodes.len()
    }
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

type EntryMap = HashMap<String, EntryRef>;

pub fn merge(
    local_node_repository: &mut impl WritableNodeRepository,
    remote_node_repository: &mut impl WritableNodeRepository,
    local_blob_repository: &mut impl WritableBlobRepository,
    remote_blob_repository: &mut impl WritableBlobRepository,
    base_root_hash: &Hash,
    local_root_hash: &Hash,
    remote_root_hash: &Hash,
) -> Result<usize, SyncError> {
    let mut plan = MergePlan::default();

    let merged_root_hash = {
        let local_nodes = &*local_node_repository;
        let remote_nodes = &*remote_node_repository;
        let local_blobs = &*local_blob_repository;
        let remote_blobs = &*remote_blob_repository;

        merge_folder(
            Some(base_root_hash),
            Some(local_root_hash),
            Some(remote_root_hash),
            local_nodes,
            remote_nodes,
            local_blobs,
            remote_blobs,
            &mut plan,
        )?
        .ok_or_else(|| SyncError::MissingSourceNode {
            hash: local_root_hash.clone(),
        })?
    };

    for hash in &plan.to_local_blobs {
        let blob = remote_blob_repository
            .get_blob(hash)?
            .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
        local_blob_repository.insert(hash.clone(), blob)?;
    }
    for hash in &plan.to_remote_blobs {
        let blob = local_blob_repository
            .get_blob(hash)?
            .ok_or_else(|| SyncError::MissingSourceBlob { hash: hash.clone() })?;
        remote_blob_repository.insert(hash.clone(), blob)?;
    }

    for hash in &plan.to_local_nodes {
        let node = remote_node_repository
            .get_node(hash)?
            .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
        local_node_repository.insert(hash.clone(), node)?;
    }
    for hash in &plan.to_remote_nodes {
        let node = local_node_repository
            .get_node(hash)?
            .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
        remote_node_repository.insert(hash.clone(), node)?;
    }
    for (hash, node) in &plan.built_nodes {
        local_node_repository.insert(hash.clone(), node.clone())?;
        remote_node_repository.insert(hash.clone(), node.clone())?;
    }

    local_node_repository.set_root(merged_root_hash.clone())?;
    remote_node_repository.set_root(merged_root_hash)?;

    Ok(plan.reconciled_nodes())
}

fn merge_folder(
    base_hash: Option<&Hash>,
    local_hash: Option<&Hash>,
    remote_hash: Option<&Hash>,
    local_node_repository: &impl NodeRepository,
    remote_node_repository: &impl NodeRepository,
    local_blob_repository: &impl BlobRepository,
    remote_blob_repository: &impl BlobRepository,
    plan: &mut MergePlan,
) -> Result<Option<Hash>, SyncError> {
    // Both sides agree (also covers all-absent).
    if local_hash == remote_hash {
        return Ok(local_hash.cloned());
    }

    // Local untouched relative to base -> accept remote (or deletion).
    if base_hash == local_hash {
        if let Some(remote_hash) = remote_hash {
            queue_remote_subtree_for_local(
                remote_hash,
                local_node_repository,
                remote_node_repository,
                local_blob_repository,
                plan,
            )?;
            return Ok(Some(remote_hash.clone()));
        }
        return Ok(None);
    }

    // Remote untouched relative to base -> accept local (or deletion).
    if base_hash == remote_hash {
        if let Some(local_hash) = local_hash {
            queue_local_subtree_for_remote(
                local_hash,
                local_node_repository,
                remote_node_repository,
                remote_blob_repository,
                plan,
            )?;
            return Ok(Some(local_hash.clone()));
        }
        return Ok(None);
    }

    // Diverged folder-level merge.
    let base_entries = load_folder_entries(
        base_hash,
        NodeSource::Any,
        local_node_repository,
        remote_node_repository,
    )?;
    let local_entries = load_folder_entries(
        local_hash,
        NodeSource::Local,
        local_node_repository,
        remote_node_repository,
    )?;
    let remote_entries = load_folder_entries(
        remote_hash,
        NodeSource::Remote,
        local_node_repository,
        remote_node_repository,
    )?;

    let mut names = BTreeSet::new();
    names.extend(base_entries.keys().cloned());
    names.extend(local_entries.keys().cloned());
    names.extend(remote_entries.keys().cloned());

    let blocked_original_names: HashSet<String> = names.iter().cloned().collect();

    let mut merged_folders = Vec::new();
    let mut merged_files = Vec::new();
    let mut used_output_names = HashSet::new();

    for name in names {
        let base_entry = base_entries.get(&name);
        let local_entry = local_entries.get(&name);
        let remote_entry = remote_entries.get(&name);

        let b_hash = base_entry.map(|entry| &entry.hash);
        let l_hash = local_entry.map(|entry| &entry.hash);
        let r_hash = remote_entry.map(|entry| &entry.hash);

        // L == R: both sides agree (including both absent).
        if l_hash == r_hash {
            if let Some(chosen) = local_entry.or(remote_entry) {
                let stable_name = reserve_exact_name(&name, &mut used_output_names);
                push_entry(
                    stable_name,
                    chosen.hash.clone(),
                    chosen.kind,
                    &mut merged_folders,
                    &mut merged_files,
                );
            }
            continue;
        }

        // B == L, R differs/present -> take R. If R absent, this is remote delete.
        if b_hash == l_hash {
            if let Some(chosen) = remote_entry {
                queue_remote_subtree_for_local(
                    &chosen.hash,
                    local_node_repository,
                    remote_node_repository,
                    local_blob_repository,
                    plan,
                )?;
                let stable_name = reserve_exact_name(&name, &mut used_output_names);
                push_entry(
                    stable_name,
                    chosen.hash.clone(),
                    chosen.kind,
                    &mut merged_folders,
                    &mut merged_files,
                );
            }
            continue;
        }

        // B == R, L differs/present -> take L. If L absent, this is local delete.
        if b_hash == r_hash {
            if let Some(chosen) = local_entry {
                queue_local_subtree_for_remote(
                    &chosen.hash,
                    local_node_repository,
                    remote_node_repository,
                    remote_blob_repository,
                    plan,
                )?;
                let stable_name = reserve_exact_name(&name, &mut used_output_names);
                push_entry(
                    stable_name,
                    chosen.hash.clone(),
                    chosen.kind,
                    &mut merged_folders,
                    &mut merged_files,
                );
            }
            continue;
        }

        match (local_entry, remote_entry) {
            // Modify/modify conflict: recurse for folder|folder, split otherwise.
            (Some(local), Some(remote)) => {
                match (local.kind, remote.kind) {
                    (EntryKind::Folder, EntryKind::Folder) => {
                        let base_folder_hash = base_entry
                            .filter(|entry| entry.kind == EntryKind::Folder)
                            .map(|entry| &entry.hash);

                        if let Some(merged_hash) = merge_folder(
                            base_folder_hash,
                            Some(&local.hash),
                            Some(&remote.hash),
                            local_node_repository,
                            remote_node_repository,
                            local_blob_repository,
                            remote_blob_repository,
                            plan,
                        )? {
                            let stable_name = reserve_exact_name(&name, &mut used_output_names);
                            push_entry(
                                stable_name,
                                merged_hash,
                                EntryKind::Folder,
                                &mut merged_folders,
                                &mut merged_files,
                            );
                        }
                    }
                    _ => {
                        // Keep both: `name -> local`, `name.conflict -> remote`.
                        queue_local_subtree_for_remote(
                            &local.hash,
                            local_node_repository,
                            remote_node_repository,
                            remote_blob_repository,
                            plan,
                        )?;
                        queue_remote_subtree_for_local(
                            &remote.hash,
                            local_node_repository,
                            remote_node_repository,
                            local_blob_repository,
                            plan,
                        )?;

                        let stable_name = reserve_exact_name(&name, &mut used_output_names);
                        push_entry(
                            stable_name,
                            local.hash.clone(),
                            local.kind,
                            &mut merged_folders,
                            &mut merged_files,
                        );

                        let conflict_name = reserve_suffixed_name(
                            &name,
                            "conflict",
                            &blocked_original_names,
                            &mut used_output_names,
                        );
                        push_entry(
                            conflict_name,
                            remote.hash.clone(),
                            remote.kind,
                            &mut merged_folders,
                            &mut merged_files,
                        );
                    }
                }
            }
            // Delete/modify conflict: preserve edited side under `.conflict-delete`.
            (Some(local), None) => {
                queue_local_subtree_for_remote(
                    &local.hash,
                    local_node_repository,
                    remote_node_repository,
                    remote_blob_repository,
                    plan,
                )?;

                let conflict_name = reserve_suffixed_name(
                    &name,
                    "conflict-delete",
                    &blocked_original_names,
                    &mut used_output_names,
                );
                push_entry(
                    conflict_name,
                    local.hash.clone(),
                    local.kind,
                    &mut merged_folders,
                    &mut merged_files,
                );
            }
            (None, Some(remote)) => {
                queue_remote_subtree_for_local(
                    &remote.hash,
                    local_node_repository,
                    remote_node_repository,
                    local_blob_repository,
                    plan,
                )?;

                let conflict_name = reserve_suffixed_name(
                    &name,
                    "conflict-delete",
                    &blocked_original_names,
                    &mut used_output_names,
                );
                push_entry(
                    conflict_name,
                    remote.hash.clone(),
                    remote.kind,
                    &mut merged_folders,
                    &mut merged_files,
                );
            }
            (None, None) => {
                // Covered by L == R, included for completeness.
            }
        }
    }

    let (merged_hash, merged_node) = Node::folder(merged_folders, merged_files);
    plan.push_built_node(merged_hash.clone(), merged_node);

    Ok(Some(merged_hash))
}

#[derive(Clone, Copy)]
enum NodeSource {
    Local,
    Remote,
    Any,
}

fn load_folder_entries(
    hash: Option<&Hash>,
    source: NodeSource,
    local_node_repository: &impl NodeRepository,
    remote_node_repository: &impl NodeRepository,
) -> Result<EntryMap, SyncError> {
    let Some(hash) = hash else {
        return Ok(HashMap::new());
    };

    let node = match source {
        NodeSource::Local => local_node_repository.get_node(hash)?,
        NodeSource::Remote => remote_node_repository.get_node(hash)?,
        NodeSource::Any => local_node_repository
            .get_node(hash)?
            .or(remote_node_repository.get_node(hash)?),
    }
    .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;

    let Node::Folder { folders, files } = node else {
        return Err(SyncError::MissingSourceNode { hash: hash.clone() });
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

fn push_entry(
    name: String,
    hash: Hash,
    kind: EntryKind,
    folders: &mut Vec<Entry>,
    files: &mut Vec<Entry>,
) {
    let entry = Entry::new(name, hash);
    match kind {
        EntryKind::Folder => folders.push(entry),
        EntryKind::File => files.push(entry),
    }
}

fn reserve_exact_name(name: &str, used_names: &mut HashSet<String>) -> String {
    if used_names.insert(name.to_string()) {
        return name.to_string();
    }

    let mut index = 1usize;
    loop {
        let candidate = format!("{}.{}", name, index);
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        index += 1;
    }
}

fn reserve_suffixed_name(
    base_name: &str,
    suffix: &str,
    blocked_original_names: &HashSet<String>,
    used_names: &mut HashSet<String>,
) -> String {
    let first_candidate = format!("{}.{}", base_name, suffix);
    if !blocked_original_names.contains(&first_candidate)
        && used_names.insert(first_candidate.clone())
    {
        return first_candidate;
    }

    let mut index = 1usize;
    loop {
        let candidate = format!("{}.{}.{}", base_name, suffix, index);
        if !blocked_original_names.contains(&candidate) && used_names.insert(candidate.clone()) {
            return candidate;
        }
        index += 1;
    }
}

fn queue_local_subtree_for_remote(
    root_hash: &Hash,
    local_node_repository: &impl NodeRepository,
    remote_node_repository: &impl NodeRepository,
    remote_blob_repository: &impl BlobRepository,
    plan: &mut MergePlan,
) -> Result<(), SyncError> {
    let (nodes, blobs) = compute_unidirectional_diff(
        local_node_repository,
        remote_node_repository,
        remote_blob_repository,
        root_hash,
    )?;
    plan.to_remote_nodes.extend(nodes);
    plan.to_remote_blobs.extend(blobs);
    Ok(())
}

fn queue_remote_subtree_for_local(
    root_hash: &Hash,
    local_node_repository: &impl NodeRepository,
    remote_node_repository: &impl NodeRepository,
    local_blob_repository: &impl BlobRepository,
    plan: &mut MergePlan,
) -> Result<(), SyncError> {
    let (nodes, blobs) = compute_unidirectional_diff(
        remote_node_repository,
        local_node_repository,
        local_blob_repository,
        root_hash,
    )?;
    plan.to_local_nodes.extend(nodes);
    plan.to_local_blobs.extend(blobs);
    Ok(())
}
