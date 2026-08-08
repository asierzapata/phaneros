use std::path::{Path, PathBuf};
use phaneros_ipc::{IpcClient, PingResult, Request};

use crate::commands::add::build_add_drive_params;
use crate::commands::client::call;
use crate::commands::daemon;

fn prompt(label: &str, default: Option<&str>) -> Option<String> {
    use std::io::Write;

    match default {
        Some(default) => print!("{} [{}]: ", label, default),
        None => print!("{}: ", label),
    }
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return default.map(str::to_string);
    }
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.map(str::to_string)
    } else {
        Some(trimmed.to_string())
    }
}

fn unescape_shell_path(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(next) = chars.next()
        {
            result.push(next);
            continue;
        }
        result.push(c);
    }
    result
}

async fn try_ping(socket_path: &Path) -> Option<PingResult> {
    let mut client = IpcClient::connect(socket_path).await.ok()?;
    let value = client.call(Request::DaemonPing).await.ok()?;
    serde_json::from_value(value).ok()
}

async fn wait_for_daemon(socket_path: &Path) -> Option<PingResult> {
    for _ in 0..25 {
        if let Some(ping) = try_ping(socket_path).await {
            return Some(ping);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    None
}

pub async fn handle(
    socket_path: &Path,
    path: Option<PathBuf>,
    store_url: Option<String>,
    token: Option<String>,
    drive_id: Option<String>,
    config: Option<PathBuf>,
    disabled: bool,
) {
    println!("=== Phaneros setup ===");

    let ping = match try_ping(socket_path).await {
        Some(ping) => {
            println!("Daemon already running (pid {}).", ping.pid);
            ping
        }
        None => {
            #[cfg(target_os = "macos")]
            match daemon::locate_daemon_binary() {
                Ok(daemon_path) => match phaneros_daemon::launchd::install(
                    &phaneros_daemon::launchd::LoginItemConfig {
                        label: daemon::LOGIN_ITEM_LABEL.to_string(),
                        daemon_path,
                    },
                ) {
                    Ok(()) => println!("Registered phanerosd to start at login."),
                    Err(err) => println!("Warning: could not register login item: {}", err),
                },
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }

            match wait_for_daemon(socket_path).await {
                Some(ping) => ping,
                None => {
                    daemon::start(config).await;
                    match wait_for_daemon(socket_path).await {
                        Some(ping) => ping,
                        None => {
                            eprintln!(
                                "phanerosd did not become reachable at {}.",
                                socket_path.display()
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
    };
    println!(
        "phanerosd {} (pid {}) is reachable.",
        ping.version, ping.pid
    );

    let path = match path {
        Some(path) => path,
        None => {
            let default_path = dirs::home_dir()
                .map(|home| home.join("Phaneros"))
                .unwrap_or_else(|| PathBuf::from("~/Phaneros"));
            let entered = prompt(
                "Local directory to sync",
                Some(&default_path.to_string_lossy()),
            );
            let entered = entered.unwrap_or_else(|| default_path.to_string_lossy().to_string());
            PathBuf::from(unescape_shell_path(&entered))
        }
    };

    let store_url = store_url.or_else(|| prompt("Store URL", Some("http://localhost:8080")));

    let token = match token {
        Some(token) => Some(token),
        None => prompt("Bearer token for the store (leave blank if none)", Some("")),
    }
    .filter(|t| !t.is_empty());

    let drive_id = drive_id.unwrap_or_else(|| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "default".to_string())
    });

    let params = build_add_drive_params(drive_id, path.clone(), store_url.clone(), token, disabled)
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            std::process::exit(1);
        });
    let drive_id = params.drive_id.clone();
    call(socket_path, Request::DrivesAdd(params)).await;

    println!();
    println!("=== Setup complete ===");
    println!("Drive:      {}", drive_id);
    println!("Path:       {}", path.display());
    println!(
        "Store URL:  {}",
        store_url.unwrap_or_else(|| "(daemon default)".to_string())
    );
    println!("Started:    {}", !disabled);
    println!();
    println!(
        "Run `phaneros status --drive-id {}` to check on it, or `phaneros watch` to follow sync events live.",
        drive_id
    );
}
