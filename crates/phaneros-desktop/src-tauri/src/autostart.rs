//! Thin Tauri-command wrappers around `phaneros_daemon::launchd`, letting
//! the desktop app register/unregister `phanerosd` as a per-user macOS
//! login item. Auto-start is macOS-only for now (see `tauri.conf.json`,
//! which has no Windows/Linux-specific config yet); other platforms get a
//! clear "not supported" error rather than a silent no-op.

use crate::daemon_locate;

const LOGIN_ITEM_LABEL: &str = "com.asierzapata.phaneros-desktop.phanerosd";

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn register_login_item() -> Result<(), String> {
    let daemon_path = daemon_locate::locate_daemon_binary()?;
    phaneros_daemon::launchd::install(&phaneros_daemon::launchd::LoginItemConfig {
        label: LOGIN_ITEM_LABEL.to_string(),
        daemon_path,
    })
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn register_login_item() -> Result<(), String> {
    Err("Starting phanerosd at login is only supported on macOS right now".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn unregister_login_item() -> Result<(), String> {
    phaneros_daemon::launchd::uninstall(LOGIN_ITEM_LABEL)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn unregister_login_item() -> Result<(), String> {
    Err("Starting phanerosd at login is only supported on macOS right now".to_string())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn is_login_item_registered() -> Result<bool, String> {
    phaneros_daemon::launchd::is_installed(LOGIN_ITEM_LABEL)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn is_login_item_registered() -> Result<bool, String> {
    Ok(false)
}
