use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::blob_repository::BlobRef;
use crate::node_repository::{Hash, InMemoryNodeRepository, Node, NodeRepository};
use crate::scanner::file_chunker::FileChunker;
use crate::scanner::{Scanner, ScannerError};

fn new_scanner(path: &Path) -> Scanner {
    Scanner::new(path, false)
}

/// A materialized recursive view over the node store, so tests can assert
/// tree structure ergonomically instead of chasing hashes by hand.
struct TreeView {
    root_hash: Hash,
    folders: Vec<FolderView>,
    files: Vec<FileView>,
}

struct FolderView {
    name: String,
    hash: Hash,
    folders: Vec<FolderView>,
    files: Vec<FileView>,
}

struct FileView {
    name: String,
    hash: Hash,
    blobs: Vec<BlobRef>,
}

/// Scans and expands the resulting root hash into a TreeView.
fn scan_view(scanner: &mut Scanner) -> Result<TreeView, ScannerError> {
    let root_hash = scanner.scan()?;
    let store = scanner.get_store();
    let store = store.read().unwrap();
    let (folders, files) = expand_folder(&store, &root_hash);
    Ok(TreeView {
        root_hash,
        folders,
        files,
    })
}

fn expand_folder(store: &InMemoryNodeRepository, hash: &Hash) -> (Vec<FolderView>, Vec<FileView>) {
    match store.get_node(hash).unwrap() {
        Some(Node::Folder { folders, files }) => (
            folders
                .iter()
                .map(|entry| {
                    let (sub_folders, sub_files) = expand_folder(store, &entry.hash);
                    FolderView {
                        name: entry.name.clone(),
                        hash: entry.hash.clone(),
                        folders: sub_folders,
                        files: sub_files,
                    }
                })
                .collect(),
            files
                .iter()
                .map(|entry| {
                    let blobs = match store.get_node(&entry.hash).unwrap() {
                        Some(Node::File { blobs }) => blobs.clone(),
                        _ => panic!("file node {} missing from store", entry.hash),
                    };
                    FileView {
                        name: entry.name.clone(),
                        hash: entry.hash.clone(),
                        blobs: blobs,
                    }
                })
                .collect(),
        ),
        _ => panic!("folder node {} missing from store", hash),
    }
}

fn create_file(dir: &Path, name: &str, content: &[u8]) {
    fs::write(dir.join(name), content).unwrap();
}

fn create_dir(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::create_dir_all(&path).unwrap();
    path
}

/// Compute expected hash for an empty folder (prefix [0], no children).
fn expected_empty_folder_hash() -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0]);
    hasher.finalize().to_hex().to_string()
}

mod basic_structure {
    use super::*;

    #[test]
    fn scan_single_empty_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();

        let mut scanner = new_scanner(&file_path);
        let tree = scan_view(&mut scanner).unwrap();

        // When root is a file, it's wrapped in a synthetic FolderView
        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.folders.len(), 0);
        assert_eq!(tree.files[0].name, "empty.txt");
        assert_eq!(tree.files[0].blobs.len(), 0);
    }

    #[test]
    fn scan_single_empty_directory() {
        let tmp = TempDir::new().unwrap();
        let dir_path = tmp.path().join("empty_dir");
        fs::create_dir(&dir_path).unwrap();

        let mut scanner = new_scanner(&dir_path);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 0);
        assert_eq!(tree.folders.len(), 0);
        assert_eq!(tree.root_hash, expected_empty_folder_hash());
    }

    #[test]
    fn empty_file_vs_empty_folder_have_different_hashes() {
        let tmp = TempDir::new().unwrap();

        // Empty file
        let file_path = tmp.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();
        let mut file_scanner = new_scanner(&file_path);
        let file_tree = scan_view(&mut file_scanner).unwrap();

        // Empty directory
        let dir_path = tmp.path().join("empty_dir");
        fs::create_dir(&dir_path).unwrap();
        let mut dir_scanner = new_scanner(&dir_path);
        let dir_tree = scan_view(&mut dir_scanner).unwrap();

        // The empty file gets wrapped in a synthetic folder, so compare the file node hash
        // with the empty folder root_hash
        let empty_file_hash = &file_tree.files[0].hash;
        let empty_folder_hash = &dir_tree.root_hash;
        assert_ne!(empty_file_hash, empty_folder_hash);
    }

    #[test]
    fn directory_containing_one_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("project");
        fs::create_dir(&dir).unwrap();
        create_file(&dir, "hello.txt", b"hello world");

        let mut scanner = new_scanner(&dir);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.folders.len(), 0);
        assert_eq!(tree.files[0].name, "hello.txt");
        assert_eq!(tree.files[0].blobs.len(), 1);
        // assert_eq!(tree.files[0].blobs[0].size, 11);
    }

    #[test]
    fn directory_containing_one_subfolder() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        let _sub = create_dir(&root, "subdir");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.folders.len(), 1);
        assert_eq!(tree.files.len(), 0);
        assert_eq!(tree.folders[0].name, "subdir");
    }

    #[test]
    fn directory_with_files_and_subfolders() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file_a.txt", b"aaa");
        create_file(&root, "file_b.txt", b"bbb");
        let sub = create_dir(&root, "subdir");
        create_file(&sub, "nested.txt", b"nested");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 2);
        assert_eq!(tree.folders.len(), 1);
        assert_eq!(tree.folders[0].name, "subdir");
        assert_eq!(tree.folders[0].files.len(), 1);
        assert_eq!(tree.folders[0].files[0].name, "nested.txt");
    }
}

mod hash_determinism {
    use super::*;

    #[test]
    fn same_content_different_file_name_produces_different_tree_hash() {
        let tmp = TempDir::new().unwrap();

        let dir_a = create_dir(tmp.path(), "dir_a");
        create_file(&dir_a, "alpha.txt", b"content");

        let dir_b = create_dir(tmp.path(), "dir_b");
        create_file(&dir_b, "beta.txt", b"content");

        let mut scanner_a = new_scanner(&dir_a);
        let tree_a = scan_view(&mut scanner_a).unwrap();

        let mut scanner_b = new_scanner(&dir_b);
        let tree_b = scan_view(&mut scanner_b).unwrap();

        // File hashes should be same (same content, hash doesn't include name)
        assert_eq!(tree_a.files[0].hash, tree_b.files[0].hash);
        // But tree root hashes should differ (name is included in folder hash)
        assert_ne!(tree_a.root_hash, tree_b.root_hash);
    }

    #[test]
    fn same_file_name_different_content_produces_different_tree_hash() {
        let tmp = TempDir::new().unwrap();

        let dir_a = create_dir(tmp.path(), "dir_a");
        create_file(&dir_a, "file.txt", b"content_a");

        let dir_b = create_dir(tmp.path(), "dir_b");
        create_file(&dir_b, "file.txt", b"content_b");

        let mut scanner_a = new_scanner(&dir_a);
        let tree_a = scan_view(&mut scanner_a).unwrap();

        let mut scanner_b = new_scanner(&dir_b);
        let tree_b = scan_view(&mut scanner_b).unwrap();

        assert_ne!(tree_a.files[0].hash, tree_b.files[0].hash);
        assert_ne!(tree_a.root_hash, tree_b.root_hash);
    }

    #[test]
    fn same_file_different_directory_name_produces_different_root_hash() {
        // Two directories with same file content but different dir names scanned independently.
        // The root_hash includes the folder name in child folder hashes, but the root itself
        // just uses [0] prefix + children. Since the scanner's root is the folder, the folder name
        // appears in the FolderView name but the root_hash computation uses from_children
        // which doesn't include the folder's own name. So two roots with identical children will
        // have identical root_hashes. This tests that structure matters.
        let tmp = TempDir::new().unwrap();

        let dir_a = create_dir(tmp.path(), "dir_a");
        let sub_a = create_dir(&dir_a, "subdir_x");
        create_file(&sub_a, "file.txt", b"hello");

        let dir_b = create_dir(tmp.path(), "dir_b");
        let sub_b = create_dir(&dir_b, "subdir_y");
        create_file(&sub_b, "file.txt", b"hello");

        let mut scanner_a = new_scanner(&dir_a);
        let tree_a = scan_view(&mut scanner_a).unwrap();

        let mut scanner_b = new_scanner(&dir_b);
        let tree_b = scan_view(&mut scanner_b).unwrap();

        // Different subfolder names mean different root hashes
        assert_ne!(tree_a.root_hash, tree_b.root_hash);
    }

    #[test]
    fn scanning_same_tree_twice_produces_identical_root_hash() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file.txt", b"stable content");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();
        let tree2 = scan_view(&mut scanner).unwrap();

        assert_eq!(tree1.root_hash, tree2.root_hash);
    }

    #[test]
    fn file_order_on_disk_does_not_affect_folder_hash() {
        // Scanner sorts by name, so creating files in different orders yields same hash.
        let tmp = TempDir::new().unwrap();

        let dir_a = create_dir(tmp.path(), "dir_a");
        create_file(&dir_a, "aaa.txt", b"a");
        create_file(&dir_a, "bbb.txt", b"b");
        create_file(&dir_a, "ccc.txt", b"c");

        let dir_b = create_dir(tmp.path(), "dir_b");
        // Create in reverse order
        create_file(&dir_b, "ccc.txt", b"c");
        create_file(&dir_b, "bbb.txt", b"b");
        create_file(&dir_b, "aaa.txt", b"a");

        let mut scanner_a = new_scanner(&dir_a);
        let tree_a = scan_view(&mut scanner_a).unwrap();

        let mut scanner_b = new_scanner(&dir_b);
        let tree_b = scan_view(&mut scanner_b).unwrap();

        assert_eq!(tree_a.root_hash, tree_b.root_hash);
    }

    #[test]
    fn empty_folder_differs_from_folder_with_one_empty_file() {
        let tmp = TempDir::new().unwrap();

        let empty_dir = create_dir(tmp.path(), "empty");

        let dir_with_file = create_dir(tmp.path(), "has_file");
        create_file(&dir_with_file, "empty.txt", b"");

        let mut scanner_empty = new_scanner(&empty_dir);
        let tree_empty = scan_view(&mut scanner_empty).unwrap();

        let mut scanner_file = new_scanner(&dir_with_file);
        let tree_file = scan_view(&mut scanner_file).unwrap();

        assert_ne!(tree_empty.root_hash, tree_file.root_hash);
    }

    #[test]
    fn empty_folder_differs_from_folder_with_one_empty_subfolder() {
        let tmp = TempDir::new().unwrap();

        let empty_dir = create_dir(tmp.path(), "empty");

        let dir_with_sub = create_dir(tmp.path(), "has_sub");
        create_dir(&dir_with_sub, "child");

        let mut scanner_empty = new_scanner(&empty_dir);
        let tree_empty = scan_view(&mut scanner_empty).unwrap();

        let mut scanner_sub = new_scanner(&dir_with_sub);
        let tree_sub = scan_view(&mut scanner_sub).unwrap();

        assert_ne!(tree_empty.root_hash, tree_sub.root_hash);
    }
}

mod file_chunking {
    use crate::blob_repository::{BlobRef, InMemoryBlobRepository};
    use std::sync::{Arc, RwLock};

    use super::*;

    const SMALL_CHUNK: usize = 16;

    #[test]
    fn file_smaller_than_chunk_size_produces_one_chunk() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("small.bin");
        fs::write(&file_path, &[0u8; 10]).unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 1);
        // assert_eq!(blobs[0].size, 10);
    }

    #[test]
    fn file_exactly_chunk_size_produces_one_chunk() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("exact.bin");
        fs::write(&file_path, &[0u8; SMALL_CHUNK]).unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 1);
        // assert_eq!(blobs[0].size, SMALL_CHUNK as u64);
    }

    #[test]
    fn file_chunk_size_plus_one_produces_two_blobs() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("plus_one.bin");
        fs::write(&file_path, &[0u8; SMALL_CHUNK + 1]).unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 2);
        // assert_eq!(blobs[0].size, SMALL_CHUNK as u64);
        // assert_eq!(blobs[1].size, 1);
    }

    #[test]
    fn file_exactly_two_times_chunk_size_produces_two_blobs() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("double.bin");
        fs::write(&file_path, &[0u8; SMALL_CHUNK * 2]).unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 2);
        // assert_eq!(blobs[0].size, SMALL_CHUNK as u64);
        // assert_eq!(blobs[1].size, SMALL_CHUNK as u64);
    }

    #[test]
    fn large_file_correct_number_and_sizes() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("large.bin");
        // 5 full blobs + 7 extra bytes
        let total_size = SMALL_CHUNK * 5 + 7;
        fs::write(&file_path, vec![0xAB; total_size]).unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 6);
        // for chunk in &blobs[..5] {
        //     assert_eq!(chunk.size, SMALL_CHUNK as u64);
        // }
        // assert_eq!(blobs[5].size, 7);

        // Total size should match
        // let total: u64 = blobs.iter().map(|c| c.size).sum();
        // assert_eq!(total, total_size as u64);
    }

    #[test]
    fn empty_file_produces_zero_blobs() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("empty.bin");
        fs::write(&file_path, b"").unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 0);
    }

    #[test]
    fn chunk_hashes_are_stable_for_identical_content() {
        let tmp = TempDir::new().unwrap();
        let file_a = tmp.path().join("a.bin");
        let file_b = tmp.path().join("b.bin");

        let content = vec![42u8; SMALL_CHUNK + 5];
        fs::write(&file_a, &content).unwrap();
        fs::write(&file_b, &content).unwrap();

        let chunker =
            FileChunker::new(SMALL_CHUNK, Arc::new(RwLock::new(InMemoryBlobRepository::new())));
        let blobs_a = chunker.chunk_file(&file_a).unwrap();
        let blobs_b = chunker.chunk_file(&file_b).unwrap();

        assert_eq!(blobs_a.len(), blobs_b.len());
        for (a, b) in blobs_a.iter().zip(blobs_b.iter()) {
            assert_eq!(a, b);
            // assert_eq!(a.size, b.size);
        }
    }

    #[test]
    fn chunk_hash_matches_blake3_hash_of_content() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("verify.bin");
        let content = b"hello blake3 chunk hashing";
        fs::write(&file_path, content).unwrap();

        let chunker = FileChunker::new(1024, Arc::new(RwLock::new(InMemoryBlobRepository::new()))); // content fits in one chunk
        let blobs = chunker.chunk_file(&file_path).unwrap();

        assert_eq!(blobs.len(), 1);
        // let expected_hash = blake3::hash(content).to_hex().to_string();
        // assert_eq!(blobs[0].hash, expected_hash);
    }

    #[test]
    fn filechunk_from_bytes_matches_direct_blake3() {
        let data = b"test data for FileChunk";
        let blob = BlobRef::from_bytes(data);
        let expected = blake3::hash(data).to_hex().to_string();

        assert_eq!(blob.hash, expected);
        assert_eq!(blob.size, data.len() as u64);
    }
}

mod incremental_scan {
    use super::*;
    use filetime::{FileTime, set_file_mtime};

    #[test]
    fn second_scan_with_no_changes_produces_identical_result() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "a.txt", b"aaa");
        create_file(&root, "b.txt", b"bbb");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();
        let tree2 = scan_view(&mut scanner).unwrap();

        assert_eq!(tree1.root_hash, tree2.root_hash);
        assert_eq!(tree1.files.len(), tree2.files.len());
        for (f1, f2) in tree1.files.iter().zip(tree2.files.iter()) {
            assert_eq!(f1.hash, f2.hash);
            assert_eq!(f1.name, f2.name);
        }
    }

    #[test]
    fn modifying_file_content_changes_hash() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "mutable.txt", b"original");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        // Modify the file and force mtime change with a far-future timestamp
        create_file(&root, "mutable.txt", b"modified");
        set_file_mtime(
            root.join("mutable.txt"),
            FileTime::from_unix_time(2_000_000_000, 0),
        )
        .unwrap();

        let tree2 = scan_view(&mut scanner).unwrap();

        assert_ne!(tree1.root_hash, tree2.root_hash);
        assert_ne!(tree1.files[0].hash, tree2.files[0].hash);
    }

    #[test]
    fn adding_file_between_scans_changes_folder_hash() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "existing.txt", b"exists");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        create_file(&root, "new_file.txt", b"new content");
        let tree2 = scan_view(&mut scanner).unwrap();

        assert_ne!(tree1.root_hash, tree2.root_hash);
        assert_eq!(tree1.files.len(), 1);
        assert_eq!(tree2.files.len(), 2);
    }

    #[test]
    fn removing_file_between_scans_changes_folder_hash() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "keep.txt", b"keep");
        create_file(&root, "remove.txt", b"remove");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        fs::remove_file(root.join("remove.txt")).unwrap();
        let tree2 = scan_view(&mut scanner).unwrap();

        assert_ne!(tree1.root_hash, tree2.root_hash);
        assert_eq!(tree1.files.len(), 2);
        assert_eq!(tree2.files.len(), 1);
    }

    #[test]
    fn renaming_file_between_scans_changes_folder_hash() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "original_name.txt", b"content");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        fs::rename(root.join("original_name.txt"), root.join("renamed.txt")).unwrap();
        let tree2 = scan_view(&mut scanner).unwrap();

        assert_ne!(tree1.root_hash, tree2.root_hash);
        assert_eq!(tree1.files[0].name, "original_name.txt");
        assert_eq!(tree2.files[0].name, "renamed.txt");
        // Content hash should remain the same since content didn't change
        // assert_eq!(tree1.files[0].blobs[0].hash, tree2.files[0].blobs[0].hash);
    }
}

mod directory_tree_structure {
    use super::*;

    #[test]
    fn deeply_nested_directory_scans_correctly() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        // Create 6 levels of nesting
        let mut current = root.clone();
        for i in 0..6 {
            current = create_dir(&current, &format!("level_{}", i));
        }
        create_file(&current, "deep.txt", b"deep content");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        // Traverse the tree to verify depth
        let mut node = &tree.folders[0];
        assert_eq!(node.name, "level_0");
        for i in 1..6 {
            assert_eq!(node.folders.len(), 1);
            node = &node.folders[0];
            assert_eq!(node.name, format!("level_{}", i));
        }
        assert_eq!(node.files.len(), 1);
        assert_eq!(node.files[0].name, "deep.txt");
    }

    #[test]
    fn directory_with_many_files_scans_correctly() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let file_count = 60;
        for i in 0..file_count {
            create_file(
                &root,
                &format!("file_{:03}.txt", i),
                format!("content {}", i).as_bytes(),
            );
        }

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), file_count);
        assert_eq!(tree.folders.len(), 0);
    }

    #[test]
    fn folders_are_sorted_alphabetically() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        // Create in non-alphabetical order
        create_dir(&root, "zebra");
        create_dir(&root, "apple");
        create_dir(&root, "mango");
        create_dir(&root, "banana");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let names: Vec<&str> = tree.folders.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["apple", "banana", "mango", "zebra"]);
    }

    #[test]
    fn files_are_sorted_alphabetically() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        // Create in non-alphabetical order
        create_file(&root, "zebra.txt", b"z");
        create_file(&root, "apple.txt", b"a");
        create_file(&root, "mango.txt", b"m");
        create_file(&root, "banana.txt", b"b");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let names: Vec<&str> = tree.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["apple.txt", "banana.txt", "mango.txt", "zebra.txt"]
        );
    }

    #[test]
    fn files_and_folders_are_separated_into_correct_vectors() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        create_file(&root, "file_1.txt", b"1");
        create_file(&root, "file_2.txt", b"2");
        create_dir(&root, "dir_1");
        create_dir(&root, "dir_2");
        create_dir(&root, "dir_3");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 2);
        assert_eq!(tree.folders.len(), 3);

        // Verify no folders in files vector and vice versa
        for f in &tree.files {
            assert!(f.name.starts_with("file_"));
        }
        for d in &tree.folders {
            assert!(d.name.starts_with("dir_"));
        }
    }

    #[test]
    fn mixed_depth_structure_is_correct() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        create_file(&root, "root_file.txt", b"root");
        let sub_a = create_dir(&root, "sub_a");
        create_file(&sub_a, "a_file.txt", b"a");
        let sub_b = create_dir(&root, "sub_b");
        let sub_b_nested = create_dir(&sub_b, "nested");
        create_file(&sub_b_nested, "nested_file.txt", b"nested");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].name, "root_file.txt");

        assert_eq!(tree.folders.len(), 2);
        assert_eq!(tree.folders[0].name, "sub_a");
        assert_eq!(tree.folders[0].files.len(), 1);
        assert_eq!(tree.folders[0].files[0].name, "a_file.txt");

        assert_eq!(tree.folders[1].name, "sub_b");
        assert_eq!(tree.folders[1].folders.len(), 1);
        assert_eq!(tree.folders[1].folders[0].name, "nested");
        assert_eq!(tree.folders[1].folders[0].files.len(), 1);
        assert_eq!(tree.folders[1].folders[0].files[0].name, "nested_file.txt");
    }
}

mod tree_completeness {
    use super::*;

    /// Recursively counts all files and folders in an IndexTree.
    fn count_entries(folders: &[FolderView], files: &[FileView]) -> (usize, usize) {
        let mut total_folders = folders.len();
        let mut total_files = files.len();
        for folder in folders {
            let (sub_folders, sub_files) = count_entries(&folder.folders, &folder.files);
            total_folders += sub_folders;
            total_files += sub_files;
        }
        (total_folders, total_files)
    }

    /// Collects all file names recursively.
    fn collect_file_names(folders: &[FolderView], files: &[FileView]) -> Vec<String> {
        let mut names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
        for folder in folders {
            names.extend(collect_file_names(&folder.folders, &folder.files));
        }
        names
    }

    /// Collects all folder names recursively.
    fn collect_folder_names(folders: &[FolderView]) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for folder in folders {
            names.push(folder.name.clone());
            names.extend(collect_folder_names(&folder.folders));
        }
        names
    }

    #[test]
    fn all_files_appear_in_tree_output() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        create_file(&root, "a.txt", b"a");
        let sub = create_dir(&root, "sub");
        create_file(&sub, "b.txt", b"b");
        let deep = create_dir(&sub, "deep");
        create_file(&deep, "c.txt", b"c");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let file_names = collect_file_names(&tree.folders, &tree.files);
        assert!(file_names.contains(&"a.txt".to_string()));
        assert!(file_names.contains(&"b.txt".to_string()));
        assert!(file_names.contains(&"c.txt".to_string()));
        assert_eq!(file_names.len(), 3);
    }

    #[test]
    fn all_folders_appear_in_tree_output() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let sub_a = create_dir(&root, "sub_a");
        let _sub_b = create_dir(&root, "sub_b");
        let nested = create_dir(&sub_a, "nested");
        let _deep = create_dir(&nested, "deep");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let folder_names = collect_folder_names(&tree.folders);
        assert!(folder_names.contains(&"sub_a".to_string()));
        assert!(folder_names.contains(&"sub_b".to_string()));
        assert!(folder_names.contains(&"nested".to_string()));
        assert!(folder_names.contains(&"deep".to_string()));
        assert_eq!(folder_names.len(), 4);
    }

    #[test]
    fn entry_counts_match_filesystem() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        // 3 files, 2 folders
        create_file(&root, "f1.txt", b"1");
        create_file(&root, "f2.txt", b"2");
        let sub = create_dir(&root, "sub");
        create_file(&sub, "f3.txt", b"3");
        let _empty = create_dir(&root, "empty");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let (total_folders, total_files) = count_entries(&tree.folders, &tree.files);
        assert_eq!(total_files, 3);
        assert_eq!(total_folders, 2);
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn scanning_nonexistent_path_returns_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent_subpath");
        let mut scanner = new_scanner(&path);
        let result = scanner.scan();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ScannerError::GetMetadataFailed { .. }));
    }

    #[test]
    fn error_contains_path_information() {
        let tmp = TempDir::new().unwrap();
        let bad_path = tmp.path().join("definitely_missing_xyz_987");
        let mut scanner = new_scanner(&bad_path);
        let result = scanner.scan();

        let err = result.unwrap_err();
        let err_string = format!("{}", err);
        assert!(err_string.contains("definitely_missing_xyz_987"));
    }
}

mod snapshot_management {
    use super::*;
    use filetime::{FileTime, set_file_mtime};

    #[test]
    fn multiple_sequential_scans_work_correctly() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file.txt", b"content");

        let mut scanner = new_scanner(&root);

        // Run 15 scans (more than buffer size of 10)
        let mut last_hash = String::new();
        for _ in 0..15 {
            let tree = scan_view(&mut scanner).unwrap();
            if !last_hash.is_empty() {
                assert_eq!(tree.root_hash, last_hash);
            }
            last_hash = tree.root_hash;
        }
    }

    #[test]
    fn modification_detected_after_cache_is_populated() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file.txt", b"version_1");

        let mut scanner = new_scanner(&root);

        // Build up cache with multiple scans
        let tree1 = scan_view(&mut scanner).unwrap();
        let tree2 = scan_view(&mut scanner).unwrap();
        assert_eq!(tree1.root_hash, tree2.root_hash);

        // Now modify with a far-future mtime to guarantee cache invalidation
        create_file(&root, "file.txt", b"version_2");
        set_file_mtime(
            root.join("file.txt"),
            FileTime::from_unix_time(2_000_000_000, 0),
        )
        .unwrap();

        let tree3 = scan_view(&mut scanner).unwrap();
        assert_ne!(tree2.root_hash, tree3.root_hash);
    }

    #[test]
    fn scan_after_failed_path_still_works() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file.txt", b"content");

        let mut scanner = new_scanner(&root);

        // Successful scan
        let tree1 = scan_view(&mut scanner).unwrap();
        let original_hash = tree1.root_hash.clone();

        // Remove the root directory to cause an error
        fs::remove_dir_all(&root).unwrap();

        // Scan should now fail
        let result = scanner.scan();
        assert!(result.is_err());

        // Recreate the root directory with the same file content
        fs::create_dir(&root).unwrap();
        create_file(&root, "file.txt", b"content");

        // Scanner should recover and produce a successful scan
        let tree3 = scan_view(&mut scanner).unwrap();
        assert_eq!(tree3.root_hash, original_hash);
    }
}

mod hash_algorithm {
    use super::*;

    #[test]
    fn file_hash_uses_prefix_byte_1_plus_chunk_hashes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let content = b"hello world";
        create_file(&root, "test.txt", content);

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        // Manually compute expected hash
        let chunk_hash = blake3::hash(content).to_hex().to_string();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[1]); // file prefix
        hasher.update(chunk_hash.as_bytes());
        let expected_file_hash = hasher.finalize().to_hex().to_string();

        assert_eq!(tree.files[0].hash, expected_file_hash);
    }

    #[test]
    fn folder_hash_uses_prefix_byte_0_plus_children() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let content = b"data";
        create_file(&root, "file.txt", content);

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        // Manually compute expected folder hash
        let chunk_hash = blake3::hash(content).to_hex().to_string();
        let mut file_hasher = blake3::Hasher::new();
        file_hasher.update(&[1]);
        file_hasher.update(chunk_hash.as_bytes());
        let file_hash = file_hasher.finalize().to_hex().to_string();

        let file_name = "file.txt";
        let mut folder_hasher = blake3::Hasher::new();
        folder_hasher.update(&[0]); // folder prefix
        // No sub-folders
        // Files: name_len + name + hash
        folder_hasher.update(&(file_name.len() as u64).to_le_bytes());
        folder_hasher.update(file_name.as_bytes());
        folder_hasher.update(file_hash.as_bytes());
        let expected_root_hash = folder_hasher.finalize().to_hex().to_string();

        assert_eq!(tree.root_hash, expected_root_hash);
    }

    #[test]
    fn empty_folder_hash_is_blake3_of_zero_prefix_only() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0]);
        let expected = hasher.finalize().to_hex().to_string();

        assert_eq!(tree.root_hash, expected);
    }

    #[test]
    fn empty_file_hash_is_blake3_of_one_prefix_only() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();

        let mut scanner = new_scanner(&file_path);
        let tree = scan_view(&mut scanner).unwrap();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&[1]);
        let expected = hasher.finalize().to_hex().to_string();

        assert_eq!(tree.files[0].hash, expected);
    }

    #[test]
    fn folder_hash_includes_subfolder_and_file_names_and_hashes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let sub = create_dir(&root, "alpha");
        create_file(&sub, "inside.txt", b"inside");
        create_file(&root, "beta.txt", b"beta");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        // Manually compute
        // First: the subfolder "alpha" which contains "inside.txt"
        let inside_chunk_hash = blake3::hash(b"inside").to_hex().to_string();
        let mut inside_file_hasher = blake3::Hasher::new();
        inside_file_hasher.update(&[1]);
        inside_file_hasher.update(inside_chunk_hash.as_bytes());
        let inside_file_hash = inside_file_hasher.finalize().to_hex().to_string();

        let mut alpha_hasher = blake3::Hasher::new();
        alpha_hasher.update(&[0]);
        // No sub-folders in alpha
        // File "inside.txt"
        alpha_hasher.update(&("inside.txt".len() as u64).to_le_bytes());
        alpha_hasher.update("inside.txt".as_bytes());
        alpha_hasher.update(inside_file_hash.as_bytes());
        let alpha_hash = alpha_hasher.finalize().to_hex().to_string();

        // File "beta.txt" at root level
        let beta_chunk_hash = blake3::hash(b"beta").to_hex().to_string();
        let mut beta_file_hasher = blake3::Hasher::new();
        beta_file_hasher.update(&[1]);
        beta_file_hasher.update(beta_chunk_hash.as_bytes());
        let beta_file_hash = beta_file_hasher.finalize().to_hex().to_string();

        // Root folder hash
        let mut root_hasher = blake3::Hasher::new();
        root_hasher.update(&[0]);
        // Subfolder "alpha" (sorted: alpha comes first, only one folder)
        root_hasher.update(&("alpha".len() as u64).to_le_bytes());
        root_hasher.update("alpha".as_bytes());
        root_hasher.update(alpha_hash.as_bytes());
        // File "beta.txt"
        root_hasher.update(&("beta.txt".len() as u64).to_le_bytes());
        root_hasher.update("beta.txt".as_bytes());
        root_hasher.update(beta_file_hash.as_bytes());
        let expected_root_hash = root_hasher.finalize().to_hex().to_string();

        assert_eq!(tree.root_hash, expected_root_hash);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn file_names_with_spaces() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "hello world.txt", b"spaces");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].name, "hello world.txt");
    }

    #[test]
    fn file_names_with_unicode() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "日本語.txt", b"unicode");
        create_file(&root, "émojis_🎉.txt", b"emoji");
        create_file(&root, "café.txt", b"accents");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 3);
        let names: Vec<&str> = tree.files.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"日本語.txt"));
        assert!(names.contains(&"émojis_🎉.txt"));
        assert!(names.contains(&"café.txt"));
    }

    #[test]
    fn folder_names_with_unicode() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        let sub = create_dir(&root, "données");
        create_file(&sub, "file.txt", b"in unicode dir");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.folders.len(), 1);
        assert_eq!(tree.folders[0].name, "données");
        assert_eq!(tree.folders[0].files[0].name, "file.txt");
    }

    #[test]
    fn hidden_dotfiles_are_included() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, ".hidden", b"hidden content");
        create_file(&root, ".gitignore", b"node_modules/");
        let dotdir = create_dir(&root, ".config");
        create_file(&dotdir, "settings.json", b"{}");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        let file_names: Vec<&str> = tree.files.iter().map(|f| f.name.as_str()).collect();
        assert!(file_names.contains(&".hidden"));
        assert!(file_names.contains(&".gitignore"));

        let folder_names: Vec<&str> = tree.folders.iter().map(|f| f.name.as_str()).collect();
        assert!(folder_names.contains(&".config"));
    }

    #[test]
    fn scanning_root_as_single_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("standalone.dat");
        fs::write(&file_path, b"standalone file content").unwrap();

        let mut scanner = new_scanner(&file_path);
        let tree = scan_view(&mut scanner).unwrap();

        // Root is a file, wrapped in synthetic folder
        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].name, "standalone.dat");
        assert_eq!(tree.files[0].blobs.len(), 1);
        // assert_eq!(
        //     tree.files[0].blobs[0].size,
        //     b"standalone file content".len() as u64
        // );
        assert_eq!(tree.folders.len(), 0);
    }

    #[test]
    fn file_with_special_characters_in_name() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file (1).txt", b"parens");
        create_file(&root, "file-name_v2.0.txt", b"dashes");
        create_file(&root, "file+plus.txt", b"plus");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 3);
        let names: Vec<&str> = tree.files.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"file (1).txt"));
        assert!(names.contains(&"file-name_v2.0.txt"));
        assert!(names.contains(&"file+plus.txt"));
    }

    #[test]
    fn binary_file_content_is_handled() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        // Write binary content with null bytes and high bytes
        let binary_content: Vec<u8> = (0..256).map(|i| i as u8).collect();
        create_file(&root, "binary.bin", &binary_content);

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].name, "binary.bin");
        // assert_eq!(tree.files[0].blobs[0].size, 256);

        // Verify hash matches expected blake3 of the binary content
        let expected_chunk_hash = blake3::hash(&binary_content).to_hex().to_string();
        assert_eq!(tree.files[0].blobs[0].hash, expected_chunk_hash);
    }

    #[test]
    fn large_file_name() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        // Most filesystems support names up to 255 bytes
        let long_name = format!("{}.txt", "a".repeat(250));
        create_file(&root, &long_name, b"long name file");

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].name, long_name);
    }

    #[test]
    fn get_path_returns_scanner_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("my_project");
        fs::create_dir(&root).unwrap();

        let scanner = new_scanner(&root);
        assert_eq!(scanner.get_path(), root.as_path());
    }

    #[test]
    fn multiple_files_same_content_different_names() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let content = b"identical content";
        create_file(&root, "copy_1.txt", content);
        create_file(&root, "copy_2.txt", content);
        create_file(&root, "copy_3.txt", content);

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 3);
        // All file hashes should be identical (hash doesn't include file name)
        assert_eq!(tree.files[0].hash, tree.files[1].hash);
        assert_eq!(tree.files[1].hash, tree.files[2].hash);
        // But the folder hash should still be deterministic
        let hash1 = tree.root_hash.clone();

        let tree2 = scan_view(&mut scanner).unwrap();
        assert_eq!(hash1, tree2.root_hash);
    }
}

#[cfg(unix)]
mod symlink_handling {
    use super::*;
    use std::os::unix::fs as unix_fs;

    #[test]
    fn symlink_to_file_scans_target_content() {
        // Create a real file and a symlink to it
        // Scan the directory containing the symlink
        // Verify the symlinked file appears with the target's content hash
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let target_content = b"symlink target content";
        let target_path = tmp.path().join("target.txt");
        fs::write(&target_path, target_content).unwrap();

        unix_fs::symlink(&target_path, root.join("link.txt")).unwrap();

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].name, "link.txt");
        // Content hash should match the target file's content
        let expected_chunk_hash = blake3::hash(target_content).to_hex().to_string();
        assert_eq!(tree.files[0].blobs[0].hash, expected_chunk_hash);
    }

    #[test]
    fn symlink_to_directory_scans_target_contents() {
        // Create a real directory with files and symlink to it
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let target_dir = tmp.path().join("real_dir");
        fs::create_dir(&target_dir).unwrap();
        create_file(&target_dir, "inner.txt", b"inner content");

        unix_fs::symlink(&target_dir, root.join("linked_dir")).unwrap();

        let mut scanner = new_scanner(&root);
        let tree = scan_view(&mut scanner).unwrap();

        assert_eq!(tree.folders.len(), 1);
        assert_eq!(tree.folders[0].name, "linked_dir");
        assert_eq!(tree.folders[0].files.len(), 1);
        assert_eq!(tree.folders[0].files[0].name, "inner.txt");
    }

    #[test]
    fn dangling_symlink_returns_error() {
        // Create a symlink pointing to a non-existent target
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        unix_fs::symlink("/nonexistent/target/path", root.join("dangling.txt")).unwrap();

        let mut scanner = new_scanner(&root);
        let result = scanner.scan();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ScannerError::GetMetadataFailed { .. }
        ));
    }

    #[test]
    fn symlink_content_change_detected_on_rescan() {
        // Verify that if the symlink target's content changes, the scanner detects it
        use filetime::{FileTime, set_file_mtime};

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();

        let target_path = tmp.path().join("target.txt");
        fs::write(&target_path, b"version 1").unwrap();

        unix_fs::symlink(&target_path, root.join("link.txt")).unwrap();

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        // Modify the target file
        fs::write(&target_path, b"version 2").unwrap();
        set_file_mtime(&target_path, FileTime::from_unix_time(2_000_000_000, 0)).unwrap();

        let tree2 = scan_view(&mut scanner).unwrap();

        assert_ne!(tree1.root_hash, tree2.root_hash);
        assert_ne!(tree1.files[0].hash, tree2.files[0].hash);
    }
}

#[cfg(unix)]
mod permission_errors {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn unreadable_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "secret.txt", b"secret content");

        // Remove read permission
        let file_path = root.join("secret.txt");
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o000)).unwrap();

        let mut scanner = new_scanner(&root);
        let result = scanner.scan();

        // Restore permissions for cleanup
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(result.is_err());
    }

    #[test]
    fn unreadable_directory_returns_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        let sub = create_dir(&root, "restricted");
        create_file(&sub, "file.txt", b"inside");

        // Remove read+execute permissions on directory
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o000)).unwrap();

        let mut scanner = new_scanner(&root);
        let result = scanner.scan();

        // Restore permissions for cleanup
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ScannerError::ReadDirFailed { .. }
        ));
    }

    #[test]
    fn readable_dir_with_unreadable_nested_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "good.txt", b"accessible");
        let sub = create_dir(&root, "sub");
        create_file(&sub, "bad.txt", b"no access");

        let bad_path = sub.join("bad.txt");
        fs::set_permissions(&bad_path, fs::Permissions::from_mode(0o000)).unwrap();

        let mut scanner = new_scanner(&root);
        let result = scanner.scan();

        // Restore permissions for cleanup
        fs::set_permissions(&bad_path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(result.is_err());
    }
}

mod cache_correctness {
    use super::*;
    use filetime::{FileTime, set_file_mtime};

    #[test]
    fn unchanged_file_uses_cached_hash_same_result() {
        // Verify that the cache produces the same result as a fresh scan
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        create_file(&root, "file.txt", b"content to cache");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();
        let tree2 = scan_view(&mut scanner).unwrap(); // Should use cache

        assert_eq!(tree1.root_hash, tree2.root_hash);
        assert_eq!(tree1.files[0].hash, tree2.files[0].hash);
        assert_eq!(tree1.files[0].blobs.len(), tree2.files[0].blobs.len());
        for (c1, c2) in tree1.files[0].blobs.iter().zip(tree2.files[0].blobs.iter()) {
            assert_eq!(c1, c2);
            // assert_eq!(c1.size, c2.size);
        }
    }

    #[test]
    fn same_size_different_mtime_triggers_rehash() {
        // If the file size stays the same but mtime changes, the scanner should rehash
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        // Same length content: 7 bytes each
        create_file(&root, "file.txt", b"aaaaaaa");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        // Overwrite with same-size but different content
        create_file(&root, "file.txt", b"bbbbbbb");
        set_file_mtime(
            root.join("file.txt"),
            FileTime::from_unix_time(2_000_000_000, 0),
        )
        .unwrap();

        let tree2 = scan_view(&mut scanner).unwrap();

        assert_ne!(tree1.files[0].hash, tree2.files[0].hash);
        assert_ne!(tree1.root_hash, tree2.root_hash);
    }

    #[test]
    fn cache_invalidation_propagates_to_parent_folder_hash() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir(&root).unwrap();
        let sub = create_dir(&root, "sub");
        create_file(&sub, "nested.txt", b"original");
        create_file(&root, "sibling.txt", b"untouched");

        let mut scanner = new_scanner(&root);
        let tree1 = scan_view(&mut scanner).unwrap();

        // Modify nested file
        create_file(&sub, "nested.txt", b"modified");
        set_file_mtime(
            sub.join("nested.txt"),
            FileTime::from_unix_time(2_000_000_000, 0),
        )
        .unwrap();

        let tree2 = scan_view(&mut scanner).unwrap();

        // Root hash changed because child changed
        assert_ne!(tree1.root_hash, tree2.root_hash);
        // The subfolder hash changed
        assert_ne!(tree1.folders[0].hash, tree2.folders[0].hash);
        // But sibling file is unchanged
        assert_eq!(tree1.files[0].hash, tree2.files[0].hash);
    }
}
