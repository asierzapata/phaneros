use std::collections::{HashMap, HashSet, VecDeque};

use phaneros_sync::hash::Hash;

use crate::{
    blob_repository::BlobRepository,
    node_repository::{Node, NodeRepository},
    syncer::SyncError,
};

pub type TransferSet = (HashSet<Hash>, HashSet<Hash>, HashMap<Hash, Node>);

pub async fn compute_unidirectional_diff(
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
    ).await
}

pub async fn compute_bidirectional_diff(
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
    ).await?;

    let target_to_source = compute_directional_diff(
        target_node_repository,
        source_node_repository,
        source_blob_repository,
        target_root_hash,
    ).await?;

    Ok((source_to_target, target_to_source))
}

async fn compute_directional_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
) -> Result<TransferSet, SyncError> {
    let mut node_transfer_set = HashSet::new();
    let mut node_cache = HashMap::new();
    let mut pending_nodes = VecDeque::new();
    let mut pending_blobs = HashSet::new();

    pending_nodes.push_back(root_hash.clone());

    while !pending_nodes.is_empty() {
        let batch: Vec<Hash> = pending_nodes.drain(..).collect();
        let missing = target_node_repository.get_missing(&batch).await?;
        let missing_vec: Vec<Hash> = missing.iter().cloned().collect();

        if missing_vec.is_empty() {
            continue;
        }

        let fetched_nodes = source_node_repository.get_nodes_batch(&missing_vec).await?;
        for hash in missing_vec {
            let node = fetched_nodes
                .get(&hash)
                .ok_or_else(|| SyncError::MissingSourceNode { hash: hash.clone() })?;
            
            if node_transfer_set.insert(hash.clone()) {
                node_cache.insert(hash.clone(), node.clone());
                match node {
                    Node::Folder { folders, files } => {
                        for folder in folders {
                            if !node_transfer_set.contains(&folder.hash) {
                                pending_nodes.push_back(folder.hash.clone());
                            }
                        }
                        for file in files {
                            if !node_transfer_set.contains(&file.hash) {
                                pending_nodes.push_back(file.hash.clone());
                            }
                        }
                    }
                    Node::File { blobs } => {
                        for blob_ref in blobs {
                            pending_blobs.insert(blob_ref.hash.clone());
                        }
                    }
                }
            }
        }
    }

    let blob_hashes_vec: Vec<Hash> = pending_blobs.into_iter().collect();
    let blob_transfer_set = target_blob_repository.get_missing(&blob_hashes_vec).await?;

    Ok((node_transfer_set, blob_transfer_set, node_cache))
}

// Keep these functions available for merge.rs tests
pub async fn compute_folder_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) -> Result<(), SyncError> {
    let (nodes, blobs, _) = compute_directional_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        root_hash,
    ).await?;
    node_transfer_set.extend(nodes);
    blob_transfer_set.extend(blobs);
    Ok(())
}

pub async fn compute_file_diff(
    source_node_repository: &impl NodeRepository,
    target_node_repository: &impl NodeRepository,
    target_blob_repository: &impl BlobRepository,
    root_hash: &Hash,
    node_transfer_set: &mut HashSet<Hash>,
    blob_transfer_set: &mut HashSet<Hash>,
) -> Result<(), SyncError> {
    let (nodes, blobs, _) = compute_directional_diff(
        source_node_repository,
        target_node_repository,
        target_blob_repository,
        root_hash,
    ).await?;
    node_transfer_set.extend(nodes);
    blob_transfer_set.extend(blobs);
    Ok(())
}
