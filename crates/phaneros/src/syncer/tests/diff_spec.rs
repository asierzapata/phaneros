use crate::syncer::diff::{compute_bidirectional_diff, compute_unidirectional_diff};

use super::fixtures::{RecordingStore, TestStore};

#[test]
fn identical_stores_produce_empty_diff() {
    let mut local = TestStore::new();
    let file = local.add_file("a.txt", b"content");
    let root = local.add_folder("root", vec![], vec![file]);

    // Remote has the exact same nodes.
    let mut remote = TestStore::new();
    let r_file = remote.add_file("a.txt", b"content");
    remote.add_folder("root", vec![], vec![r_file]);

    let (node_diff, blob_diff) =
        compute_unidirectional_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    assert!(node_diff.is_empty());
    assert!(blob_diff.is_empty());
}

#[test]
fn empty_target_needs_every_node_and_blob() {
    let mut local = TestStore::new();
    let file_a = local.add_file("a.txt", b"aaa");
    let file_b = local.add_file("b.txt", b"bbb");
    let sub = local.add_folder("sub", vec![], vec![file_b.clone()]);
    let root = local.add_folder("root", vec![sub.clone()], vec![file_a.clone()]);

    let remote = TestStore::new();

    let (node_diff, blob_diff) =
        compute_unidirectional_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    // 4 distinct nodes: file_a, file_b, sub, root.
    assert_eq!(node_diff.len(), 4);
    for hash in [&file_a.hash, &file_b.hash, &sub.hash, &root.hash] {
        assert!(node_diff.contains(hash), "diff should contain {}", hash);
    }

    // 2 distinct blobs from the two files.
    assert_eq!(blob_diff.len(), 2);
}

#[test]
fn changed_file_sends_only_the_path_to_root() {
    // Remote holds version 1: root -> [docs(one.txt), photos(cat.jpg)]
    let mut remote = TestStore::new();
    let r_one = remote.add_file("one.txt", b"v1");
    let r_cat = remote.add_file("cat.jpg", b"cat-bytes");
    let r_docs = remote.add_folder("docs", vec![], vec![r_one]);
    let r_photos = remote.add_folder("photos", vec![], vec![r_cat]);
    remote.add_folder("root", vec![r_docs, r_photos], vec![]);

    // Local is version 2: only one.txt changed.
    let mut local = TestStore::new();
    let one_v2 = local.add_file("one.txt", b"v2");
    let cat = local.add_file("cat.jpg", b"cat-bytes");
    let docs = local.add_folder("docs", vec![], vec![one_v2.clone()]);
    let photos = local.add_folder("photos", vec![], vec![cat]);
    let root = local.add_folder("root", vec![docs.clone(), photos], vec![]);

    let (node_diff, blob_diff) =
        compute_unidirectional_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    // O(depth): new file node, new docs node, new root. Nothing from
    // the untouched photos subtree.
    assert_eq!(node_diff.len(), 3);
    assert!(node_diff.contains(&one_v2.hash));
    assert!(node_diff.contains(&docs.hash));
    assert!(node_diff.contains(&root.hash));

    // Only the new one.txt payload should transfer.
    assert_eq!(blob_diff.len(), 1);
}

#[test]
fn rename_sends_only_ancestor_folders_never_the_file() {
    // Remote: root -> docs -> original.txt
    let mut remote = TestStore::new();
    let r_file = remote.add_file("original.txt", b"same bytes");
    let r_docs = remote.add_folder("docs", vec![], vec![r_file]);
    remote.add_folder("root", vec![r_docs], vec![]);

    // Local: same content, file renamed.
    let mut local = TestStore::new();
    let renamed = local.add_file("renamed.txt", b"same bytes");
    let docs = local.add_folder("docs", vec![], vec![renamed.clone()]);
    let root = local.add_folder("root", vec![docs.clone()], vec![]);

    let (node_diff, blob_diff) =
        compute_unidirectional_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    // The file's content hash is unchanged, so the remote already has the
    // blob: a rename must transfer zero file bytes.
    assert!(!node_diff.contains(&renamed.hash));
    assert_eq!(node_diff.len(), 2);
    assert!(node_diff.contains(&docs.hash));
    assert!(node_diff.contains(&root.hash));
    assert!(blob_diff.is_empty());
}

#[test]
fn duplicated_content_is_transferred_once() {
    // Two identical files under different folders -> one file node hash.
    let mut local = TestStore::new();
    let copy_a = local.add_file("copy_a.txt", b"identical");
    let copy_b = local.add_file("copy_b.txt", b"identical");
    assert_eq!(copy_a.hash, copy_b.hash);

    let dir_a = local.add_folder("dir_a", vec![], vec![copy_a.clone()]);
    let dir_b = local.add_folder("dir_b", vec![], vec![copy_b]);
    let root = local.add_folder("root", vec![dir_a, dir_b], vec![]);

    let remote = TestStore::new();

    let (node_diff, blob_diff) =
        compute_unidirectional_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    // root + dir_a + dir_b + ONE shared file node = 4, and no duplicates.
    assert_eq!(node_diff.len(), 4);
    let file_occurrences = node_diff.iter().filter(|h| **h == copy_a.hash).count();
    assert_eq!(file_occurrences, 1);
    assert_eq!(blob_diff.len(), 1);
}

#[test]
fn duplicated_folder_is_transferred_once() {
    // Two backups holding an identical `photos` subtree -> one folder node.
    let mut local = TestStore::new();
    let cat = local.add_file("cat.jpg", b"cat-bytes");
    let cat_hash = cat.hash.clone();
    let photos_2023 = local.add_folder("photos", vec![], vec![cat.clone()]);
    let photos_2024 = local.add_folder("photos", vec![], vec![cat]);
    // Same name + same contents -> same folder hash.
    assert_eq!(photos_2023.hash, photos_2024.hash);

    // A folder's hash is derived from its children's names+hashes, not its
    // own name (the name lives in the parent's Entry). Both backups hold a
    // single identical `photos` child, so the two backup folders collapse to
    // ONE node too.
    let backup_2023 = local.add_folder("backup_2023", vec![photos_2023.clone()], vec![]);
    let backup_2024 = local.add_folder("backup_2024", vec![photos_2024], vec![]);
    assert_eq!(backup_2023.hash, backup_2024.hash);
    let root = local.add_folder("root", vec![backup_2023.clone(), backup_2024], vec![]);

    let remote = TestStore::new();

    let recording_local = RecordingStore::new(&local.nodes);
    let (node_diff, _blob_diff) =
        compute_unidirectional_diff(&recording_local, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    // root + ONE shared backup + ONE shared photos + ONE shared cat file = 4.
    assert_eq!(node_diff.len(), 4);

    // The real property: reaching the identical second backup must NOT
    // re-walk its subtree. The visited guard short-circuits recursion, so
    // the deepest shared node is fetched from the source exactly once. A
    // weak `is_none()`-only guard would descend twice and fetch it twice.
    let requested = recording_local.requested.borrow();
    let photos_walks = requested.iter().filter(|h| **h == photos_2023.hash).count();
    let cat_walks = requested.iter().filter(|h| **h == cat_hash).count();
    assert_eq!(
        photos_walks, 1,
        "shared photos subtree was walked more than once"
    );
    assert_eq!(cat_walks, 1, "shared cat file was walked more than once");
}

#[test]
fn shared_subtrees_are_pruned_not_walked() {
    // Remote already has the photos subtree; the walk must never even
    // *look inside* it — that is the whole point of a merkle diff.
    let mut remote = TestStore::new();
    let r_cat = remote.add_file("cat.jpg", b"cat-bytes");
    let r_photos = remote.add_folder("photos", vec![], vec![r_cat]);
    remote.add_folder("root", vec![r_photos], vec![]);

    let mut local = TestStore::new();
    let cat = local.add_file("cat.jpg", b"cat-bytes");
    let photos = local.add_folder("photos", vec![], vec![cat.clone()]);
    let new_file = local.add_file("new.txt", b"new");
    let root = local.add_folder("root", vec![photos], vec![new_file]);

    let recording_local = RecordingStore::new(&local.nodes);
    let (node_diff, _blob_diff) =
        compute_unidirectional_diff(&recording_local, &remote.nodes, &remote.blobs, &root.hash)
            .unwrap();

    // Correct transfer set: new root + new file.
    assert_eq!(node_diff.len(), 2);
    // And the file inside the shared photos subtree was never fetched
    // from the source: the walk pruned at the matching folder hash.
    let requested = recording_local.requested.borrow();
    assert!(
        !requested.contains(&cat.hash),
        "walk descended into a subtree the target already has"
    );
}

#[test]
fn bidirectional_diff_reports_each_direction_independently() {
    // local has local-only file, remote has remote-only file.
    let mut local = TestStore::new();
    let l_file = local.add_file("local.txt", b"local-only");
    let l_root = local.add_folder("root", vec![], vec![l_file.clone()]);

    let mut remote = TestStore::new();
    let r_file = remote.add_file("remote.txt", b"remote-only");
    let r_root = remote.add_folder("root", vec![], vec![r_file.clone()]);

    let (
        (local_to_remote_nodes, local_to_remote_blobs),
        (remote_to_local_nodes, remote_to_local_blobs),
    ) = compute_bidirectional_diff(
        &local.nodes,
        &local.blobs,
        &l_root.hash,
        &remote.nodes,
        &remote.blobs,
        &r_root.hash,
    )
    .unwrap();

    assert!(local_to_remote_nodes.contains(&l_file.hash));
    assert!(local_to_remote_nodes.contains(&l_root.hash));
    assert_eq!(local_to_remote_blobs.len(), 1);

    assert!(remote_to_local_nodes.contains(&r_file.hash));
    assert!(remote_to_local_nodes.contains(&r_root.hash));
    assert_eq!(remote_to_local_blobs.len(), 1);
}
