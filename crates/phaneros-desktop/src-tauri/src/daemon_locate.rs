use std::path::PathBuf;

/// Locates the `phanerosd` binary the desktop app should spawn/register for
/// auto-start, trying the packaged sidecar location first and falling back
/// to a `$PATH` lookup for local development.
///
/// Production builds bundle `phanerosd` as a Tauri external binary
/// (`tauri.conf.json` -> `bundle.externalBin`), which places it next to the
/// app's own executable inside the bundle. In `tauri dev`/local `cargo
/// build` setups there is no bundle, so a bare binary name resolved via
/// `$PATH` is used instead (works for a `cargo install`-in-place developer
/// setup).
pub fn locate_daemon_binary() -> Result<PathBuf, String> {
    if let Some(bundled) = bundled_daemon_path() {
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    // Bare name: let the OS resolve it via $PATH. We can't verify existence
    // ahead of time without duplicating $PATH-scanning logic, so this is
    // handed to `Command::spawn`, which will surface a clear "not found"
    // error itself if it can't be resolved either.
    let path_name = PathBuf::from("phanerosd");
    if which_on_path(&path_name) {
        return Ok(path_name);
    }

    Err(format!(
        "Could not locate the phanerosd binary (checked {} and $PATH). Is phaneros-daemon installed?",
        bundled_daemon_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<bundle path unavailable>".to_string())
    ))
}

fn bundled_daemon_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join(daemon_binary_name()))
}

#[cfg(target_os = "windows")]
fn daemon_binary_name() -> &'static str {
    "phanerosd.exe"
}

#[cfg(not(target_os = "windows"))]
fn daemon_binary_name() -> &'static str {
    "phanerosd"
}

fn which_on_path(binary: &PathBuf) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(binary).is_file())
}
