use std::path::Path;

use tempfile::TempDir;

use crate::watcher::{WatchHandle, Watcher};

fn watch_temp_dir(path: &Path) -> WatchHandle {
    Watcher::new(path.to_string_lossy().into_owned())
        .watch()
        .unwrap()
}

#[test]
fn rescan_returns_the_current_root_hash() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("hello.txt"), b"hello").unwrap();

    let handle = watch_temp_dir(tmp.path());
    let root_hash = handle.rescan.rescan().unwrap();

    assert_eq!(root_hash, handle.initial_root_hash);
}

#[test]
fn rescan_picks_up_changes_made_since_the_last_scan() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("hello.txt"), b"hello").unwrap();

    let handle = watch_temp_dir(tmp.path());
    std::fs::write(tmp.path().join("other.txt"), b"other").unwrap();
    let root_hash = handle.rescan.rescan().unwrap();

    assert_ne!(root_hash, handle.initial_root_hash);
}

#[test]
fn rescan_still_works_after_the_root_hash_receiver_is_dropped() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("hello.txt"), b"hello").unwrap();

    let handle = watch_temp_dir(tmp.path());
    let rescan = handle.rescan.clone();
    let initial_root_hash = handle.initial_root_hash.clone();
    drop(handle.root_hashes);

    assert_eq!(rescan.rescan().unwrap(), initial_root_hash);
}
