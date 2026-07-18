use crate::{blob::BlobRef, hash::Hash};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    Folder {
        folders: Vec<Entry>,
        files: Vec<Entry>,
    },
    File {
        blobs: Vec<BlobRef>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeWire {
    Folder {
        folders: Vec<Entry>,
        files: Vec<Entry>,
    },
    File {
        blobs: Vec<BlobRef>,
    },
}

impl NodeWire {
    pub fn reconstruct(self) -> (Hash, Node) {
        match self {
            NodeWire::Folder { folders, files } => Node::folder(folders, files),
            NodeWire::File { blobs } => Node::file(blobs),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub hash: Hash,
}

impl Entry {
    pub fn new(name: impl Into<String>, hash: impl Into<Hash>) -> Self {
        Entry {
            name: name.into(),
            hash: hash.into(),
        }
    }
}

impl Node {
    pub fn folder(mut folders: Vec<Entry>, mut files: Vec<Entry>) -> (Hash, Node) {
        folders.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        let mut hasher = blake3::Hasher::new();

        // We add a first byte to the hash to differentiate between files and folders
        // so an empty folder and an empty file don't have the same hash.
        hasher.update(&[0]);

        for folder in &folders {
            hasher.update(&(folder.name.len() as u64).to_le_bytes());
            hasher.update(folder.name.as_bytes());
            hasher.update(folder.hash.as_bytes());
        }

        for file in &files {
            hasher.update(&(file.name.len() as u64).to_le_bytes());
            hasher.update(file.name.as_bytes());
            hasher.update(file.hash.as_bytes());
        }

        let hash = hasher.finalize().to_hex().to_string();

        (hash, Node::Folder { folders, files })
    }

    pub fn file(blobs: Vec<BlobRef>) -> (Hash, Node) {
        let mut hasher = blake3::Hasher::new();

        // We add a first byte to the hash to differentiate between files and folders
        // so an empty folder and an empty file don't have the same hash.
        hasher.update(&[1]);

        for blob in &blobs {
            hasher.update(blob.hash.as_bytes());
        }

        let hash = hasher.finalize().to_hex().to_string();

        (hash, Node::File { blobs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A folder built from deliberately unsorted entries round-trips through the wire.
    // `h1 == h2` proves the Serialize/Deserialize shapes agree (no drift); `node == node2`
    // proves the value survives intact, including canonical ordering.
    #[test]
    fn folder_round_trips_through_wire() {
        let (h1, node) = Node::folder(
            vec![Entry::new("gamma", "hg"), Entry::new("alpha", "ha")],
            vec![Entry::new("z.txt", "hz"), Entry::new("a.txt", "ha2")],
        );

        let json = serde_json::to_string(&node).unwrap();
        let (h2, node2) = serde_json::from_str::<NodeWire>(&json)
            .unwrap()
            .reconstruct();

        assert_eq!(h1, h2);
        assert_eq!(node, node2);
    }

    #[test]
    fn file_round_trips_through_wire() {
        let (h1, node) = Node::file(vec![
            BlobRef::from_bytes(b"hello"),
            BlobRef::from_bytes(b"world"),
        ]);

        let json = serde_json::to_string(&node).unwrap();
        let (h2, node2) = serde_json::from_str::<NodeWire>(&json)
            .unwrap()
            .reconstruct();

        assert_eq!(h1, h2);
        assert_eq!(node, node2);
    }

    // The emitted JSON must match the spec's field/tag names exactly.
    #[test]
    fn serialized_shape_matches_spec() {
        let (_, folder) = Node::folder(vec![Entry::new("sub", "abc")], vec![]);
        let v: serde_json::Value = serde_json::to_value(&folder).unwrap();
        assert_eq!(v["type"], "folder");
        assert!(v["folders"].is_array());
        assert!(v["files"].is_array());

        let (_, file) = Node::file(vec![BlobRef::from_bytes(b"x")]);
        let v: serde_json::Value = serde_json::to_value(&file).unwrap();
        assert_eq!(v["type"], "file");
        assert_eq!(v["blobs"][0]["size"], 1);
        assert!(v["blobs"][0]["hash"].is_string());
    }

    // The blind spot a pure round-trip misses: a hostile/buggy client can send arrays
    // out of canonical order. Reconstructing through the constructor must normalize them,
    // so the hash matches the canonical one regardless of wire order.
    #[test]
    fn reconstruct_normalizes_unsorted_wire_order() {
        let (canonical, _) = Node::folder(
            vec![Entry::new("alpha", "ha"), Entry::new("beta", "hb")],
            vec![],
        );

        let wire_json = r#"{"type":"folder","folders":[{"name":"beta","hash":"hb"},{"name":"alpha","hash":"ha"}],"files":[]}"#;
        let (reconstructed, _) = serde_json::from_str::<NodeWire>(wire_json)
            .unwrap()
            .reconstruct();

        assert_eq!(canonical, reconstructed);
    }
}
