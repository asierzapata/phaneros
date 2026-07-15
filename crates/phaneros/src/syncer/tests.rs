use std::cell::RefCell;

use crate::node_store::{
    Entry, FileChunk, Hash, InMemoryNodeStore, Node, NodeStore, WritableNodeStore,
};
use crate::syncer::{compute_diff, reconcile_node_stores};

// ---- fixture helpers -------------------------------------------------------

/// Inserts a file node built from `content` and returns its entry.
fn add_file(store: &mut InMemoryNodeStore, name: &str, content: &[u8]) -> Entry {
    let (hash, node) = Node::file(vec![FileChunk::from_bytes(content)]);
    store.insert(hash.clone(), node);
    Entry::new(name, hash)
}

/// Inserts a folder node from child entries and returns its entry.
fn add_folder(
    store: &mut InMemoryNodeStore,
    name: &str,
    folders: Vec<Entry>,
    files: Vec<Entry>,
) -> Entry {
    let (hash, node) = Node::folder(folders, files);
    store.insert(hash.clone(), node);
    Entry::new(name, hash)
}

/// A NodeStore wrapper that records every hash requested from it, so tests
/// can assert that shared subtrees are pruned (never walked).
struct RecordingStore<'a> {
    inner: &'a InMemoryNodeStore,
    requested: RefCell<Vec<Hash>>,
}

impl<'a> RecordingStore<'a> {
    fn new(inner: &'a InMemoryNodeStore) -> Self {
        RecordingStore {
            inner,
            requested: RefCell::new(Vec::new()),
        }
    }
}

impl NodeStore for RecordingStore<'_> {
    fn root_hash(&self) -> Option<&Hash> {
        self.inner.root_hash()
    }

    fn get_node(&self, hash: &Hash) -> Option<&Node> {
        self.requested.borrow_mut().push(hash.clone());
        self.inner.get_node(hash)
    }
}

// ---- compute_diff: the transfer set ----------------------------------------

mod compute_diff_spec {
    use super::*;

    #[test]
    fn identical_stores_produce_empty_diff() {
        let mut local = InMemoryNodeStore::new();
        let file = add_file(&mut local, "a.txt", b"content");
        let root = add_folder(&mut local, "root", vec![], vec![file]);

        // Remote has the exact same nodes.
        let mut remote = InMemoryNodeStore::new();
        let r_file = add_file(&mut remote, "a.txt", b"content");
        add_folder(&mut remote, "root", vec![], vec![r_file]);

        let diff = compute_diff(&local, &remote, &root.hash);

        assert!(diff.is_empty());
    }

    #[test]
    fn empty_target_needs_every_node() {
        let mut local = InMemoryNodeStore::new();
        let file_a = add_file(&mut local, "a.txt", b"aaa");
        let file_b = add_file(&mut local, "b.txt", b"bbb");
        let sub = add_folder(&mut local, "sub", vec![], vec![file_b.clone()]);
        let root = add_folder(&mut local, "root", vec![sub.clone()], vec![file_a.clone()]);

        let remote = InMemoryNodeStore::new();

        let diff = compute_diff(&local, &remote, &root.hash);

        // 4 distinct nodes: file_a, file_b, sub, root.
        assert_eq!(diff.len(), 4);
        for hash in [&file_a.hash, &file_b.hash, &sub.hash, &root.hash] {
            assert!(diff.contains(hash), "diff should contain {}", hash);
        }
    }

    #[test]
    fn changed_file_sends_only_the_path_to_root() {
        // Remote holds version 1: root -> [docs(one.txt), photos(cat.jpg)]
        let mut remote = InMemoryNodeStore::new();
        let r_one = add_file(&mut remote, "one.txt", b"v1");
        let r_cat = add_file(&mut remote, "cat.jpg", b"cat-bytes");
        let r_docs = add_folder(&mut remote, "docs", vec![], vec![r_one]);
        let r_photos = add_folder(&mut remote, "photos", vec![], vec![r_cat]);
        add_folder(&mut remote, "root", vec![r_docs, r_photos], vec![]);

        // Local is version 2: only one.txt changed.
        let mut local = InMemoryNodeStore::new();
        let one_v2 = add_file(&mut local, "one.txt", b"v2");
        let cat = add_file(&mut local, "cat.jpg", b"cat-bytes");
        let docs = add_folder(&mut local, "docs", vec![], vec![one_v2.clone()]);
        let photos = add_folder(&mut local, "photos", vec![], vec![cat]);
        let root = add_folder(&mut local, "root", vec![docs.clone(), photos], vec![]);

        let diff = compute_diff(&local, &remote, &root.hash);

        // O(depth): new file node, new docs node, new root. Nothing from
        // the untouched photos subtree.
        assert_eq!(diff.len(), 3);
        assert!(diff.contains(&one_v2.hash));
        assert!(diff.contains(&docs.hash));
        assert!(diff.contains(&root.hash));
    }
}

mod compute_diff_merkle_properties {
    use super::*;

    #[test]
    fn rename_sends_only_ancestor_folders_never_the_file() {
        // Remote: root -> docs -> original.txt
        let mut remote = InMemoryNodeStore::new();
        let r_file = add_file(&mut remote, "original.txt", b"same bytes");
        let r_docs = add_folder(&mut remote, "docs", vec![], vec![r_file]);
        add_folder(&mut remote, "root", vec![r_docs], vec![]);

        // Local: same content, file renamed.
        let mut local = InMemoryNodeStore::new();
        let renamed = add_file(&mut local, "renamed.txt", b"same bytes");
        let docs = add_folder(&mut local, "docs", vec![], vec![renamed.clone()]);
        let root = add_folder(&mut local, "root", vec![docs.clone()], vec![]);

        let diff = compute_diff(&local, &remote, &root.hash);

        // The file's content hash is unchanged, so the remote already has the
        // blob: a rename must transfer zero file bytes.
        assert!(!diff.contains(&renamed.hash));
        assert_eq!(diff.len(), 2);
        assert!(diff.contains(&docs.hash));
        assert!(diff.contains(&root.hash));
    }

    #[test]
    fn duplicated_content_is_transferred_once() {
        // Two identical files under different folders -> one blob node.
        let mut local = InMemoryNodeStore::new();
        let copy_a = add_file(&mut local, "copy_a.txt", b"identical");
        let copy_b = add_file(&mut local, "copy_b.txt", b"identical");
        assert_eq!(copy_a.hash, copy_b.hash);

        let dir_a = add_folder(&mut local, "dir_a", vec![], vec![copy_a.clone()]);
        let dir_b = add_folder(&mut local, "dir_b", vec![], vec![copy_b]);
        let root = add_folder(&mut local, "root", vec![dir_a, dir_b], vec![]);

        let remote = InMemoryNodeStore::new();

        let diff = compute_diff(&local, &remote, &root.hash);

        // root + dir_a + dir_b + ONE shared blob = 4, and no duplicates.
        assert_eq!(diff.len(), 4);
        let blob_occurrences = diff.iter().filter(|h| **h == copy_a.hash).count();
        assert_eq!(blob_occurrences, 1);
    }

    #[test]
    fn shared_subtrees_are_pruned_not_walked() {
        // Remote already has the photos subtree; the walk must never even
        // *look inside* it — that is the whole point of a merkle diff.
        let mut remote = InMemoryNodeStore::new();
        let r_cat = add_file(&mut remote, "cat.jpg", b"cat-bytes");
        let r_photos = add_folder(&mut remote, "photos", vec![], vec![r_cat]);
        add_folder(&mut remote, "root", vec![r_photos], vec![]);

        let mut local = InMemoryNodeStore::new();
        let cat = add_file(&mut local, "cat.jpg", b"cat-bytes");
        let photos = add_folder(&mut local, "photos", vec![], vec![cat.clone()]);
        let new_file = add_file(&mut local, "new.txt", b"new");
        let root = add_folder(&mut local, "root", vec![photos], vec![new_file]);

        let recording_local = RecordingStore::new(&local);
        let diff = compute_diff(&recording_local, &remote, &root.hash);

        // Correct transfer set: new root + new file.
        assert_eq!(diff.len(), 2);
        // And the file inside the shared photos subtree was never fetched
        // from the source: the walk pruned at the matching folder hash.
        let requested = recording_local.requested.borrow();
        assert!(
            !requested.contains(&cat.hash),
            "walk descended into a subtree the target already has"
        );
    }
}

// ---- reconcile_node_stores: transfer + root flip ----------------------------

mod reconcile_spec {
    use super::*;

    #[test]
    fn reconcile_copies_missing_nodes_and_sets_root() {
        let mut local = InMemoryNodeStore::new();
        let file = add_file(&mut local, "a.txt", b"payload");
        let root = add_folder(&mut local, "root", vec![], vec![file.clone()]);

        let mut remote = InMemoryNodeStore::new();

        let transferred = reconcile_node_stores(&local, &mut remote, &root.hash);

        assert_eq!(transferred, 2);
        assert_eq!(remote.root_hash(), Some(&root.hash));
        assert!(remote.get_node(&root.hash).is_some());
        assert!(remote.get_node(&file.hash).is_some());
        // The transferred nodes are byte-identical to the source's.
        assert_eq!(remote.get_node(&file.hash), local.get_node(&file.hash));
    }

    #[test]
    fn reconcile_identical_stores_transfers_nothing_but_updates_root() {
        let mut local = InMemoryNodeStore::new();
        let file = add_file(&mut local, "a.txt", b"same");
        let root = add_folder(&mut local, "root", vec![], vec![file]);

        // Remote has all the new version's nodes but still points at an
        // older root (whose nodes it also retains).
        let mut remote = InMemoryNodeStore::new();
        let old_file = add_file(&mut remote, "a.txt", b"older");
        let old = add_folder(&mut remote, "root", vec![], vec![old_file]);
        let r_file = add_file(&mut remote, "a.txt", b"same");
        let r_root = add_folder(&mut remote, "root", vec![], vec![r_file]);
        assert_eq!(r_root.hash, root.hash);
        assert_ne!(old.hash, root.hash);
        remote.set_root(old.hash);

        let transferred = reconcile_node_stores(&local, &mut remote, &root.hash);

        assert_eq!(transferred, 0);
        assert_eq!(remote.root_hash(), Some(&root.hash));
    }

    #[test]
    fn reconcile_preserves_the_previous_version_nodes() {
        // Version 1 lives on the remote.
        let mut local_v1 = InMemoryNodeStore::new();
        let file_v1 = add_file(&mut local_v1, "doc.txt", b"v1");
        let root_v1 = add_folder(&mut local_v1, "root", vec![], vec![file_v1.clone()]);

        let mut remote = InMemoryNodeStore::new();
        reconcile_node_stores(&local_v1, &mut remote, &root_v1.hash);

        // Version 2 replaces the file.
        let mut local_v2 = InMemoryNodeStore::new();
        let file_v2 = add_file(&mut local_v2, "doc.txt", b"v2");
        let root_v2 = add_folder(&mut local_v2, "root", vec![], vec![file_v2]);

        reconcile_node_stores(&local_v2, &mut remote, &root_v2.hash);

        assert_eq!(remote.root_hash(), Some(&root_v2.hash));
        // Old version's nodes are still reachable by hash: this is what makes
        // server-side version history possible (GC will prune them later).
        assert!(remote.get_node(&root_v1.hash).is_some());
        assert!(remote.get_node(&file_v1.hash).is_some());
    }
}
