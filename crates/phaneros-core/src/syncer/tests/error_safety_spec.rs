use crate::node_repository::WritableNodeRepository;
use crate::blob_repository::InMemoryBlobRepository;
use crate::node_repository::NodeRepository;
use crate::syncer::{SyncError, local_push, remote_pull};

use super::fixtures::{TestStore, assert_missing_node};

#[tokio::test]
async fn local_push_missing_source_blob_aborts_before_root_flip() {
    // Remote starts on a healthy old version.
    let mut remote = TestStore::new();
    let old_file = remote.add_file("doc.txt", b"old").await;
    let old_root = remote.add_folder("root", vec![], vec![old_file]).await;
    remote.nodes.set_root(old_root.hash.clone()).await.unwrap();

    // Local has a new version, but its blob store is missing the bytes
    // the new file node references (scanner bug, eviction, corruption...).
    let mut local = TestStore::new();
    let file = local.add_file("doc.txt", b"new bytes").await;
    let root = local.add_folder("root", vec![], vec![file.clone()]).await;
    local.blobs = InMemoryBlobRepository::new(); // sabotage: wipe the bytes

    let result = local_push(
        &local.nodes,
        &mut remote.nodes,
        &local.blobs,
        &mut remote.blobs,
        &root.hash,
    );

    // The sync reports the missing blob...
    assert!(matches!(result.await, Err(SyncError::MissingSourceBlob { .. })));
    // ...and the actual invariant: the remote's visible tree is untouched.
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(old_root.hash.clone()));
    assert_missing_node(&remote.nodes, &root.hash).await;
    assert_missing_node(&remote.nodes, &file.hash).await;
}

#[tokio::test]
async fn remote_pull_missing_source_blob_aborts_before_root_flip() {
    // Local starts on a healthy old version.
    let mut local = TestStore::new();
    let old_file = local.add_file("doc.txt", b"old").await;
    let old_root = local.add_folder("root", vec![], vec![old_file]).await;
    local.nodes.set_root(old_root.hash.clone()).await.unwrap();

    // Remote has a new version but is missing required blob bytes.
    let mut remote = TestStore::new();
    let file = remote.add_file("doc.txt", b"new bytes").await;
    let root = remote.add_folder("root", vec![], vec![file.clone()]).await;
    remote.blobs = InMemoryBlobRepository::new(); // sabotage: wipe the bytes

    let result = remote_pull(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &root.hash,
    );

    assert!(matches!(result.await, Err(SyncError::MissingSourceBlob { .. })));
    assert_eq!(local.nodes.root_hash().await.unwrap(), Some(old_root.hash.clone()));
    assert_missing_node(&local.nodes, &root.hash).await;
    assert_missing_node(&local.nodes, &file.hash).await;
}

#[tokio::test]
async fn merge_missing_source_blob_aborts_before_root_flip() {
    // Shared base.
    let mut local = TestStore::new();
    let local_base_file = local.add_file("doc.txt", b"v1").await;
    let local_base_root = local.add_folder("root", vec![], vec![local_base_file]).await;
    let local_edit = local.add_file("doc.txt", b"local-v2").await;
    let local_root = local.add_folder("root", vec![], vec![local_edit]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let remote_base_file = remote.add_file("doc.txt", b"v1").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![remote_base_file]).await;
    let remote_edit = remote.add_file("doc.txt", b"remote-v2").await;
    let remote_root = remote.add_folder("root", vec![], vec![remote_edit]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(local_base_root.hash, remote_base_root.hash);

    // Sabotage local blob source: local->remote leg of merge apply must fail,
    // and roots must stay untouched.
    local.blobs = InMemoryBlobRepository::new();

    let local_root_before = local.nodes.root_hash().await.unwrap();
    let remote_root_before = remote.nodes.root_hash().await.unwrap();

    let result = super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &local_base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    );

    assert!(matches!(result.await, Err(SyncError::MissingSourceBlob { .. })));
    assert_eq!(local.nodes.root_hash().await.unwrap(), local_root_before);
    assert_eq!(
        remote.nodes.root_hash().await.unwrap(),
        remote_root_before
    );
}
