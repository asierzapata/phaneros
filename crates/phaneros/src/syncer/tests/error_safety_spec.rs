use crate::blob_repository::InMemoryBlobRepository;
use crate::node_repository::NodeRepository;
use crate::syncer::{SyncError, local_push, remote_pull};

use super::fixtures::{TestStore, assert_missing_node};

#[test]
fn local_push_missing_source_blob_aborts_before_root_flip() {
    // Remote starts on a healthy old version.
    let mut remote = TestStore::new();
    let old_file = remote.add_file("doc.txt", b"old");
    let old_root = remote.add_folder("root", vec![], vec![old_file]);
    remote.nodes.set_root(old_root.hash.clone()).unwrap();

    // Local has a new version, but its blob store is missing the bytes
    // the new file node references (scanner bug, eviction, corruption...).
    let mut local = TestStore::new();
    let file = local.add_file("doc.txt", b"new bytes");
    let root = local.add_folder("root", vec![], vec![file.clone()]);
    local.blobs = InMemoryBlobRepository::new(); // sabotage: wipe the bytes

    let result = local_push(
        &local.nodes,
        &mut remote.nodes,
        &local.blobs,
        &mut remote.blobs,
        &root.hash,
    );

    // The sync reports the missing blob...
    assert!(matches!(result, Err(SyncError::MissingSourceBlob { .. })));
    // ...and the actual invariant: the remote's visible tree is untouched.
    assert_eq!(remote.nodes.root_hash().unwrap(), Some(&old_root.hash));
    assert_missing_node(&remote.nodes, &root.hash);
    assert_missing_node(&remote.nodes, &file.hash);
}

#[test]
fn remote_pull_missing_source_blob_aborts_before_root_flip() {
    // Local starts on a healthy old version.
    let mut local = TestStore::new();
    let old_file = local.add_file("doc.txt", b"old");
    let old_root = local.add_folder("root", vec![], vec![old_file]);
    local.nodes.set_root(old_root.hash.clone()).unwrap();

    // Remote has a new version but is missing required blob bytes.
    let mut remote = TestStore::new();
    let file = remote.add_file("doc.txt", b"new bytes");
    let root = remote.add_folder("root", vec![], vec![file.clone()]);
    remote.blobs = InMemoryBlobRepository::new(); // sabotage: wipe the bytes

    let result = remote_pull(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &root.hash,
    );

    assert!(matches!(result, Err(SyncError::MissingSourceBlob { .. })));
    assert_eq!(local.nodes.root_hash().unwrap(), Some(&old_root.hash));
    assert_missing_node(&local.nodes, &root.hash);
    assert_missing_node(&local.nodes, &file.hash);
}

#[test]
fn merge_not_implemented_returns_error_without_mutation() {
    let mut local = TestStore::new();
    let local_file = local.add_file("local.txt", b"local");
    let local_root = local.add_folder("root", vec![], vec![local_file.clone()]);
    local.nodes.set_root(local_root.hash.clone()).unwrap();

    let mut remote = TestStore::new();
    let remote_file = remote.add_file("remote.txt", b"remote");
    let remote_root = remote.add_folder("root", vec![], vec![remote_file.clone()]);
    remote.nodes.set_root(remote_root.hash.clone()).unwrap();

    let base_root = "base-root".to_string();

    let local_nodes_before = local.nodes.len();
    let remote_nodes_before = remote.nodes.len();
    let local_blobs_before = local.blobs.len();
    let remote_blobs_before = remote.blobs.len();
    let local_root_before = local.nodes.root_hash().unwrap().cloned();
    let remote_root_before = remote.nodes.root_hash().unwrap().cloned();

    let result = super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root,
        &local_root.hash,
        &remote_root.hash,
    );

    assert!(matches!(result, Err(SyncError::MergeNotImplemented)));
    assert_eq!(local.nodes.len(), local_nodes_before);
    assert_eq!(remote.nodes.len(), remote_nodes_before);
    assert_eq!(local.blobs.len(), local_blobs_before);
    assert_eq!(remote.blobs.len(), remote_blobs_before);
    assert_eq!(local.nodes.root_hash().unwrap().cloned(), local_root_before);
    assert_eq!(
        remote.nodes.root_hash().unwrap().cloned(),
        remote_root_before
    );
}
