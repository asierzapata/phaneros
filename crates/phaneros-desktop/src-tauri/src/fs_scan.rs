use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::format::{format_bytes, format_relative_time};

/// The desktop app shows a vault's file tree straight off the local
/// filesystem (what actually got synced/materialized to disk) rather than
/// the remote Merkle tree `phaneros-core` uses internally for diffing — it's
/// simpler, doesn't need the daemon at all, and matches what a user
/// browsing their synced folder actually expects to see.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNodeDto {
    pub id: String,
    pub name: String,
    pub ext: String,
    pub is_dir: bool,
    pub size: Option<String>,
    pub modified: Option<String>,
    pub children: Option<Vec<FileNodeDto>>,
    pub badge: Option<String>,
}

pub fn build_file_tree(root: &Path) -> Vec<FileNodeDto> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    let mut nodes: Vec<FileNodeDto> = entries
        .flatten()
        .filter_map(|entry| build_node(&entry.path()))
        .collect();

    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    nodes
}

fn build_node(path: &Path) -> Option<FileNodeDto> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    let file_name = path.file_name()?.to_string_lossy().to_string();
    if file_name.starts_with('.') {
        return None;
    }

    let is_dir = metadata.is_dir();
    let (name, ext) = split_name_ext(&file_name, is_dir);
    let modified = metadata
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| format_relative_time(Some(d.as_secs())));

    Some(FileNodeDto {
        id: path.to_string_lossy().to_string(),
        badge: if is_dir { None } else { Some(ext.to_uppercase()) },
        name,
        ext,
        is_dir,
        size: if is_dir {
            None
        } else {
            Some(format_bytes(metadata.len()))
        },
        modified,
        children: if is_dir {
            Some(build_file_tree(path))
        } else {
            None
        },
    })
}

fn split_name_ext(file_name: &str, is_dir: bool) -> (String, String) {
    if is_dir {
        return (file_name.to_string(), String::new());
    }
    match file_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), ext.to_string()),
        _ => (file_name.to_string(), String::new()),
    }
}
