use std::collections::HashSet;

use phaneros_sync::hash::Hash;

use crate::{blob_repository::BlobRepository, node_repository::NodeRepository, syncer::SyncError};

pub type TransferSet = (HashSet<Hash>, HashSet<Hash>);

/// Computes a directional transfer set: every node reachable from `root_hash` in
/// `source` that `target` does not have, plus every blob those file nodes
/// reference that `target_blob_repository` does not have. When the target already
/// has a node, its entire subtree is pruned from the walk:
/// reconcile writes blobs before nodes, so a node's presence on the target
/// implies its blobs' presence.
pub fn compute_unidirectional_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
) -> Result<TransferSet, SyncError> {
    compute_directional_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        root_hash,
    )
}

/// Computes transfer sets in both directions:
///
/// - first tuple entry: `source -> target` rooted at `source_root_hash`
/// - second tuple entry: `target -> source` rooted at `target_root_hash`
///
/// This keeps each side's root independent, which is important when stores have
/// diverged.
pub fn compute_bidirectional_diff(
    source_node_repository: &impl NodeRepository,
    source_blob_repository: &impl BlobRepository,
    source_root_hash: &Hash,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    target_root_hash: &Hash,
) -> Result<(TransferSet, TransferSet), SyncError> {
    let source_to_target = compute_directional_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        source_root_hash,
    )?;

    let target_to_source = compute_directional_diff(
        target_node_repository,
        source_node_repository,
        source_blob_repository,
        target_root_hash,
    )?;

    Ok((source_to_target, target_to_source))
}

fn compute_directional_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
) -> Result<TransferSet, SyncError> {
    let mut node_transfer_set = HashSet::new();
    let mut blob_transfer_set = HashSet::new();

    if let Some(node) = source_node_repository.get_node(root_hash)? {
        match node {
            crate::node_repository::Node::Folder { .. } => {
                compute_folder_diff(
                    source_node_repository,
                    target_node_repository,
                    target_blob_repository,
                    root_hash,
                    &mut node_transfer_set,
                    &mut blob_transfer_set,
                )?;
            }
            crate::node_repository::Node::File { .. } => {
                compute_file_diff(
                    source_node_repository,
                    target_node_repository,
                    target_blob_repository,
                    root_hash,
                    &mut node_transfer_set,
                    &mut blob_transfer_set,
                )?;
            }
        }
    }

    Ok((node_transfer_set, blob_transfer_set))
}

pub fn compute_folder_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) -> Result<(), SyncError> {
    let Some(crate::node_repository::Node::Folder { folders, files }) =
        source_node_repository.get_node(root_hash)?
    else {
        return Ok(());
    };

    // We have to both check if the folder is not on the target node store
    // and neither has been visited to only transfer it once.
    // If we don't check the transfer set, we will transfer the same folder multiple times
    // if it is referenced by multiple folders.
    if target_node_repository.get_node(root_hash)?.is_none()
        && node_transfer_set.insert(root_hash.clone())
    {
        for folder in folders {
            compute_folder_diff(
                source_node_repository,
                target_node_repository,
                target_blob_repository,
                &folder.hash,
                node_transfer_set,
                blob_transfer_set,
            )?;
        }
        for file in files {
            compute_file_diff(
                source_node_repository,
                target_node_repository,
                target_blob_repository,
                &file.hash,
                node_transfer_set,
                blob_transfer_set,
            )?;
        }
    }

    Ok(())
}

pub fn compute_file_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) -> Result<(), SyncError> {
    let Some(crate::node_repository::Node::File { blobs }) =
        source_node_repository.get_node(root_hash)?
    else {
        return Ok(());
    };

    if target_node_repository.get_node(root_hash)?.is_none()
        && node_transfer_set.insert(root_hash.clone())
    {
        for blob_ref in blobs {
            if !target_blob_repository.contains(&blob_ref.hash)? {
                blob_transfer_set.insert(blob_ref.hash.clone());
            }
        }
    }

    Ok(())
}
