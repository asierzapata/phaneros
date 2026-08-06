use crate::node_repository::WritableNodeRepository;
use crate::blob_repository::BlobRef;
use crate::node_repository::NodeRepository;
use crate::syncer::{TransferOptions, local_push, remote_pull};

use super::fixtures::{
    TestStore, assert_has_blob, assert_has_node, assert_missing_blob, assert_missing_node,
};

#[tokio::test]
async fn local_push_copies_missing_nodes_and_blobs_and_sets_root() {
    let mut local = TestStore::new();
    let file = local.add_file("a.txt", b"payload").await;
    let root = local.add_folder("root", vec![], vec![file.clone()]).await;

    let expected_blob = BlobRef::from_bytes(b"payload");

    let mut remote = TestStore::new();

    let transferred = local_push(
        &local.nodes,
        &mut remote.nodes,
        &local.blobs,
        &mut remote.blobs,
        &root.hash,
        TransferOptions::default(),
    )
        .await.unwrap();

    assert_eq!(transferred, 2);
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(root.hash.clone()));
    assert_has_node(&remote.nodes, &root.hash).await;
    assert_has_node(&remote.nodes, &file.hash).await;
    assert_has_blob(&remote.blobs, &expected_blob.hash).await;
    // The transferred nodes are byte-identical to the source's.
    assert_eq!(
        remote.nodes.get_node(&file.hash).await.unwrap(),
        local.nodes.get_node(&file.hash).await.unwrap()
    );
}

#[tokio::test]
async fn local_push_identical_stores_transfers_nothing_but_updates_root() {
    let mut local = TestStore::new();
    let file = local.add_file("a.txt", b"same").await;
    let root = local.add_folder("root", vec![], vec![file]).await;

    // Remote has all the new version's nodes but still points at an
    // older root (whose nodes it also retains).
    let mut remote = TestStore::new();
    let old_file = remote.add_file("a.txt", b"older").await;
    let old = remote.add_folder("root", vec![], vec![old_file]).await;
    let r_file = remote.add_file("a.txt", b"same").await;
    let r_root = remote.add_folder("root", vec![], vec![r_file]).await;
    assert_eq!(r_root.hash, root.hash);
    assert_ne!(old.hash, root.hash);
    remote.nodes.set_root(old.hash).await.unwrap();

    let transferred = local_push(
        &local.nodes,
        &mut remote.nodes,
        &local.blobs,
        &mut remote.blobs,
        &root.hash,
        TransferOptions::default(),
    )
        .await.unwrap();

    assert_eq!(transferred, 0);
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(root.hash.clone()));
}

#[tokio::test]
async fn local_push_preserves_previous_version_nodes() {
    // Version 1 lives on the remote.
    let mut local_v1 = TestStore::new();
    let file_v1 = local_v1.add_file("doc.txt", b"v1").await;
    let root_v1 = local_v1.add_folder("root", vec![], vec![file_v1.clone()]).await;

    let mut remote = TestStore::new();
    local_push(
        &local_v1.nodes,
        &mut remote.nodes,
        &local_v1.blobs,
        &mut remote.blobs,
        &root_v1.hash,
        TransferOptions::default(),
    )
        .await.unwrap();

    // Version 2 replaces the file.
    let mut local_v2 = TestStore::new();
    let file_v2 = local_v2.add_file("doc.txt", b"v2").await;
    let root_v2 = local_v2.add_folder("root", vec![], vec![file_v2]).await;

    local_push(
        &local_v2.nodes,
        &mut remote.nodes,
        &local_v2.blobs,
        &mut remote.blobs,
        &root_v2.hash,
        TransferOptions::default(),
    )
        .await.unwrap();

    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(root_v2.hash.clone()));
    // Old version's nodes are still reachable by hash: this is what makes
    // server-side version history possible (GC will prune them later).
    assert_has_node(&remote.nodes, &root_v1.hash).await;
    assert_has_node(&remote.nodes, &file_v1.hash).await;
}

#[tokio::test]
async fn remote_pull_copies_missing_nodes_and_blobs_and_sets_root() {
    let mut remote = TestStore::new();
    let file = remote.add_file("from-remote.txt", b"payload").await;
    let root = remote.add_folder("root", vec![], vec![file.clone()]).await;

    let expected_blob = BlobRef::from_bytes(b"payload");

    let mut local = TestStore::new();

    let transferred = remote_pull(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &root.hash,
        TransferOptions::default(),
    )
        .await.unwrap();

    assert_eq!(transferred, 2);
    assert_eq!(local.nodes.root_hash().await.unwrap(), Some(root.hash.clone()));
    assert_has_node(&local.nodes, &root.hash).await;
    assert_has_node(&local.nodes, &file.hash).await;
    assert_has_blob(&local.blobs, &expected_blob.hash).await;
}

#[tokio::test]
async fn remote_pull_identical_stores_transfers_nothing_but_updates_root() {
    let mut remote = TestStore::new();
    let file = remote.add_file("doc.txt", b"same").await;
    let root = remote.add_folder("root", vec![], vec![file.clone()]).await;

    let mut local = TestStore::new();
    // local already has the content nodes but points at an old root.
    let old_file = local.add_file("doc.txt", b"old").await;
    let old_root = local.add_folder("root", vec![], vec![old_file]).await;
    let l_file = local.add_file("doc.txt", b"same").await;
    let l_root = local.add_folder("root", vec![], vec![l_file]).await;
    assert_eq!(l_root.hash, root.hash);
    local.nodes.set_root(old_root.hash).await.unwrap();

    let transferred = remote_pull(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &root.hash,
        TransferOptions::default(),
    )
        .await.unwrap();

    assert_eq!(transferred, 0);
    assert_eq!(local.nodes.root_hash().await.unwrap(), Some(root.hash.clone()));
}

#[tokio::test]
async fn bootstrap_pull_copies_remote_state_into_local() {
    let mut remote = TestStore::new();
    let file = remote.add_file("from-remote.txt", b"payload").await;
    let root = remote.add_folder("root", vec![], vec![file.clone()]).await;

    let expected_blob = BlobRef::from_bytes(b"payload");

    let mut local = TestStore::new();
    // local has stale unrelated data before bootstrap.
    let stale_file = local.add_file("stale.txt", b"stale").await;
    let stale_root = local.add_folder("root", vec![], vec![stale_file.clone()]).await;
    local.nodes.set_root(stale_root.hash.clone()).await.unwrap();
    assert_missing_node(&local.nodes, &root.hash).await;
    assert_missing_blob(&local.blobs, &expected_blob.hash).await;

    let transferred = super::super::bootstrap_pull(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &root.hash,
        TransferOptions::default(),
    )
    .await
    .unwrap();

    assert_eq!(transferred, 2);
    assert_eq!(local.nodes.root_hash().await.unwrap(), Some(root.hash.clone()));
    assert_has_node(&local.nodes, &root.hash).await;
    assert_has_node(&local.nodes, &file.hash).await;
    assert_has_blob(&local.blobs, &expected_blob.hash).await;
    // old nodes are still in the local object store by hash.
    assert_has_node(&local.nodes, &stale_root.hash).await;
    assert_has_node(&local.nodes, &stale_file.hash).await;
}
