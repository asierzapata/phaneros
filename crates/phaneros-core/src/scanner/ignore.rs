use std::fs;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::utils::filesystem_write::is_internal_entry;

#[derive(Debug, Clone)]
pub struct IgnoreFilter {
    root: PathBuf,
    gitignore: Gitignore,
}

impl IgnoreFilter {
    pub fn new(root: &Path) -> Self {
        let root = root.to_path_buf();
        let mut builder = GitignoreBuilder::new(&root);

        // Always ignore internal phaneros state directories and temp files
        let _ = builder.add_line(None, ".phaneros/");
        let _ = builder.add_line(None, ".phaneros");

        // Standard default noise files and directories (no git required)
        let _ = builder.add_line(None, ".git/");
        let _ = builder.add_line(None, ".DS_Store");
        let _ = builder.add_line(None, "node_modules/");
        let _ = builder.add_line(None, "target/");
        let _ = builder.add_line(None, ".goutputstream-*");
        let _ = builder.add_line(None, "*~");
        let _ = builder.add_line(None, ".*.swp");
        let _ = builder.add_line(None, ".*.swo");
        let _ = builder.add_line(None, "\\#*\\#");

        // Discover and load all .phanerosignore and .gitignore files in root and subdirectories
        Self::add_ignore_files_recursively(&mut builder, &root);

        let gitignore = builder.build().unwrap_or_else(|_| Gitignore::empty());

        Self { root, gitignore }
    }

    fn add_ignore_files_recursively(builder: &mut GitignoreBuilder, dir: &Path) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            // Skip heavy build directories during ignore file discovery
            if name == ".git" || name == ".phaneros" || name == "node_modules" || name == "target" {
                continue;
            }

            if path.is_file() && (name == ".gitignore" || name == ".phanerosignore") {
                let _ = builder.add(&path);
            } else if path.is_dir() {
                Self::add_ignore_files_recursively(builder, &path);
            }
        }
    }

    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        if let Some(file_name) = path.file_name() {
            let name_str = file_name.to_string_lossy();
            if is_internal_entry(&name_str) {
                return true;
            }
        }

        // Safely skip gitignore matching if path is outside the watched root
        if !path.starts_with(&self.root) {
            return false;
        }

        self.gitignore
            .matched_path_or_any_parents(path, is_dir)
            .is_ignore()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_default_ignore_patterns_without_git() {
        let dir = tempdir().unwrap();
        let filter = IgnoreFilter::new(dir.path());

        assert!(filter.is_ignored(&dir.path().join(".git"), true));
        assert!(filter.is_ignored(&dir.path().join(".DS_Store"), false));
        assert!(filter.is_ignored(&dir.path().join("node_modules"), true));
        assert!(filter.is_ignored(&dir.path().join("target"), true));
        assert!(filter.is_ignored(&dir.path().join(".phaneros"), true));

        assert!(!filter.is_ignored(&dir.path().join("src"), true));
        assert!(!filter.is_ignored(&dir.path().join("main.rs"), false));
    }

    #[tokio::test]
    async fn test_custom_phanerosignore_patterns() {
        let dir = tempdir().unwrap();

        let ignore_content = "*.log\nbuild/\nsecret.txt";
        fs::write(dir.path().join(".phanerosignore"), ignore_content).unwrap();

        let filter = IgnoreFilter::new(dir.path());

        assert!(filter.is_ignored(&dir.path().join("app.log"), false));
        assert!(filter.is_ignored(&dir.path().join("build"), true));
        assert!(filter.is_ignored(&dir.path().join("secret.txt"), false));

        assert!(!filter.is_ignored(&dir.path().join("app.rs"), false));
    }

    #[tokio::test]
    async fn test_nested_gitignore_patterns() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("subfolder");
        fs::create_dir_all(&sub).unwrap();

        fs::write(sub.join(".gitignore"), "*.dist\noutput/").unwrap();

        let filter = IgnoreFilter::new(dir.path());

        // File inside subfolder matching subfolder's .gitignore
        assert!(filter.is_ignored(&sub.join("bundle.dist"), false));
        assert!(filter.is_ignored(&sub.join("output"), true));
    }
}
