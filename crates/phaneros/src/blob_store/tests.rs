use crate::blob_store::{Blob, BlobRef, BlobStore, InMemoryBlobStore};

// ---- fixture helpers -------------------------------------------------------

/// Inserts `bytes` under their own content hash and returns the tree-side ref.
/// This mirrors how the scanner will feed the store: the key is always derived
/// from the content, never chosen by the caller.
fn insert_bytes(store: &mut InMemoryBlobStore, bytes: &[u8]) -> BlobRef {
    let blob_ref = BlobRef::from_bytes(bytes);
    store.insert(
        blob_ref.hash.clone(),
        Blob {
            bytes: bytes.to_vec(),
        },
    );
    blob_ref
}

// ---- the fundamental contract ----------------------------------------------

mod round_trip {
    use super::*;

    #[test]
    fn stored_bytes_come_back_identical() {
        // The one property a blob store cannot exist without: what you put in
        // is what you get out. Everything else is optimization.
        let mut store = InMemoryBlobStore::new();
        let blob_ref = insert_bytes(&mut store, b"cat-bytes");

        let stored = store.get_blob(&blob_ref.hash).expect("blob should exist");

        assert_eq!(stored.bytes, b"cat-bytes");
    }

    #[test]
    fn stored_bytes_rehash_to_their_key() {
        // Integrity: the key must BE the content's hash, or the store is a
        // plain map wearing a content-addressed costume. A reader must be able
        // to verify any blob it receives by rehashing it.
        let mut store = InMemoryBlobStore::new();
        let blob_ref = insert_bytes(&mut store, b"verify me");

        let stored = store.get_blob(&blob_ref.hash).unwrap();
        let rehashed = blake3::hash(&stored.bytes).to_hex().to_string();

        assert_eq!(rehashed, blob_ref.hash);
    }

    #[test]
    fn ref_size_matches_stored_bytes() {
        // The tree-side BlobRef promises a size so the syncer can plan
        // transfers without fetching; that promise must match reality.
        let mut store = InMemoryBlobStore::new();
        let blob_ref = insert_bytes(&mut store, b"12 bytes long");

        let stored = store.get_blob(&blob_ref.hash).unwrap();

        assert_eq!(blob_ref.size, stored.bytes.len() as u64);
    }
}

// ---- lookup ------------------------------------------------------------------

mod lookup {
    use super::*;

    #[test]
    fn missing_hash_is_absent_everywhere() {
        let store = InMemoryBlobStore::new();
        let never_inserted = BlobRef::from_bytes(b"ghost");

        assert!(store.get_blob(&never_inserted.hash).is_none());
        assert!(!store.contains(&never_inserted.hash));
    }

    #[test]
    fn contains_agrees_with_get_blob() {
        // `contains` exists so a remote store can answer "do you have it?"
        // without shipping bytes. Cheap answer and expensive answer must never
        // disagree.
        let mut store = InMemoryBlobStore::new();
        let blob_ref = insert_bytes(&mut store, b"present");

        assert!(store.contains(&blob_ref.hash));
        assert_eq!(
            store.contains(&blob_ref.hash),
            store.get_blob(&blob_ref.hash).is_some()
        );
    }
}

// ---- content addressing -------------------------------------------------------

mod content_addressing {
    use super::*;

    #[test]
    fn identical_bytes_occupy_one_slot() {
        // Content addressing IS deduplication: two files with the same bytes
        // share one blob, no matter how many times the scanner meets them.
        let mut store = InMemoryBlobStore::new();
        let ref_a = insert_bytes(&mut store, b"identical");
        let ref_b = insert_bytes(&mut store, b"identical");

        assert_eq!(ref_a.hash, ref_b.hash);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn reinserting_a_hash_does_not_clobber_the_original() {
        // insert() uses or_insert: first write wins. For honest callers this
        // is idempotence (same hash implies same bytes). It also means a buggy
        // or malicious second write can't silently corrupt an existing blob.
        let mut store = InMemoryBlobStore::new();
        let blob_ref = insert_bytes(&mut store, b"original");

        store.insert(
            blob_ref.hash.clone(),
            Blob {
                bytes: b"impostor".to_vec(),
            },
        );

        let stored = store.get_blob(&blob_ref.hash).unwrap();
        assert_eq!(stored.bytes, b"original");
        assert_eq!(store.len(), 1);
    }
}
