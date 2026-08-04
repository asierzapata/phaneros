use crate::node_repository::WritableNodeRepository;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::blob_repository::BlobRef;
use crate::node_repository::{Entry, Hash, Node};
use crate::syncer::materialize::{MaterializeError, MaterializeStats, materialize};

use super::fixtures::{RecordingStore, TestStore};

struct Vault {
    _tmp: TempDir,
    path: PathBuf,
    trash_batch: PathBuf,
}

impl Vault {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vault");
        std::fs::create_dir_all(&path).unwrap();
        let trash_batch = path.join(".phaneros/trash/1");

        Vault {
            _tmp: tmp,
            path,
            trash_batch,
        }
    }

    fn write(&self, rel_path: &str, content: &[u8]) {
        let path = self.path.join(rel_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn read(&self, rel_path: &str) -> Vec<u8> {
        std::fs::read(self.path.join(rel_path))
            .unwrap_or_else(|err| panic!("{} should exist: {}", rel_path, err))
    }

    fn exists(&self, rel_path: &str) -> bool {
        self.path.join(rel_path).exists()
    }

    fn read_trashed(&self, rel_path: &str) -> Vec<u8> {
        std::fs::read(self.trash_batch.join(rel_path))
            .unwrap_or_else(|err| panic!("{} should be in the trash: {}", rel_path, err))
    }

    fn entry_names(&self, rel_dir: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.path.join(rel_dir))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    async fn run(
        &self,
        store: &TestStore,
        from_root: Option<&Hash>,
        to_root: &Hash,
    ) -> Result<MaterializeStats, MaterializeError> {
        materialize(
            &store.nodes,
            &store.blobs,
            &self.path,
            &self.trash_batch,
            from_root,
            to_root,
        )
        .await
    }
}

/// `docs/hello.txt` + `media/song.mp3`, the shape most of these tests start from.
async fn two_folder_tree(store: &mut TestStore) -> (Hash, Hash) {
    let hello = store.add_file("hello.txt", b"hello").await;
    let docs = store.add_folder("docs", vec![], vec![hello]).await;
    let song = store.add_file("song.mp3", b"song").await;
    let media = store.add_folder("media", vec![], vec![song]).await;
    let media_hash = media.hash.clone();
    let root = store.add_folder("", vec![docs, media], vec![]).await;

    (root.hash, media_hash)
}

#[tokio::test]
async fn writes_the_whole_tree_into_an_empty_vault() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (root, _) = two_folder_tree(&mut store).await;

    let stats = vault.run(&store, None, &root).await.unwrap();

    assert_eq!(vault.read("docs/hello.txt"), b"hello");
    assert_eq!(vault.read("media/song.mp3"), b"song");
    assert_eq!(stats.files_written, 2);
    assert_eq!(stats.folders_created, 2);
    assert_eq!(stats.entries_trashed, 0);
}

#[tokio::test]
async fn leaves_no_temp_files_behind() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (root, _) = two_folder_tree(&mut store).await;

    vault.run(&store, None, &root).await.unwrap();

    assert_eq!(vault.entry_names("docs"), vec!["hello.txt".to_string()]);
}

#[tokio::test]
async fn overwrites_a_file_whose_content_changed() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (before, _) = two_folder_tree(&mut store).await;
    vault.run(&store, None, &before).await.unwrap();

    let hello = store.add_file("hello.txt", b"hello again").await;
    let docs = store.add_folder("docs", vec![], vec![hello]).await;
    let song = store.add_file("song.mp3", b"song").await;
    let media = store.add_folder("media", vec![], vec![song]).await;
    let after = store.add_folder("", vec![docs, media], vec![]).await.hash;

    let stats = vault.run(&store, Some(&before), &after).await.unwrap();

    assert_eq!(vault.read("docs/hello.txt"), b"hello again");
    assert_eq!(stats.files_written, 1);
    assert_eq!(stats.entries_trashed, 0);
}

#[tokio::test]
async fn an_unchanged_subtree_is_never_walked() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (before, media_hash) = two_folder_tree(&mut store).await;
    vault.run(&store, None, &before).await.unwrap();

    let hello = store.add_file("hello.txt", b"hello again").await;
    let docs = store.add_folder("docs", vec![], vec![hello]).await;
    let song = store.add_file("song.mp3", b"song").await;
    let media = store.add_folder("media", vec![], vec![song]).await;
    let after = store.add_folder("", vec![docs, media], vec![]).await.hash;

    let recording = RecordingStore::new(&store.nodes);
    materialize(
        &recording,
        &store.blobs,
        &vault.path,
        &vault.trash_batch,
        Some(&before),
        &after,
    )
    .await
    .unwrap();

    let requested = recording.requested.read().unwrap();
    assert!(
        !requested.contains(&media_hash),
        "media/ did not change, so its node should never have been read"
    );
}

#[tokio::test]
async fn a_removed_file_moves_to_the_trash() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (before, _) = two_folder_tree(&mut store).await;
    vault.run(&store, None, &before).await.unwrap();

    let docs = store.add_folder("docs", vec![], vec![]).await;
    let song = store.add_file("song.mp3", b"song").await;
    let media = store.add_folder("media", vec![], vec![song]).await;
    let after = store.add_folder("", vec![docs, media], vec![]).await.hash;

    let stats = vault.run(&store, Some(&before), &after).await.unwrap();

    assert!(!vault.exists("docs/hello.txt"));
    assert_eq!(vault.read_trashed("docs/hello.txt"), b"hello");
    assert_eq!(stats.entries_trashed, 1);
}

#[tokio::test]
async fn a_removed_folder_moves_to_the_trash_whole() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (before, _) = two_folder_tree(&mut store).await;
    vault.run(&store, None, &before).await.unwrap();

    let song = store.add_file("song.mp3", b"song").await;
    let media = store.add_folder("media", vec![], vec![song]).await;
    let after = store.add_folder("", vec![media], vec![]).await.hash;

    vault.run(&store, Some(&before), &after).await.unwrap();

    assert!(!vault.exists("docs"));
    assert_eq!(vault.read_trashed("docs/hello.txt"), b"hello");
}

#[tokio::test]
async fn a_locally_edited_file_is_trashed_before_it_is_overwritten() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (before, _) = two_folder_tree(&mut store).await;
    vault.run(&store, None, &before).await.unwrap();

    // The user edits the file after the scan that produced `before`.
    vault.write("docs/hello.txt", b"edited by hand");

    let hello = store.add_file("hello.txt", b"hello again").await;
    let docs = store.add_folder("docs", vec![], vec![hello]).await;
    let song = store.add_file("song.mp3", b"song").await;
    let media = store.add_folder("media", vec![], vec![song]).await;
    let after = store.add_folder("", vec![docs, media], vec![]).await.hash;

    let stats = vault.run(&store, Some(&before), &after).await.unwrap();

    assert_eq!(vault.read("docs/hello.txt"), b"hello again");
    assert_eq!(vault.read_trashed("docs/hello.txt"), b"edited by hand");
    assert_eq!(stats.entries_trashed, 1);
}

#[tokio::test]
async fn a_file_replaced_by_a_folder_swaps_cleanly() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let hello = store.add_file("hello", b"a file").await;
    let before = store.add_folder("", vec![], vec![hello]).await.hash;
    vault.run(&store, None, &before).await.unwrap();

    let nested = store.add_file("nested.txt", b"now a folder").await;
    let hello_folder = store.add_folder("hello", vec![], vec![nested]).await;
    let after = store.add_folder("", vec![hello_folder], vec![]).await.hash;

    vault.run(&store, Some(&before), &after).await.unwrap();

    assert_eq!(vault.read("hello/nested.txt"), b"now a folder");
    assert_eq!(vault.read_trashed("hello"), b"a file");
}

#[tokio::test]
async fn a_folder_replaced_by_a_file_swaps_cleanly() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let nested = store.add_file("nested.txt", b"in a folder").await;
    let hello_folder = store.add_folder("hello", vec![], vec![nested]).await;
    let before = store.add_folder("", vec![hello_folder], vec![]).await.hash;
    vault.run(&store, None, &before).await.unwrap();

    let hello = store.add_file("hello", b"now a file").await;
    let after = store.add_folder("", vec![], vec![hello]).await.hash;

    vault.run(&store, Some(&before), &after).await.unwrap();

    assert_eq!(vault.read("hello"), b"now a file");
    assert_eq!(vault.read_trashed("hello/nested.txt"), b"in a folder");
}

#[tokio::test]
async fn running_twice_touches_nothing_the_second_time() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (root, _) = two_folder_tree(&mut store).await;

    vault.run(&store, None, &root).await.unwrap();
    // Same call again, still with no `from` to go on: the content is already
    // right, so nothing is rewritten and nothing lands in the trash.
    let stats = vault.run(&store, None, &root).await.unwrap();

    assert!(stats.touched_nothing(), "second run did work: {:?}", stats);
    assert!(!vault.trash_batch.exists());
    assert_eq!(vault.read("docs/hello.txt"), b"hello");
}

#[tokio::test]
async fn phaneros_internal_entries_are_left_alone() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (root, _) = two_folder_tree(&mut store).await;

    vault.write(".phaneros/trash/0/docs/old.txt", b"deleted earlier");
    vault.write("docs/gone.txt.phaneros-tmp", b"half writ");

    let stats = vault.run(&store, None, &root).await.unwrap();

    assert_eq!(
        vault.read(".phaneros/trash/0/docs/old.txt"),
        b"deleted earlier"
    );
    assert_eq!(vault.read("docs/gone.txt.phaneros-tmp"), b"half writ");
    assert_eq!(stats.entries_trashed, 0);
}

#[tokio::test]
async fn a_stale_temp_file_is_reclaimed_by_the_write_it_belonged_to() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let (root, _) = two_folder_tree(&mut store).await;

    // Left behind by a write that died before its rename.
    vault.write("docs/hello.txt.phaneros-tmp", b"half writ");

    vault.run(&store, None, &root).await.unwrap();

    assert_eq!(vault.read("docs/hello.txt"), b"hello");
    assert!(!vault.exists("docs/hello.txt.phaneros-tmp"));
}

#[tokio::test]
async fn a_missing_blob_aborts_without_writing_the_file() {
    let vault = Vault::new();
    let mut store = TestStore::new();

    // A file node whose blob never made it into the local store.
    let (orphan_hash, orphan) = Node::file(vec![BlobRef::from_bytes(b"never stored")]);
    store.nodes.insert(orphan_hash.clone(), orphan).await.unwrap();
    let root = store
        .add_folder("", vec![], vec![Entry::new("orphan.txt", orphan_hash)])
        .await
        .hash;

    let error = vault.run(&store, None, &root).await.unwrap_err();

    assert!(
        matches!(error, MaterializeError::MissingBlob { .. }),
        "unexpected error: {:?}",
        error
    );
    assert!(!vault.exists("orphan.txt"));
}

#[tokio::test]
async fn nested_empty_folders_are_created() {
    let vault = Vault::new();
    let mut store = TestStore::new();
    let inner = store.add_folder("inner", vec![], vec![]).await;
    let outer = store.add_folder("outer", vec![inner], vec![]).await;
    let root = store.add_folder("", vec![outer], vec![]).await.hash;

    let stats = vault.run(&store, None, &root).await.unwrap();

    assert!(vault.path.join("outer/inner").is_dir());
    assert_eq!(stats.folders_created, 2);
    assert_eq!(stats.files_written, 0);
}
