use std::path::Path;

use serde::Serialize;
use similar::{ChangeTag, TextDiff};

use crate::format::{format_bytes, format_relative_time};

/// `phaneros-core`'s merge logic (`syncer::merge`) already resolves conflicts
/// it can't auto-merge by leaving both sides on disk: the original filename
/// keeps one side, and the other is written alongside as `{name}.conflict`
/// (modify/modify) or `{name}.conflict-delete` (delete/modify), with `.N`
/// suffixes on repeat collisions. Everything here is a client-side scan of
/// that filesystem convention — no daemon/core involvement.
const MODIFY_SUFFIX: &str = ".conflict";
const DELETE_SUFFIX: &str = ".conflict-delete";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSummaryDto {
    pub id: String,
    pub filename: String,
    pub is_binary: bool,
    pub conflict_kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffLineDto {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffChunkDto {
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<CodeDiffLineDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDiffDto {
    pub filename: String,
    pub path: String,
    pub lines_added: u32,
    pub lines_removed: u32,
    pub chunks: Vec<CodeDiffChunkDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSideDto {
    pub size: String,
    pub modified: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryMetadataDiffDto {
    pub filename: String,
    pub path: String,
    pub is_binary: bool,
    pub local: FileSideDto,
    pub store: FileSideDto,
    pub recommended_action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ConflictDiffDto {
    Text { diff: CodeDiffDto },
    Binary { diff: BinaryMetadataDiffDto },
}

/// Strips a trailing `.conflict`/`.conflict-delete` (optionally followed by
/// `.N` for repeat collisions) off a file name, matching
/// `syncer::merge::reserve_suffixed_name`'s naming scheme.
fn strip_conflict_suffix(file_name: &str) -> Option<(String, bool)> {
    for (marker, is_delete) in [(DELETE_SUFFIX, true), (MODIFY_SUFFIX, false)] {
        let Some(idx) = file_name.rfind(marker) else { continue };
        let after = &file_name[idx + marker.len()..];
        let valid_tail = after.is_empty()
            || (after.starts_with('.')
                && after.len() > 1
                && after[1..].chars().all(|c| c.is_ascii_digit()));
        if valid_tail {
            return Some((file_name[..idx].to_string(), is_delete));
        }
    }
    None
}

fn is_binary_file(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return true;
    };
    let sample = &bytes[..bytes.len().min(8192)];
    sample.contains(&0) || std::str::from_utf8(sample).is_err()
}

pub fn scan(root: &Path) -> Vec<ConflictSummaryDto> {
    let mut results = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some((original_name, is_delete)) = strip_conflict_suffix(file_name) else {
                continue;
            };

            let is_binary = if is_delete {
                is_binary_file(&path)
            } else {
                let original_path = path.with_file_name(&original_name);
                is_binary_file(&path) || (original_path.exists() && is_binary_file(&original_path))
            };

            results.push(ConflictSummaryDto {
                id: path.to_string_lossy().to_string(),
                filename: original_name,
                is_binary,
                conflict_kind: if is_delete { "delete" } else { "modify" }.to_string(),
            });
        }
    }

    results
}

fn file_side(path: &Path) -> FileSideDto {
    let metadata = std::fs::metadata(path).ok();
    let size = metadata
        .as_ref()
        .map(|m| format_bytes(m.len()))
        .unwrap_or_else(|| "Unknown".to_string());
    let modified = metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| format_relative_time(Some(d.as_secs())))
        .unwrap_or_else(|| "Unknown".to_string());
    let hash = std::fs::read(path)
        .map(|bytes| blake3::hash(&bytes).to_hex()[..16].to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    FileSideDto { size, modified, hash }
}

fn deleted_side() -> FileSideDto {
    FileSideDto {
        size: "Deleted".to_string(),
        modified: "\u{2014}".to_string(),
        hash: "\u{2014}".to_string(),
    }
}

fn text_diff(original_name: &str, local_path: &Path, store_path: &Path) -> Result<CodeDiffDto, String> {
    let local_text = std::fs::read_to_string(local_path).map_err(|e| e.to_string())?;
    let store_text = std::fs::read_to_string(store_path).map_err(|e| e.to_string())?;

    let diff = TextDiff::from_lines(&local_text, &store_text);
    let mut lines = Vec::new();
    let mut lines_added = 0u32;
    let mut lines_removed = 0u32;

    for change in diff.iter_all_changes() {
        let text = change.value().trim_end_matches('\n').to_string();
        let (kind, text) = match change.tag() {
            ChangeTag::Equal => ("same", text),
            ChangeTag::Delete => {
                lines_removed += 1;
                ("delete", format!("- {}", text))
            }
            ChangeTag::Insert => {
                lines_added += 1;
                ("add", format!("+ {}", text))
            }
        };
        lines.push(CodeDiffLineDto { kind, text });
    }

    Ok(CodeDiffDto {
        filename: original_name.to_string(),
        path: local_path.to_string_lossy().to_string(),
        lines_added,
        lines_removed,
        chunks: vec![CodeDiffChunkDto {
            old_start: 1,
            new_start: 1,
            lines,
        }],
    })
}

pub fn diff(conflict_path: &Path) -> Result<ConflictDiffDto, String> {
    let file_name = conflict_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid conflict path".to_string())?;
    let (original_name, is_delete) = strip_conflict_suffix(file_name)
        .ok_or_else(|| format!("'{}' is not a conflict file", file_name))?;
    let original_path = conflict_path.with_file_name(&original_name);

    if is_delete {
        return Ok(ConflictDiffDto::Binary {
            diff: BinaryMetadataDiffDto {
                filename: original_name,
                path: conflict_path.to_string_lossy().to_string(),
                is_binary: true,
                local: file_side(conflict_path),
                store: deleted_side(),
                recommended_action: "Keep Local",
            },
        });
    }

    if !original_path.exists() {
        return Err(format!(
            "original file '{}' is missing; cannot diff",
            original_name
        ));
    }

    if is_binary_file(&original_path) || is_binary_file(conflict_path) {
        let local = file_side(&original_path);
        let store = file_side(conflict_path);
        let recommended_action = if local.modified == store.modified {
            "Keep Local"
        } else {
            "Keep Store"
        };
        return Ok(ConflictDiffDto::Binary {
            diff: BinaryMetadataDiffDto {
                filename: original_name,
                path: original_path.to_string_lossy().to_string(),
                is_binary: true,
                local,
                store,
                recommended_action,
            },
        });
    }

    let diff = text_diff(&original_name, &original_path, conflict_path)?;
    Ok(ConflictDiffDto::Text { diff })
}

pub fn resolve(conflict_path: &Path, keep_local: bool) -> Result<(), String> {
    let file_name = conflict_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "invalid conflict path".to_string())?;
    let (original_name, is_delete) = strip_conflict_suffix(file_name)
        .ok_or_else(|| format!("'{}' is not a conflict file", file_name))?;
    let original_path = conflict_path.with_file_name(&original_name);

    if is_delete {
        if keep_local {
            std::fs::rename(conflict_path, &original_path).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(conflict_path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    if keep_local {
        std::fs::remove_file(conflict_path).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(conflict_path, &original_path).map_err(|e| e.to_string())?;
        std::fs::remove_file(conflict_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scan_finds_modify_conflict_pair() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "local content").unwrap();
        std::fs::write(dir.path().join("README.md.conflict"), "store content").unwrap();

        let results = scan(dir.path());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "README.md");
        assert_eq!(results[0].conflict_kind, "modify");
        assert!(!results[0].is_binary);
    }

    #[test]
    fn scan_finds_delete_conflict() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt.conflict-delete"), "edited content").unwrap();

        let results = scan(dir.path());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "notes.txt");
        assert_eq!(results[0].conflict_kind, "delete");
    }

    #[test]
    fn scan_ignores_regular_files() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("plain.txt"), "content").unwrap();

        assert!(scan(dir.path()).is_empty());
    }

    #[test]
    fn scan_recurses_into_subdirectories() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("nested");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.txt"), "local").unwrap();
        std::fs::write(sub.join("a.txt.conflict"), "remote").unwrap();

        let results = scan(dir.path());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "a.txt");
    }

    #[test]
    fn diff_text_modify_conflict_reports_line_changes() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("notes.md"), "line one\nline two\n").unwrap();
        let conflict_path = dir.path().join("notes.md.conflict");
        std::fs::write(&conflict_path, "line one\nline two changed\n").unwrap();

        let result = diff(&conflict_path).unwrap();
        match result {
            ConflictDiffDto::Text { diff } => {
                assert_eq!(diff.filename, "notes.md");
                assert_eq!(diff.lines_removed, 1);
                assert_eq!(diff.lines_added, 1);
            }
            ConflictDiffDto::Binary { .. } => panic!("expected a text diff"),
        }
    }

    #[test]
    fn diff_binary_modify_conflict_returns_metadata_comparison() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("db.sqlite"), [0u8, 1, 2, 3]).unwrap();
        let conflict_path = dir.path().join("db.sqlite.conflict");
        std::fs::write(&conflict_path, [4u8, 5, 6, 7]).unwrap();

        let result = diff(&conflict_path).unwrap();
        match result {
            ConflictDiffDto::Binary { diff } => {
                assert_eq!(diff.filename, "db.sqlite");
                assert_ne!(diff.local.hash, diff.store.hash);
            }
            ConflictDiffDto::Text { .. } => panic!("expected a binary diff"),
        }
    }

    #[test]
    fn diff_delete_conflict_shows_deleted_other_side() {
        let dir = tempdir().unwrap();
        let conflict_path = dir.path().join("gone.txt.conflict-delete");
        std::fs::write(&conflict_path, "surviving edit").unwrap();

        let result = diff(&conflict_path).unwrap();
        match result {
            ConflictDiffDto::Binary { diff } => {
                assert_eq!(diff.filename, "gone.txt");
                assert_eq!(diff.store.size, "Deleted");
            }
            ConflictDiffDto::Text { .. } => panic!("expected a binary-shaped diff"),
        }
    }

    #[test]
    fn resolve_modify_keep_local_drops_conflict_file() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("f.txt");
        let conflict_path = dir.path().join("f.txt.conflict");
        std::fs::write(&original, "local").unwrap();
        std::fs::write(&conflict_path, "store").unwrap();

        resolve(&conflict_path, true).unwrap();

        assert!(original.exists());
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "local");
        assert!(!conflict_path.exists());
    }

    #[test]
    fn resolve_modify_keep_store_overwrites_original() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("f.txt");
        let conflict_path = dir.path().join("f.txt.conflict");
        std::fs::write(&original, "local").unwrap();
        std::fs::write(&conflict_path, "store").unwrap();

        resolve(&conflict_path, false).unwrap();

        assert_eq!(std::fs::read_to_string(&original).unwrap(), "store");
        assert!(!conflict_path.exists());
    }

    #[test]
    fn resolve_delete_keep_local_restores_original_name() {
        let dir = tempdir().unwrap();
        let conflict_path = dir.path().join("gone.txt.conflict-delete");
        std::fs::write(&conflict_path, "surviving edit").unwrap();

        resolve(&conflict_path, true).unwrap();

        let original = dir.path().join("gone.txt");
        assert!(original.exists());
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "surviving edit");
        assert!(!conflict_path.exists());
    }

    #[test]
    fn resolve_delete_keep_store_removes_conflict_file() {
        let dir = tempdir().unwrap();
        let conflict_path = dir.path().join("gone.txt.conflict-delete");
        std::fs::write(&conflict_path, "surviving edit").unwrap();

        resolve(&conflict_path, false).unwrap();

        assert!(!conflict_path.exists());
        assert!(!dir.path().join("gone.txt").exists());
    }
}
