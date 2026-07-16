use crate::blob_store::blob::BlobRef;
use crate::node_store::{Entry, Hash, InMemoryNodeStore, Node, NodeStore};
#[cfg(test)]
mod tests {

    use super::*;

    fn chunk(bytes: &[u8]) -> BlobRef {
        BlobRef::from_bytes(bytes)
    }

    // These tests pin the hash contract by recomputing expected hashes by hand
    // ([1] + chunk hashes for files, [0] + len-prefixed names + hashes for
    // folders). Any change to the constructors that alters produced hashes —
    // and would therefore invalidate every hash already stored — fails here.

    fn expected_file_hash(blobs: &[BlobRef]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[1]);
        for blob in blobs {
            hasher.update(blob.hash.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    fn expected_folder_hash(folders: &[Entry], files: &[Entry]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0]);
        for entry in folders.iter().chain(files.iter()) {
            hasher.update(&(entry.name.len() as u64).to_le_bytes());
            hasher.update(entry.name.as_bytes());
            hasher.update(entry.hash.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    #[test]
    fn file_hash_matches_contract() {
        let chunks = vec![chunk(b"hello"), chunk(b"world")];

        let (hash, _) = Node::file(chunks.clone());

        assert_eq!(hash, expected_file_hash(&chunks));
    }

    #[test]
    fn empty_file_hash_matches_contract() {
        let (hash, _) = Node::file(vec![]);

        assert_eq!(hash, expected_file_hash(&[]));
    }

    #[test]
    fn folder_hash_matches_contract() {
        let (file_a_hash, _) = Node::file(vec![chunk(b"aaa")]);
        let (file_b_hash, _) = Node::file(vec![chunk(b"bbb")]);
        let (subfolder_hash, _) = Node::folder(vec![], vec![Entry::new("a.txt", &file_a_hash)]);

        let folders = vec![Entry::new("sub", &subfolder_hash)];
        let files = vec![
            Entry::new("a.txt", &file_a_hash),
            Entry::new("b.txt", &file_b_hash),
        ];

        let (hash, _) = Node::folder(folders.clone(), files.clone());

        assert_eq!(hash, expected_folder_hash(&folders, &files));
    }

    #[test]
    fn empty_folder_hash_matches_contract() {
        let (hash, _) = Node::folder(vec![], vec![]);

        assert_eq!(hash, expected_folder_hash(&[], &[]));
    }

    #[test]
    fn empty_folder_and_empty_file_hashes_differ() {
        let (folder_hash, _) = Node::folder(vec![], vec![]);
        let (file_hash, _) = Node::file(vec![]);

        assert_ne!(folder_hash, file_hash);
    }

    #[test]
    fn folder_hash_is_canonical_regardless_of_entry_order() {
        let a = Entry::new("a.txt", "hash-a");
        let b = Entry::new("b.txt", "hash-b");

        let (sorted_hash, _) = Node::folder(vec![], vec![a.clone(), b.clone()]);
        let (unsorted_hash, node) = Node::folder(vec![], vec![b, a]);

        assert_eq!(sorted_hash, unsorted_hash);
        match node {
            Node::Folder { files, .. } => {
                assert_eq!(files[0].name, "a.txt");
                assert_eq!(files[1].name, "b.txt");
            }
            _ => panic!("expected folder node"),
        }
    }

    #[test]
    fn folder_hash_depends_on_child_names() {
        let (file_hash, _) = Node::file(vec![chunk(b"same content")]);

        let (folder_a_hash, _) = Node::folder(vec![], vec![Entry::new("original.txt", &file_hash)]);
        let (folder_b_hash, _) = Node::folder(vec![], vec![Entry::new("renamed.txt", &file_hash)]);

        assert_ne!(folder_a_hash, folder_b_hash);
    }

    #[test]
    fn store_deduplicates_identical_nodes() {
        let mut store = InMemoryNodeStore::new();

        let (hash_a, node_a) = Node::file(vec![chunk(b"same")]);
        let (hash_b, node_b) = Node::file(vec![chunk(b"same")]);
        assert_eq!(hash_a, hash_b);

        store.insert(hash_a.clone(), node_a);
        store.insert(hash_b, node_b);

        assert_eq!(store.len(), 1);
        assert!(store.get_node(&hash_a).is_some());
    }

    #[test]
    fn store_returns_root_and_nodes_by_hash() {
        let mut store = InMemoryNodeStore::new();

        let (file_hash, file_node) = Node::file(vec![chunk(b"content")]);
        let (root_hash, root_node) = Node::folder(vec![], vec![Entry::new("file.txt", &file_hash)]);

        store.insert(file_hash.clone(), file_node);
        store.insert(root_hash.clone(), root_node);
        store.set_root(root_hash.clone());

        assert_eq!(store.root_hash(), Some(&root_hash));

        let root = store.get_node(&root_hash).expect("root node present");
        match root {
            Node::Folder { files, .. } => assert_eq!(files[0].hash, file_hash),
            _ => panic!("expected folder node"),
        }
    }
}
