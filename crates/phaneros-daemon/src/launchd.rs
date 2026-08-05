//! Registers/unregisters `phanerosd` as a per-user macOS `launchd`
//! LaunchAgent, so it starts at login independent of whether any UI (the
//! desktop app) is open. No `sudo`/elevated privileges are needed — the
//! agent runs as the logged-in user, matching the daemon's existing
//! same-user socket/config assumptions.

use std::path::PathBuf;

/// What to install: a stable `label` identifying the LaunchAgent (reverse-DNS
/// style, e.g. `com.asierzapata.phaneros-desktop.phanerosd`) and the
/// absolute path to the `phanerosd` binary to run.
pub struct LoginItemConfig {
    pub label: String,
    pub daemon_path: PathBuf,
}

fn launch_agents_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    Ok(home.join("Library").join("LaunchAgents"))
}

fn plist_path(label: &str) -> Result<PathBuf, String> {
    Ok(launch_agents_dir()?.join(format!("{label}.plist")))
}

fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("phaneros")
}

fn plist_contents(config: &LoginItemConfig) -> String {
    let logs = log_dir();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{daemon_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
</dict>
</plist>
"#,
        label = config.label,
        daemon_path = config.daemon_path.display(),
        stdout = logs.join("phanerosd.log").display(),
        stderr = logs.join("phanerosd.err.log").display(),
    )
}

/// Writes the LaunchAgent plist and loads it via `launchctl`, so `phanerosd`
/// starts at the next login (and immediately, since `load -w` also starts
/// it now if `RunAtLoad` is set).
pub fn install(config: &LoginItemConfig) -> Result<(), String> {
    let dir = launch_agents_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    std::fs::create_dir_all(log_dir()).map_err(|e| format!("Could not create log directory: {e}"))?;

    let plist = plist_path(&config.label)?;
    std::fs::write(&plist, plist_contents(config))
        .map_err(|e| format!("Could not write {}: {e}", plist.display()))?;

    let output = std::process::Command::new("launchctl")
        .args(["load", "-w", &plist.display().to_string()])
        .output()
        .map_err(|e| format!("Failed to run launchctl load: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "launchctl load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Unloads and removes the LaunchAgent plist. Safe to call even if it was
/// never installed (unload failures are ignored; removing a missing file is
/// a no-op).
pub fn uninstall(label: &str) -> Result<(), String> {
    let plist = plist_path(label)?;

    if plist.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &plist.display().to_string()])
            .output();
        std::fs::remove_file(&plist)
            .map_err(|e| format!("Could not remove {}: {e}", plist.display()))?;
    }
    Ok(())
}

/// Whether the LaunchAgent plist is currently installed on disk. This checks
/// for the plist file rather than querying `launchctl list`, since the
/// latter only reflects the current login session, not persisted
/// registration.
pub fn is_installed(label: &str) -> Result<bool, String> {
    Ok(plist_path(label)?.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contents_embeds_label_and_path() {
        let config = LoginItemConfig {
            label: "com.example.phanerosd".to_string(),
            daemon_path: PathBuf::from("/usr/local/bin/phanerosd"),
        };
        let xml = plist_contents(&config);
        assert!(xml.contains("<string>com.example.phanerosd</string>"));
        assert!(xml.contains("<string>/usr/local/bin/phanerosd</string>"));
        assert!(xml.contains("<key>RunAtLoad</key>"));
        assert!(xml.contains("<false/>"));
    }
}
