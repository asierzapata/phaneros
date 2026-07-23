use std::path::Path;

use crate::{
    blob_repository::BlobRepository,
    node_repository::{Hash, Node, NodeRepository},
};

/// Debug util: writes a snapshot of a node/blob store pair to a text file —
/// the current root and the tree reachable from it, with per-file blob refs,
/// flagging any blob the blob store doesn't hold.
pub fn dump_store(
    node_repository: &impl NodeRepository,
    blob_repository: &impl BlobRepository,
    path: &Path,
) -> std::io::Result<()> {
    let mut out = String::new();
    match node_repository.root_hash() {
        Ok(Some(root)) => {
            out.push_str(&format!("root: {}\n\n", root));
            let root = root.clone();
            dump_tree(node_repository, blob_repository, &root, ".", 0, &mut out);
        }
        Ok(None) => out.push_str("root: (unset)\n"),
        Err(err) => out.push_str(&format!("root: (error: {})\n", err)),
    }
    std::fs::write(path, out)
}

fn dump_tree(
    node_repository: &impl NodeRepository,
    blob_repository: &impl BlobRepository,
    hash: &Hash,
    name: &str,
    depth: usize,
    out: &mut String,
) {
    let indent = "    ".repeat(depth);
    match node_repository.get_node(hash) {
        Ok(Some(Node::Folder { folders, files })) => {
            out.push_str(&format!("{}{}/  [{}]\n", indent, name, short_hash(hash)));
            for folder in folders {
                dump_tree(
                    node_repository,
                    blob_repository,
                    &folder.hash,
                    &folder.name,
                    depth + 1,
                    out,
                );
            }
            for file in files {
                dump_tree(
                    node_repository,
                    blob_repository,
                    &file.hash,
                    &file.name,
                    depth + 1,
                    out,
                );
            }
        }
        Ok(Some(Node::File { blobs })) => {
            out.push_str(&format!("{}{}  [{}]\n", indent, name, short_hash(hash)));
            for blob_ref in blobs {
                let missing = match blob_repository.contains(&blob_ref.hash) {
                    Ok(true) => "",
                    Ok(false) => "  <- MISSING BLOB",
                    Err(_) => "  <- blob check failed",
                };
                out.push_str(&format!(
                    "{}    blob [{}] {} bytes{}\n",
                    indent,
                    short_hash(&blob_ref.hash),
                    blob_ref.size,
                    missing,
                ));
            }
        }
        Ok(None) => out.push_str(&format!(
            "{}{}  [{}]  <- MISSING NODE\n",
            indent,
            name,
            short_hash(hash)
        )),
        Err(err) => out.push_str(&format!(
            "{}{}  [{}]  <- error: {}\n",
            indent,
            name,
            short_hash(hash),
            err
        )),
    }
}

fn short_hash(hash: &Hash) -> &str {
    &hash[..hash.len().min(12)]
}
