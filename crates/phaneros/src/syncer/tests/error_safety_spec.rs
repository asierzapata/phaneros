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
fn merge_missing_source_blob_aborts_before_root_flip() {
    // Shared base.
    let mut local = TestStore::new();
    let local_base_file = local.add_file("doc.txt", b"v1");
    let local_base_root = local.add_folder("root", vec![], vec![local_base_file]);
    let local_edit = local.add_file("doc.txt", b"local-v2");
    let local_root = local.add_folder("root", vec![], vec![local_edit]);
    local.nodes.set_root(local_root.hash.clone()).unwrap();

    let mut remote = TestStore::new();
    let remote_base_file = remote.add_file("doc.txt", b"v1");
    let remote_base_root = remote.add_folder("root", vec![], vec![remote_base_file]);
    let remote_edit = remote.add_file("doc.txt", b"remote-v2");
    let remote_root = remote.add_folder("root", vec![], vec![remote_edit]);
    remote.nodes.set_root(remote_root.hash.clone()).unwrap();

    assert_eq!(local_base_root.hash, remote_base_root.hash);

    // Sabotage local blob source: local->remote leg of merge apply must fail,
    // and roots must stay untouched.
    local.blobs = InMemoryBlobRepository::new();

    let local_root_before = local.nodes.root_hash().unwrap().cloned();
    let remote_root_before = remote.nodes.root_hash().unwrap().cloned();

    let result = super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &local_base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    );

    assert!(matches!(result, Err(SyncError::MissingSourceBlob { .. })));
    assert_eq!(local.nodes.root_hash().unwrap().cloned(), local_root_before);
    assert_eq!(
        remote.nodes.root_hash().unwrap().cloned(),
        remote_root_before
    );
}
