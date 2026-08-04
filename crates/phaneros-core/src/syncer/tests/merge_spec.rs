use crate::node_repository::WritableNodeRepository;
use std::collections::HashMap;

use crate::{
    blob_repository::BlobRef,
    node_repository::{Node, NodeRepository},
};

use super::fixtures::{TestStore, assert_has_blob};

async fn folder_maps(
    store: &TestStore,
    folder_hash: &String,
) -> (HashMap<String, String>, HashMap<String, String>) {
    let Some(Node::Folder { folders, files }) = store.nodes.get_node(folder_hash).await.unwrap() else {
        panic!("expected folder node {}", folder_hash);
    };

    let folder_map = folders
        .into_iter()
        .map(|entry| (entry.name, entry.hash))
        .collect();
    let file_map = files
        .into_iter()
        .map(|entry| (entry.name, entry.hash))
        .collect();

    (folder_map, file_map)
}

#[tokio::test]
async fn merge_combines_independent_local_and_remote_changes() {
    let mut local = TestStore::new();
    let local_base_file = local.add_file("shared.txt", b"v1").await;
    let local_base_root = local.add_folder("root", vec![], vec![local_base_file]).await;
    let local_v2_file = local.add_file("shared.txt", b"v2").await;
    let local_root = local.add_folder("root", vec![], vec![local_v2_file.clone()]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let remote_base_file = remote.add_file("shared.txt", b"v1").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![remote_base_file]).await;
    let remote_only_file = remote.add_file("remote.txt", b"remote-only").await;
    let remote_shared_v1 = remote.add_file("shared.txt", b"v1").await;
    let remote_root = remote.add_folder(
        "root",
        vec![],
        vec![remote_shared_v1, remote_only_file.clone()],
    ).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(local_base_root.hash, remote_base_root.hash);

    let reconciled = super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &local_base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    assert!(reconciled > 0);

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(merged_root.clone()));
    assert_ne!(merged_root, local_root.hash);
    assert_ne!(merged_root, remote_root.hash);

    let (folders, files) = folder_maps(&local, &merged_root).await;
    assert!(folders.is_empty());
    assert_eq!(files.len(), 2);
    assert_eq!(files.get("remote.txt"), Some(&remote_only_file.hash));
    assert_eq!(files.get("shared.txt"), Some(&local_v2_file.hash));
}

#[tokio::test]
async fn merge_file_modify_modify_keeps_both_with_conflict_suffix() {
    let local_blob = BlobRef::from_bytes(b"local-v2");
    let remote_blob = BlobRef::from_bytes(b"remote-v2");

    let mut local = TestStore::new();
    let local_base_file = local.add_file("doc.txt", b"v1").await;
    let local_base_root = local.add_folder("root", vec![], vec![local_base_file]).await;
    let local_v2_file = local.add_file("doc.txt", b"local-v2").await;
    let local_root = local.add_folder("root", vec![], vec![local_v2_file.clone()]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let remote_base_file = remote.add_file("doc.txt", b"v1").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![remote_base_file]).await;
    let remote_v2_file = remote.add_file("doc.txt", b"remote-v2").await;
    let remote_root = remote.add_folder("root", vec![], vec![remote_v2_file.clone()]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(local_base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &local_base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(merged_root.clone()));

    let (folders, files) = folder_maps(&local, &merged_root).await;
    assert!(folders.is_empty());
    assert_eq!(files.len(), 2);
    assert_eq!(files.get("doc.txt"), Some(&local_v2_file.hash));
    assert_eq!(files.get("doc.txt.conflict"), Some(&remote_v2_file.hash));

    assert_has_blob(&local.blobs, &local_blob.hash).await;
    assert_has_blob(&local.blobs, &remote_blob.hash).await;
    assert_has_blob(&remote.blobs, &local_blob.hash).await;
    assert_has_blob(&remote.blobs, &remote_blob.hash).await;
}

#[tokio::test]
async fn merge_delete_modify_preserves_edit_as_conflict_delete() {
    let remote_blob = BlobRef::from_bytes(b"remote-edit");

    let mut local = TestStore::new();
    let local_base_file = local.add_file("note.txt", b"v1").await;
    let local_base_root = local.add_folder("root", vec![], vec![local_base_file]).await;
    let local_root = local.add_folder("root", vec![], vec![]).await; // deleted note.txt
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let remote_base_file = remote.add_file("note.txt", b"v1").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![remote_base_file]).await;
    let remote_edited_file = remote.add_file("note.txt", b"remote-edit").await;
    let remote_root = remote.add_folder("root", vec![], vec![remote_edited_file.clone()]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(local_base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &local_base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(merged_root.clone()));

    let (folders, files) = folder_maps(&local, &merged_root).await;
    assert!(folders.is_empty());
    assert_eq!(files.len(), 1);
    assert_eq!(
        files.get("note.txt.conflict-delete"),
        Some(&remote_edited_file.hash)
    );

    assert_has_blob(&local.blobs, &remote_blob.hash).await;
    assert_has_blob(&remote.blobs, &remote_blob.hash).await;
}

#[tokio::test]
async fn merge_folder_modify_modify_recurses_and_keeps_both_subtree_changes() {
    let mut local = TestStore::new();
    let base_note = local.add_file("note.txt", b"v1").await;
    let base_docs = local.add_folder("docs", vec![], vec![base_note]).await;
    let base_root = local.add_folder("root", vec![base_docs], vec![]).await;

    let local_note = local.add_file("note.txt", b"local-v2").await;
    let local_only = local.add_file("local-only.txt", b"local-only").await;
    let local_docs = local.add_folder("docs", vec![], vec![local_note.clone(), local_only.clone()]).await;
    let local_root = local.add_folder("root", vec![local_docs], vec![]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let r_base_note = remote.add_file("note.txt", b"v1").await;
    let r_base_docs = remote.add_folder("docs", vec![], vec![r_base_note]).await;
    let remote_base_root = remote.add_folder("root", vec![r_base_docs], vec![]).await;

    let remote_note = remote.add_file("note.txt", b"v1").await;
    let remote_only = remote.add_file("remote-only.txt", b"remote-only").await;
    let remote_docs = remote.add_folder("docs", vec![], vec![remote_note, remote_only.clone()]).await;
    let remote_root = remote.add_folder("root", vec![remote_docs], vec![]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(merged_root.clone()));

    let (root_folders, root_files) = folder_maps(&local, &merged_root).await;
    assert!(root_files.is_empty());
    let docs_hash = root_folders
        .get("docs")
        .expect("merged root should keep docs folder")
        .clone();

    let (docs_folders, docs_files) = folder_maps(&local, &docs_hash).await;
    assert!(docs_folders.is_empty());
    assert_eq!(docs_files.len(), 3);
    assert_eq!(docs_files.get("note.txt"), Some(&local_note.hash));
    assert_eq!(docs_files.get("local-only.txt"), Some(&local_only.hash));
    assert_eq!(docs_files.get("remote-only.txt"), Some(&remote_only.hash));
}

#[tokio::test]
async fn merge_fast_forwards_remote_delete_when_local_unchanged() {
    let mut local = TestStore::new();
    let keep = local.add_file("keep.txt", b"keep").await;
    let drop_file = local.add_file("drop.txt", b"drop").await;
    let base_root = local.add_folder("root", vec![], vec![keep.clone(), drop_file]).await;
    local.nodes.set_root(base_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let r_keep = remote.add_file("keep.txt", b"keep").await;
    let r_drop_file = remote.add_file("drop.txt", b"drop").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![r_keep.clone(), r_drop_file]).await;
    let remote_root = remote.add_folder("root", vec![], vec![r_keep.clone()]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root.hash,
        &base_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    assert_eq!(local.nodes.root_hash().await.unwrap(), Some(remote_root.hash.clone()));
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(remote_root.hash.clone()));

    let (_, files) = folder_maps(&local, &remote_root.hash).await;
    assert_eq!(files.len(), 1);
    assert_eq!(files.get("keep.txt"), Some(&keep.hash));
    assert!(!files.contains_key("drop.txt"));
}

#[tokio::test]
async fn merge_fast_forwards_local_delete_when_remote_unchanged() {
    let mut local = TestStore::new();
    let keep = local.add_file("keep.txt", b"keep").await;
    let drop_file = local.add_file("drop.txt", b"drop").await;
    let base_root = local.add_folder("root", vec![], vec![keep.clone(), drop_file]).await;
    let local_root = local.add_folder("root", vec![], vec![keep.clone()]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let r_keep = remote.add_file("keep.txt", b"keep").await;
    let r_drop_file = remote.add_file("drop.txt", b"drop").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![r_keep.clone(), r_drop_file]).await;
    remote
        .nodes
        .set_root(remote_base_root.hash.clone())
        .await
        .unwrap();

    assert_eq!(base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root.hash,
        &local_root.hash,
        &remote_base_root.hash,
    )
        .await.unwrap();

    assert_eq!(local.nodes.root_hash().await.unwrap(), Some(local_root.hash.clone()));
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(local_root.hash.clone()));

    let (_, files) = folder_maps(&remote, &local_root.hash).await;
    assert_eq!(files.len(), 1);
    assert_eq!(files.get("keep.txt"), Some(&keep.hash));
    assert!(!files.contains_key("drop.txt"));
}

#[tokio::test]
async fn merge_conflict_name_collision_uses_incremented_suffix() {
    let mut local = TestStore::new();
    let base_doc = local.add_file("doc.txt", b"v1").await;
    let occupied = local.add_file("doc.txt.conflict", b"already-here").await;
    let base_root = local.add_folder("root", vec![], vec![base_doc, occupied.clone()]).await;

    let local_doc = local.add_file("doc.txt", b"local-v2").await;
    let local_root = local.add_folder("root", vec![], vec![local_doc.clone(), occupied.clone()]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let r_base_doc = remote.add_file("doc.txt", b"v1").await;
    let r_occupied = remote.add_file("doc.txt.conflict", b"already-here").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![r_base_doc, r_occupied.clone()]).await;

    let remote_doc = remote.add_file("doc.txt", b"remote-v2").await;
    let remote_root =
        remote.add_folder("root", vec![], vec![remote_doc.clone(), r_occupied.clone()]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    let (_, files) = folder_maps(&local, &merged_root).await;

    assert_eq!(files.len(), 3);
    assert_eq!(files.get("doc.txt"), Some(&local_doc.hash));
    assert_eq!(files.get("doc.txt.conflict"), Some(&occupied.hash));
    assert_eq!(files.get("doc.txt.conflict.1"), Some(&remote_doc.hash));
}

#[tokio::test]
async fn merge_conflict_delete_name_collision_uses_incremented_suffix() {
    let mut local = TestStore::new();
    let base_note = local.add_file("note.txt", b"v1").await;
    let occupied = local.add_file("note.txt.conflict-delete", b"already-here").await;
    let base_root = local.add_folder("root", vec![], vec![base_note, occupied.clone()]).await;

    let local_root = local.add_folder("root", vec![], vec![occupied.clone()]).await; // note deleted
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let r_base_note = remote.add_file("note.txt", b"v1").await;
    let r_occupied = remote.add_file("note.txt.conflict-delete", b"already-here").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![r_base_note, r_occupied.clone()]).await;

    let remote_edit = remote.add_file("note.txt", b"remote-v2").await;
    let remote_root = remote.add_folder(
        "root",
        vec![],
        vec![remote_edit.clone(), r_occupied.clone()],
    ).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(base_root.hash, remote_base_root.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    let (_, files) = folder_maps(&local, &merged_root).await;

    assert_eq!(files.len(), 2);
    assert_eq!(files.get("note.txt.conflict-delete"), Some(&occupied.hash));
    assert_eq!(
        files.get("note.txt.conflict-delete.1"),
        Some(&remote_edit.hash)
    );
}

#[tokio::test]
async fn merge_identical_modification_keeps_single_entry_without_conflict() {
    let mut local = TestStore::new();
    let base_doc = local.add_file("doc.txt", b"v1").await;
    let base_root = local.add_folder("root", vec![], vec![base_doc]).await;

    let local_doc = local.add_file("doc.txt", b"v2").await;
    let local_root = local.add_folder("root", vec![], vec![local_doc.clone()]).await;
    local.nodes.set_root(local_root.hash.clone()).await.unwrap();

    let mut remote = TestStore::new();
    let remote_base_doc = remote.add_file("doc.txt", b"v1").await;
    let remote_base_root = remote.add_folder("root", vec![], vec![remote_base_doc]).await;

    let remote_doc = remote.add_file("doc.txt", b"v2").await;
    let remote_root = remote.add_folder("root", vec![], vec![remote_doc.clone()]).await;
    remote.nodes.set_root(remote_root.hash.clone()).await.unwrap();

    assert_eq!(base_root.hash, remote_base_root.hash);
    assert_eq!(local_doc.hash, remote_doc.hash);

    super::super::merge(
        &mut local.nodes,
        &mut remote.nodes,
        &mut local.blobs,
        &mut remote.blobs,
        &base_root.hash,
        &local_root.hash,
        &remote_root.hash,
    )
        .await.unwrap();

    let merged_root = local.nodes.root_hash().await.unwrap().unwrap();
    assert_eq!(merged_root, local_root.hash);
    assert_eq!(merged_root, remote_root.hash);
    assert_eq!(remote.nodes.root_hash().await.unwrap(), Some(merged_root.clone()));

    let (_, files) = folder_maps(&local, &merged_root).await;
    assert_eq!(files.len(), 1);
    assert_eq!(files.get("doc.txt"), Some(&local_doc.hash));
    assert!(!files.contains_key("doc.txt.conflict"));
}
