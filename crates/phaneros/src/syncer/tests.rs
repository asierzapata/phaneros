use std::cell::RefCell;

use crate::blob_repository::{Blob, BlobRef, InMemoryBlobRepository};
use crate::node_repository::{
    Entry, Hash, InMemoryNodeRepository, Node, NodeRepository, NodeRepositoryError,
};
use crate::syncer::{SyncError, SyncPlan, compute_diff, local_push, plan_sync};

// ---- fixture helpers -------------------------------------------------------

/// One side of a sync: node store + blob store, which always travel together.
struct TestStore {
    nodes: InMemoryNodeRepository,
    blobs: InMemoryBlobRepository,
}

impl TestStore {
    fn new() -> Self {
        TestStore {
            nodes: InMemoryNodeRepository::new(),
            blobs: InMemoryBlobRepository::new(),
        }
    }

    /// Inserts a file node built from `content` — and, like the scanner does,
    /// also stores the content bytes in the blob store.
    fn add_file(&mut self, name: &str, content: &[u8]) -> Entry {
        let blob_ref = BlobRef::from_bytes(content);
        self.blobs
            .insert(
                blob_ref.hash.clone(),
                Blob {
                    bytes: content.to_vec(),
                },
            )
            .unwrap();
        let (hash, node) = Node::file(vec![blob_ref]);
        self.nodes.insert(hash.clone(), node).unwrap();
        Entry::new(name, hash)
    }

    /// Inserts a folder node from child entries and returns its entry.
    fn add_folder(&mut self, name: &str, folders: Vec<Entry>, files: Vec<Entry>) -> Entry {
        let (hash, node) = Node::folder(folders, files);
        self.nodes.insert(hash.clone(), node).unwrap();
        Entry::new(name, hash)
    }
}

/// A NodeRepository wrapper that records every hash requested from it, so tests
/// can assert that shared subtrees are pruned (never walked).
struct RecordingStore<'a> {
    inner: &'a InMemoryNodeRepository,
    requested: RefCell<Vec<Hash>>,
}

impl<'a> RecordingStore<'a> {
    fn new(inner: &'a InMemoryNodeRepository) -> Self {
        RecordingStore {
            inner,
            requested: RefCell::new(Vec::new()),
        }
    }
}

impl NodeRepository for RecordingStore<'_> {
    fn root_hash(&self) -> Result<Option<&Hash>, NodeRepositoryError> {
        self.inner.root_hash()
    }

    fn get_node(&self, hash: &Hash) -> Result<Option<Node>, NodeRepositoryError> {
        self.requested.borrow_mut().push(hash.clone());
        self.inner.get_node(hash)
    }
}

// ---- sync planning (B/L/R) -------------------------------------------------

mod sync_plan_spec {
    use super::*;

    fn hash(v: &str) -> Hash {
        v.to_string()
    }

    #[test]
    fn no_base_always_uses_bootstrap_pull_policy() {
        let local = hash("local");

        assert_eq!(
            plan_sync(None, &local, Some(&local)),
            SyncPlan::RemoteBootstrapPull
        );
        assert_eq!(plan_sync(None, &local, None), SyncPlan::RemoteBootstrapPull);
    }

    #[test]
    fn local_and_remote_equal_means_converged_even_with_stale_base() {
        let base = hash("old-base");
        let current = hash("current");

        assert_eq!(
            plan_sync(Some(&base), &current, Some(&current)),
            SyncPlan::Converged
        );
    }

    #[test]
    fn pull_when_only_remote_changed_since_base() {
        let base = hash("base");
        let remote = hash("remote-new");

        assert_eq!(
            plan_sync(Some(&base), &base, Some(&remote)),
            SyncPlan::RemotePull
        );
    }

    #[test]
    fn push_when_only_local_changed_since_base() {
        let base = hash("base");
        let local = hash("local-new");

        assert_eq!(
            plan_sync(Some(&base), &local, Some(&base)),
            SyncPlan::LocalPush
        );
    }

    #[test]
    fn merge_when_both_sides_diverged_from_base() {
        let base = hash("base");
        let local = hash("local-new");
        let remote = hash("remote-new");

        assert_eq!(
            plan_sync(Some(&base), &local, Some(&remote)),
            SyncPlan::Merge
        );
    }

    #[test]
    fn merge_when_remote_is_absent_but_base_exists() {
        let base = hash("base");
        let local = hash("local-new");

        assert_eq!(plan_sync(Some(&base), &local, None), SyncPlan::Merge);
    }
}

// ---- compute_diff: the transfer set ----------------------------------------

mod compute_diff_spec {
    use super::*;

    #[test]
    fn identical_stores_produce_empty_diff() {
        let mut local = TestStore::new();
        let file = local.add_file("a.txt", b"content");
        let root = local.add_folder("root", vec![], vec![file]);

        // Remote has the exact same nodes.
        let mut remote = TestStore::new();
        let r_file = remote.add_file("a.txt", b"content");
        remote.add_folder("root", vec![], vec![r_file]);

        let (diff, _blob_diff) =
            compute_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash).unwrap();

        assert!(diff.is_empty());
    }

    #[test]
    fn empty_target_needs_every_node() {
        let mut local = TestStore::new();
        let file_a = local.add_file("a.txt", b"aaa");
        let file_b = local.add_file("b.txt", b"bbb");
        let sub = local.add_folder("sub", vec![], vec![file_b.clone()]);
        let root = local.add_folder("root", vec![sub.clone()], vec![file_a.clone()]);

        let remote = TestStore::new();

        let (diff, _blob_diff) =
            compute_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash).unwrap();

        // 4 distinct nodes: file_a, file_b, sub, root.
        assert_eq!(diff.len(), 4);
        for hash in [&file_a.hash, &file_b.hash, &sub.hash, &root.hash] {
            assert!(diff.contains(hash), "diff should contain {}", hash);
        }
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

        let (diff, _blob_diff) =
            compute_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash).unwrap();

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
        let mut remote = TestStore::new();
        let r_file = remote.add_file("original.txt", b"same bytes");
        let r_docs = remote.add_folder("docs", vec![], vec![r_file]);
        remote.add_folder("root", vec![r_docs], vec![]);

        // Local: same content, file renamed.
        let mut local = TestStore::new();
        let renamed = local.add_file("renamed.txt", b"same bytes");
        let docs = local.add_folder("docs", vec![], vec![renamed.clone()]);
        let root = local.add_folder("root", vec![docs.clone()], vec![]);

        let (diff, _blob_diff) =
            compute_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash).unwrap();

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
        let mut local = TestStore::new();
        let copy_a = local.add_file("copy_a.txt", b"identical");
        let copy_b = local.add_file("copy_b.txt", b"identical");
        assert_eq!(copy_a.hash, copy_b.hash);

        let dir_a = local.add_folder("dir_a", vec![], vec![copy_a.clone()]);
        let dir_b = local.add_folder("dir_b", vec![], vec![copy_b]);
        let root = local.add_folder("root", vec![dir_a, dir_b], vec![]);

        let remote = TestStore::new();

        let (diff, _blob_diff) =
            compute_diff(&local.nodes, &remote.nodes, &remote.blobs, &root.hash).unwrap();

        // root + dir_a + dir_b + ONE shared blob = 4, and no duplicates.
        assert_eq!(diff.len(), 4);
        let blob_occurrences = diff.iter().filter(|h| **h == copy_a.hash).count();
        assert_eq!(blob_occurrences, 1);
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
        let (diff, _blob_diff) =
            compute_diff(&recording_local, &remote.nodes, &remote.blobs, &root.hash).unwrap();

        // root + ONE shared backup + ONE shared photos + ONE shared cat blob = 4.
        // (The HashSet guarantees this on its own — see the walk assertion below
        // for what the dedup *guard* actually buys us.)
        assert_eq!(diff.len(), 4);

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
        assert_eq!(cat_walks, 1, "shared cat blob was walked more than once");
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
        let (diff, _blob_diff) =
            compute_diff(&recording_local, &remote.nodes, &remote.blobs, &root.hash).unwrap();

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

// ---- reconcile_node_repositorys: transfer + root flip ----------------------------

mod reconcile_spec {
    use super::*;

    #[test]
    fn reconcile_copies_missing_nodes_and_sets_root() {
        let mut local = TestStore::new();
        let file = local.add_file("a.txt", b"payload");
        let root = local.add_folder("root", vec![], vec![file.clone()]);

        let mut remote = TestStore::new();

        let transferred = local_push(
            &local.nodes,
            &mut remote.nodes,
            &local.blobs,
            &mut remote.blobs,
            &root.hash,
        )
        .unwrap();

        assert_eq!(transferred, 2);
        assert_eq!(remote.nodes.root_hash().unwrap(), Some(&root.hash));
        assert!(remote.nodes.get_node(&root.hash).unwrap().is_some());
        assert!(remote.nodes.get_node(&file.hash).unwrap().is_some());
        // The transferred nodes are byte-identical to the source's.
        assert_eq!(
            remote.nodes.get_node(&file.hash).unwrap(),
            local.nodes.get_node(&file.hash).unwrap()
        );
    }

    #[test]
    fn reconcile_identical_stores_transfers_nothing_but_updates_root() {
        let mut local = TestStore::new();
        let file = local.add_file("a.txt", b"same");
        let root = local.add_folder("root", vec![], vec![file]);

        // Remote has all the new version's nodes but still points at an
        // older root (whose nodes it also retains).
        let mut remote = TestStore::new();
        let old_file = remote.add_file("a.txt", b"older");
        let old = remote.add_folder("root", vec![], vec![old_file]);
        let r_file = remote.add_file("a.txt", b"same");
        let r_root = remote.add_folder("root", vec![], vec![r_file]);
        assert_eq!(r_root.hash, root.hash);
        assert_ne!(old.hash, root.hash);
        remote.nodes.set_root(old.hash).unwrap();

        let transferred = local_push(
            &local.nodes,
            &mut remote.nodes,
            &local.blobs,
            &mut remote.blobs,
            &root.hash,
        )
        .unwrap();

        assert_eq!(transferred, 0);
        assert_eq!(remote.nodes.root_hash().unwrap(), Some(&root.hash));
    }

    #[test]
    fn reconcile_preserves_the_previous_version_nodes() {
        // Version 1 lives on the remote.
        let mut local_v1 = TestStore::new();
        let file_v1 = local_v1.add_file("doc.txt", b"v1");
        let root_v1 = local_v1.add_folder("root", vec![], vec![file_v1.clone()]);

        let mut remote = TestStore::new();
        local_push(
            &local_v1.nodes,
            &mut remote.nodes,
            &local_v1.blobs,
            &mut remote.blobs,
            &root_v1.hash,
        )
        .unwrap();

        // Version 2 replaces the file.
        let mut local_v2 = TestStore::new();
        let file_v2 = local_v2.add_file("doc.txt", b"v2");
        let root_v2 = local_v2.add_folder("root", vec![], vec![file_v2]);

        local_push(
            &local_v2.nodes,
            &mut remote.nodes,
            &local_v2.blobs,
            &mut remote.blobs,
            &root_v2.hash,
        )
        .unwrap();

        assert_eq!(remote.nodes.root_hash().unwrap(), Some(&root_v2.hash));
        // Old version's nodes are still reachable by hash: this is what makes
        // server-side version history possible (GC will prune them later).
        assert!(remote.nodes.get_node(&root_v1.hash).unwrap().is_some());
        assert!(remote.nodes.get_node(&file_v1.hash).unwrap().is_some());
    }
}

// ---- reconcile: error safety -------------------------------------------------

mod reconcile_error_safety {
    use super::*;

    #[test]
    fn missing_source_blob_aborts_before_root_flip() {
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
        // ...and — the actual invariant — the remote's visible tree is
        // untouched: root still points at the old version, and none of the
        // new nodes became reachable. Blobs land before nodes and the root
        // flips last, so failing between phases can never publish a tree
        // with dangling references.
        assert_eq!(remote.nodes.root_hash().unwrap(), Some(&old_root.hash));
        assert!(remote.nodes.get_node(&root.hash).unwrap().is_none());
        assert!(remote.nodes.get_node(&file.hash).unwrap().is_none());
    }
}
