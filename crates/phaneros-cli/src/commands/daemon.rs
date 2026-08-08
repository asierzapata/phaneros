use std::path::{Path, PathBuf};
use phaneros_ipc::{IpcClient, PingResult, Request};
use crate::commands::client::{expect_value, IpcCaller};

#[cfg(target_os = "macos")]
pub const LOGIN_ITEM_LABEL: &str = "com.asierzapata.phaneros-cli.phanerosd";

#[derive(clap::Subcommand)]
pub enum DaemonCommands {
    /// Check that the daemon is reachable and responsive.
    Ping,
    /// Report whether the daemon is reachable, and (on macOS) whether it's
    /// registered to start at login.
    Status,
    /// Spawn `phanerosd` in the background (resolved via `$PATH`).
    Start {
        /// Config file to pass through to `phanerosd --config`. Defaults to
        /// the daemon's own default config path if omitted.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Gracefully shut down the running daemon.
    Stop,
    /// Register `phanerosd` as a per-user login item (macOS only).
    Install,
    /// Unregister the per-user login item (macOS only).
    Uninstall,
}

pub async fn handle(client: &impl IpcCaller, socket_path: &Path, command: DaemonCommands) {
    match command {
        DaemonCommands::Ping => {
            let value = client.call(Request::DaemonPing).await;
            let ping: PingResult = expect_value(value);
            println!(
                "phanerosd {} (pid {}), up {}s, configured: {}",
                ping.version, ping.pid, ping.uptime_seconds, ping.configured
            );
        }
        DaemonCommands::Status => status(socket_path).await,
        DaemonCommands::Start { config } => start(config).await,
        DaemonCommands::Stop => {
            client.call(Request::DaemonShutdown).await;
            println!("Stopped phanerosd.");
        }
        DaemonCommands::Install => install(),
        DaemonCommands::Uninstall => uninstall(),
    }
}

pub async fn status(socket_path: &Path) {
    let mut client_result = IpcClient::connect(socket_path).await;
    match client_result.as_mut() {
        Ok(client) => match client.call(Request::DaemonPing).await {
            Ok(value) => {
                let ping: PingResult = expect_value(value);
                println!(
                    "Reachable: phanerosd {} (pid {}), up {}s, configured: {}",
                    ping.version, ping.pid, ping.uptime_seconds, ping.configured
                );
            }
            Err(err) => println!("Unreachable: {}", err),
        },
        Err(err) => println!("Unreachable: {}", err),
    }

    #[cfg(target_os = "macos")]
    match phaneros_daemon::launchd::is_installed(LOGIN_ITEM_LABEL) {
        Ok(true) => println!("Login item: installed"),
        Ok(false) => println!("Login item: not installed"),
        Err(err) => println!("Login item: unknown ({})", err),
    }
}

pub async fn start(config: Option<PathBuf>) {
    let daemon_path = match locate_daemon_binary() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let mut command = std::process::Command::new(&daemon_path);
    if let Some(config) = &config {
        command.arg("--config").arg(config);
    }

    let log_dir = phaneros_daemon::log_dir();
    if let Err(err) = std::fs::create_dir_all(&log_dir) {
        eprintln!(
            "Warning: could not create log directory {}: {}",
            log_dir.display(),
            err
        );
    }
    let open_log = |name: &str| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(name))
    };
    let stdout = open_log("phanerosd.log");
    let stderr = open_log("phanerosd.err.log");

    command.stdin(std::process::Stdio::null());
    match stdout {
        Ok(file) => {
            command.stdout(file);
        }
        Err(err) => {
            eprintln!("Warning: could not open phanerosd.log: {}", err);
            command.stdout(std::process::Stdio::null());
        }
    }
    match stderr {
        Ok(file) => {
            command.stderr(file);
        }
        Err(err) => {
            eprintln!("Warning: could not open phanerosd.err.log: {}", err);
            command.stderr(std::process::Stdio::null());
        }
    }

    match command.spawn() {
        Ok(child) => println!("Started phanerosd (pid {}).", child.id()),
        Err(err) => {
            eprintln!("Failed to start phanerosd: {}", err);
            std::process::exit(1);
        }
    }
}

pub fn locate_daemon_binary() -> Result<PathBuf, String> {
    let binary_name = if cfg!(windows) {
        "phanerosd.exe"
    } else {
        "phanerosd"
    };

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err("Could not locate the phanerosd binary (checked alongside phaneros and $PATH). Is phaneros-daemon installed?".to_string())
}

#[cfg(target_os = "macos")]
pub fn install() {
    let daemon_path = match locate_daemon_binary() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };
    match phaneros_daemon::launchd::install(&phaneros_daemon::launchd::LoginItemConfig {
        label: LOGIN_ITEM_LABEL.to_string(),
        daemon_path,
    }) {
        Ok(()) => println!("Registered phanerosd to start at login."),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install() {
    eprintln!("Starting phanerosd at login is only supported on macOS right now.");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
pub fn uninstall() {
    match phaneros_daemon::launchd::uninstall(LOGIN_ITEM_LABEL) {
        Ok(()) => println!("Unregistered phanerosd's login item."),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn uninstall() {
    eprintln!("Starting phanerosd at login is only supported on macOS right now.");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::client::MockIpcCaller;
    use serde_json::json;

    #[tokio::test]
    async fn test_daemon_ping() {
        let mock = MockIpcCaller::new(|req| {
            assert!(matches!(req, Request::DaemonPing));
            json!({
                "version": "0.2.1",
                "pid": 1234,
                "uptime_seconds": 100,
                "configured": true
            })
        });

        handle(&mock, Path::new("/tmp/test.sock"), DaemonCommands::Ping).await;
    }
}
