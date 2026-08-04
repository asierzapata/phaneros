use std::io;
use std::path::{Path, PathBuf};

pub const TEMP_SUFFIX: &str = ".phaneros-tmp";

pub const INTERNAL_DIR: &str = ".phaneros";

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let temp_path = temp_path_for(path);
    std::fs::write(&temp_path, bytes)?;
    std::fs::rename(&temp_path, path)?;

    Ok(())
}

pub fn temp_path_for(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    path.with_file_name(format!("{}{}", name, TEMP_SUFFIX))
}

pub fn is_internal_entry(name: &str) -> bool {
    name == INTERNAL_DIR || name.ends_with(TEMP_SUFFIX)
}

pub fn trash_batch_path(vault_path: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    vault_path
        .join(INTERNAL_DIR)
        .join("trash")
        .join(stamp.to_string())
}

pub fn move_to_trash(path: &Path, trash_batch: &Path, rel_path: &Path) -> io::Result<PathBuf> {
    let destination = free_destination(&trash_batch.join(rel_path));

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(path, &destination)?;

    Ok(destination)
}

fn free_destination(candidate: &Path) -> PathBuf {
    if !candidate.exists() {
        return candidate.to_path_buf();
    }

    let name = candidate
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut index = 1usize;
    loop {
        let next = candidate.with_file_name(format!("{}-{}", name, index));
        if !next.exists() {
            return next;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn write_atomic_creates_missing_parents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/deeper/notes.txt");

        write_atomic(&path, b"hello").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn write_atomic_replaces_existing_content_and_leaves_no_temp_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"old").unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(!temp_path_for(&path).exists());
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(entries, vec!["notes.txt".to_string()]);
    }

    #[tokio::test]
    async fn temp_path_appends_instead_of_replacing_the_extension() {
        // Replacing the extension would collapse these two onto one temp path.
        assert_eq!(
            temp_path_for(Path::new("/vault/notes.tar.gz")),
            PathBuf::from("/vault/notes.tar.gz.phaneros-tmp")
        );
        assert_eq!(
            temp_path_for(Path::new("/vault/notes.tar.bz2")),
            PathBuf::from("/vault/notes.tar.bz2.phaneros-tmp")
        );
    }

    #[tokio::test]
    async fn internal_entries_are_the_state_dir_and_temp_files() {
        assert!(is_internal_entry(".phaneros"));
        assert!(is_internal_entry("notes.txt.phaneros-tmp"));

        assert!(!is_internal_entry("notes.txt"));
        assert!(!is_internal_entry(".phaneros-notes"));
        assert!(!is_internal_entry("phaneros"));
    }

    #[tokio::test]
    async fn move_to_trash_preserves_the_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        let trash_batch = vault.join(".phaneros/trash/1");
        let path = vault.join("docs/hello.txt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"hello").unwrap();

        let destination = move_to_trash(&path, &trash_batch, Path::new("docs/hello.txt")).unwrap();

        assert_eq!(destination, trash_batch.join("docs/hello.txt"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"hello");
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn move_to_trash_moves_directories_whole() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        let trash_batch = vault.join(".phaneros/trash/1");
        let folder = vault.join("docs");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("hello.txt"), b"hello").unwrap();

        let destination = move_to_trash(&folder, &trash_batch, Path::new("docs")).unwrap();

        assert_eq!(
            std::fs::read(destination.join("hello.txt")).unwrap(),
            b"hello"
        );
        assert!(!folder.exists());
    }

    #[tokio::test]
    async fn move_to_trash_suffixes_a_name_already_taken_in_the_batch() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault");
        let trash_batch = vault.join(".phaneros/trash/1");
        let path = vault.join("hello.txt");
        std::fs::create_dir_all(&vault).unwrap();

        std::fs::write(&path, b"first").unwrap();
        move_to_trash(&path, &trash_batch, Path::new("hello.txt")).unwrap();

        std::fs::write(&path, b"second").unwrap();
        let destination = move_to_trash(&path, &trash_batch, Path::new("hello.txt")).unwrap();

        assert_eq!(destination, trash_batch.join("hello.txt-1"));
        assert_eq!(std::fs::read(&destination).unwrap(), b"second");
        assert_eq!(
            std::fs::read(trash_batch.join("hello.txt")).unwrap(),
            b"first"
        );
    }
}
