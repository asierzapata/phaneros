use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use phaneros_ipc::methods::{
    ActivityListParams, AddDriveParams, DriveIdParams, DriveStatusResult, DriveSummary, StatsParams,
};
use phaneros_ipc::{IpcClient, IpcError, Notification, PingResult, Request};
use serde_json::Value;

/// Label for the CLI's own per-user LaunchAgent registration. Distinct from
/// the desktop app's `com.asierzapata.phaneros-desktop.phanerosd` so the two
/// clients don't fight over the same registration.
#[cfg(target_os = "macos")]
const LOGIN_ITEM_LABEL: &str = "com.asierzapata.phaneros-cli.phanerosd";

/// A command-line client for the Phaneros sync daemon (`phanerosd`).
///
/// Talks to the daemon over its JSON-RPC control-plane socket; it does not
/// run any sync engine itself. Start `phanerosd` first.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the daemon's control-plane unix socket. Defaults to the
    /// daemon's default socket path; must match `daemon.ipc_socket` if the
    /// daemon was configured with a custom one.
    #[arg(long, global = true, value_name = "PATH")]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List every drive known to the daemon.
    List,
    /// Show a single drive's status and current sync progress.
    Status {
        #[arg(long)]
        drive_id: String,
    },
    /// Start a configured-but-stopped drive.
    Start { drive_id: String },
    /// Gracefully stop a running drive.
    Stop { drive_id: String },
    /// Add a new drive to the daemon's configuration.
    Add {
        drive_id: String,
        /// Local directory to sync.
        #[arg(long)]
        path: PathBuf,
        /// Base URL of the remote phaneros-store, if different from the daemon default.
        #[arg(long)]
        store_url: Option<String>,
        /// Bearer token for authenticating with the remote store.
        #[arg(long)]
        token: Option<String>,
        /// Add the drive without starting it.
        #[arg(long)]
        disabled: bool,
    },
    /// Guided setup: install and start the daemon, then configure and start
    /// a drive. Prompts for anything not passed as a flag.
    Setup {
        /// Local directory to sync. Prompted for if omitted.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Base URL of the remote phaneros-store. Prompted for if omitted.
        #[arg(long)]
        store_url: Option<String>,
        /// Bearer token for authenticating with the remote store. Prompted for if omitted.
        #[arg(long)]
        token: Option<String>,
        /// Identifier for the new drive. Defaults to the path's directory name.
        #[arg(long)]
        drive_id: Option<String>,
        /// Config file to pass through to `phanerosd --config`.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Add the drive without starting it.
        #[arg(long)]
        disabled: bool,
    },
    /// Remove a drive from the daemon's configuration.
    Remove { drive_id: String },
    /// Force an immediate sync pass for a drive.
    Sync { drive_id: String },
    /// Stream live sync progress and status changes.
    Watch {
        /// Only show events for this drive.
        #[arg(long)]
        drive_id: Option<String>,
    },
    /// Display sync efficiency metrics and historical insights.
    Stats {
        #[arg(long)]
        drive_id: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show recent completed sync sessions.
    Activity {
        #[arg(long)]
        drive_id: Option<String>,
        /// Maximum number of sessions to show.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Manage the daemon itself (lifecycle, diagnostics).
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
}

#[derive(Subcommand)]
enum DaemonCommands {
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let socket_path = cli.socket.clone().or_else(default_socket_path).unwrap_or_else(|| {
        eprintln!(
            "Could not determine a default daemon socket path; pass --socket explicitly."
        );
        std::process::exit(1);
    });

    match cli.command {
        Commands::List => {
            let value = call(&socket_path, Request::DrivesList).await;
            let drives: Vec<DriveSummary> = expect_value(value);
            print_drive_table(&drives);
        }
        Commands::Status { drive_id } => {
            let value = call(&socket_path, Request::DrivesStatus(DriveIdParams { drive_id })).await;
            let status: DriveStatusResult = expect_value(value);
            print_drive_status(&status);
        }
        Commands::Start { drive_id } => {
            call(&socket_path, Request::DrivesStart(DriveIdParams { drive_id: drive_id.clone() })).await;
            println!("Started drive '{}'.", drive_id);
        }
        Commands::Stop { drive_id } => {
            call(&socket_path, Request::DrivesStop(DriveIdParams { drive_id: drive_id.clone() })).await;
            println!("Stopped drive '{}'.", drive_id);
        }
        Commands::Add {
            drive_id,
            path,
            store_url,
            token,
            disabled,
        } => {
            let params = build_add_drive_params(drive_id.clone(), path, store_url, token, disabled);
            call(&socket_path, Request::DrivesAdd(params)).await;
            println!("Added drive '{}'.", drive_id);
        }
        Commands::Setup {
            path,
            store_url,
            token,
            drive_id,
            config,
            disabled,
        } => setup(&socket_path, path, store_url, token, drive_id, config, disabled).await,
        Commands::Remove { drive_id } => {
            call(&socket_path, Request::DrivesRemove(DriveIdParams { drive_id: drive_id.clone() })).await;
            println!("Removed drive '{}'.", drive_id);
        }
        Commands::Sync { drive_id } => {
            call(
                &socket_path,
                Request::DrivesTriggerSync(DriveIdParams { drive_id: drive_id.clone() }),
            )
            .await;
            println!("Triggered a sync for drive '{}'.", drive_id);
        }
        Commands::Watch { drive_id } => watch(&socket_path, drive_id).await,
        Commands::Stats { drive_id, json } => {
            let value = call(&socket_path, Request::StatsAggregate(StatsParams { drive_id: drive_id.clone() })).await;
            if json {
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
            } else {
                print_stats(drive_id.as_deref(), &value);
            }
        }
        Commands::Activity { drive_id, limit, json } => {
            let value = call(
                &socket_path,
                Request::ActivityList(ActivityListParams { drive_id: drive_id.clone(), limit }),
            )
            .await;
            if json {
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
            } else {
                print_activity(&value);
            }
        }
        Commands::Daemon { command } => match command {
            DaemonCommands::Ping => {
                let value = call(&socket_path, Request::DaemonPing).await;
                let ping: PingResult = expect_value(value);
                println!(
                    "phanerosd {} (pid {}), up {}s, configured: {}",
                    ping.version, ping.pid, ping.uptime_seconds, ping.configured
                );
            }
            DaemonCommands::Status => daemon_status(&socket_path).await,
            DaemonCommands::Start { config } => daemon_start(config).await,
            DaemonCommands::Stop => {
                call(&socket_path, Request::DaemonShutdown).await;
                println!("Stopped phanerosd.");
            }
            DaemonCommands::Install => daemon_install(),
            DaemonCommands::Uninstall => daemon_uninstall(),
        },
    }
}

async fn daemon_status(socket_path: &Path) {
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

fn build_add_drive_params(
    drive_id: String,
    path: PathBuf,
    store_url: Option<String>,
    token: Option<String>,
    disabled: bool,
) -> AddDriveParams {
    AddDriveParams {
        drive_id,
        path: path.to_string_lossy().to_string(),
        token,
        store_url,
        enabled: !disabled,
    }
}

/// Prompts on stdout/stdin for a value, falling back to `default` (if any)
/// on an empty line. Returns `None` if no default was given and the user
/// entered nothing.
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

/// Undoes POSIX shell-style backslash-escaping (`\ ` -> ` `, `\'` -> `'`,
/// etc). Paths dragged from Finder or pasted from shell history often arrive
/// pre-escaped this way, but `prompt()` reads raw stdin rather than going
/// through a shell, so those literal backslashes would otherwise end up as
/// part of the path.
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

/// Pings the daemon once without exiting the process on failure, so callers
/// can decide how to react (e.g. `setup` deciding whether to spawn a new
/// daemon or reuse an already-running one).
async fn try_ping(socket_path: &Path) -> Option<PingResult> {
    let mut client = IpcClient::connect(socket_path).await.ok()?;
    let value = client.call(Request::DaemonPing).await.ok()?;
    serde_json::from_value(value).ok()
}

/// Polls the daemon until it responds to a ping or the retry budget is
/// exhausted. Used right after spawning `phanerosd`, since startup (socket
/// bind, etc.) happens asynchronously relative to the spawned process.
async fn wait_for_daemon(socket_path: &Path) -> Option<PingResult> {
    for _ in 0..25 {
        if let Some(ping) = try_ping(socket_path).await {
            return Some(ping);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    None
}

async fn setup(
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
            match locate_daemon_binary() {
                Ok(daemon_path) => match phaneros_daemon::launchd::install(&phaneros_daemon::launchd::LoginItemConfig {
                    label: LOGIN_ITEM_LABEL.to_string(),
                    daemon_path,
                }) {
                    Ok(()) => println!("Registered phanerosd to start at login."),
                    Err(err) => println!("Warning: could not register login item: {}", err),
                },
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }

            daemon_start(config).await;

            match wait_for_daemon(socket_path).await {
                Some(ping) => ping,
                None => {
                    eprintln!("phanerosd did not become reachable at {}.", socket_path.display());
                    std::process::exit(1);
                }
            }
        }
    };
    println!("phanerosd {} (pid {}) is reachable.", ping.version, ping.pid);

    let path = match path {
        Some(path) => path,
        None => {
            let default_path = dirs::home_dir()
                .map(|home| home.join("Phaneros"))
                .unwrap_or_else(|| PathBuf::from("~/Phaneros"));
            let entered = prompt("Local directory to sync", Some(&default_path.to_string_lossy()));
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

    let params = build_add_drive_params(drive_id.clone(), path.clone(), store_url.clone(), token, disabled);
    call(socket_path, Request::DrivesAdd(params)).await;

    println!();
    println!("=== Setup complete ===");
    println!("Drive:      {}", drive_id);
    println!("Path:       {}", path.display());
    println!("Store URL:  {}", store_url.unwrap_or_else(|| "(daemon default)".to_string()));
    println!("Started:    {}", !disabled);
    println!();
    println!("Run `phaneros status --drive-id {}` to check on it, or `phaneros watch` to follow sync events live.", drive_id);
}

/// Resolves `phanerosd` via `$PATH` only; the CLI is a plain binary with no
/// bundled sidecar, unlike the desktop app.
fn locate_daemon_binary() -> Result<PathBuf, String> {
    let binary_name = if cfg!(windows) { "phanerosd.exe" } else { "phanerosd" };

    // Check next to the running `phaneros` executable first, so a binary
    // invoked straight out of `target/debug` or `target/release` (or an
    // install directory containing both binaries) finds its daemon without
    // needing anything on `$PATH`.
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

async fn daemon_start(config: Option<PathBuf>) {
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

    match command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => println!("Started phanerosd (pid {}).", child.id()),
        Err(err) => {
            eprintln!("Failed to start phanerosd: {}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(target_os = "macos")]
fn daemon_install() {
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
fn daemon_install() {
    eprintln!("Starting phanerosd at login is only supported on macOS right now.");
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
fn daemon_uninstall() {
    match phaneros_daemon::launchd::uninstall(LOGIN_ITEM_LABEL) {
        Ok(()) => println!("Unregistered phanerosd's login item."),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn daemon_uninstall() {
    eprintln!("Starting phanerosd at login is only supported on macOS right now.");
    std::process::exit(1);
}

fn default_socket_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("phaneros").join("phaneros.sock"))
}

async fn connect(socket_path: &Path) -> IpcClient {
    match IpcClient::connect(socket_path).await {
        Ok(client) => client,
        Err(err) => {
            eprintln!(
                "Could not connect to the phaneros daemon at {} ({}).",
                socket_path.display(),
                err
            );
            eprintln!("Is `phanerosd` running? Start it, or pass --socket to point at a different instance.");
            std::process::exit(1);
        }
    }
}

/// Connects fresh, makes one request, and exits the process with a clear
/// error message on any transport or daemon-side failure.
async fn call(socket_path: &Path, request: Request) -> Value {
    let mut client = connect(socket_path).await;
    match client.call(request).await {
        Ok(value) => value,
        Err(IpcError::Remote(err)) => {
            eprintln!("Error: {}", err.message);
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("Error talking to phanerosd: {}", err);
            std::process::exit(1);
        }
    }
}

fn expect_value<T: serde::de::DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("Received an unexpected response from phanerosd: {}", err);
            std::process::exit(1);
        }
    }
}

async fn watch(socket_path: &Path, drive_id_filter: Option<String>) {
    let client = connect(socket_path).await;
    let mut client = match client.subscribe().await {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Failed to subscribe to daemon events: {}", err);
            std::process::exit(1);
        }
    };

    println!("Watching for sync events... (Ctrl-C to stop)");
    loop {
        match client.next_notification().await {
            Ok(Some(Notification::EventProgress { drive_id, event })) => {
                if drive_id_filter.as_deref().is_some_and(|f| f != drive_id) {
                    continue;
                }
                println!(
                    "[{}] {:?} - {}/{} blobs, {} B sent",
                    drive_id, event.phase, event.blobs_completed, event.blobs_total, event.bytes_sent
                );
            }
            Ok(Some(Notification::EventDriveStatusChanged { drive_id, status })) => {
                if drive_id_filter.as_deref().is_some_and(|f| f != drive_id) {
                    continue;
                }
                println!("[{}] status changed: {}", drive_id, status);
            }
            Ok(None) => {
                println!("Connection closed by daemon.");
                break;
            }
            Err(err) => {
                eprintln!("Error reading events: {}", err);
                break;
            }
        }
    }
}

fn print_drive_table(drives: &[DriveSummary]) {
    if drives.is_empty() {
        println!("No drives configured.");
        return;
    }
    println!("{:<20} {:<10} {:<12} {:<40}", "DRIVE", "ENABLED", "STATUS", "PATH");
    for drive in drives {
        println!(
            "{:<20} {:<10} {:<12} {:<40}",
            drive.drive_id,
            drive.enabled,
            drive.status.to_string(),
            drive.path
        );
    }
}

fn print_drive_status(status: &DriveStatusResult) {
    println!("Drive:      {}", status.summary.drive_id);
    println!("Path:       {}", status.summary.path);
    println!("Store URL:  {}", status.summary.store_url);
    println!("Enabled:    {}", status.summary.enabled);
    println!("Status:     {}", status.summary.status);
    if let Some(root) = &status.summary.last_synced_root {
        println!("Last root:  {}", root);
    }
    if let Some(progress) = &status.progress {
        println!(
            "Progress:   {:?} ({}/{} blobs, {} B sent)",
            progress.phase, progress.blobs_completed, progress.blobs_total, progress.bytes_sent
        );
    }
}

fn print_stats(drive_id: Option<&str>, value: &Value) {
    let total_syncs = value["total_syncs"].as_u64().unwrap_or(0);
    let total_raw_bytes = value["total_raw_bytes"].as_u64().unwrap_or(0);
    let total_wire_bytes = value["total_wire_bytes"].as_u64().unwrap_or(0);
    let total_dedup_bytes = value["total_dedup_bytes"].as_u64().unwrap_or(0);
    let compression_pct = value["overall_compression_ratio_pct"].as_f64().unwrap_or(0.0);
    let avg_speed = value["avg_upload_rate_bps"].as_u64().unwrap_or(0);

    println!("=== Phaneros Sync Telemetry Insights ===");
    if let Some(did) = drive_id {
        println!("Filter Drive ID:        {}", did);
    }
    println!("Total Sync Sessions:     {}", total_syncs);
    println!("Logical Data Processed:  {}", format_bytes(total_raw_bytes));
    println!("Wire Bytes Transferred:  {}", format_bytes(total_wire_bytes));
    println!("Deduplicated Bytes Saved:{}", format_bytes(total_dedup_bytes));
    println!("Compression Efficiency:  {:.2}% savings", compression_pct);
    println!("Average Upload Speed:    {}", format_speed(avg_speed));
    println!("=========================================");
}

fn print_activity(value: &Value) {
    let sessions = value.as_array().cloned().unwrap_or_default();
    if sessions.is_empty() {
        println!("No sync activity recorded.");
        return;
    }
    println!("{:<20} {:<20} {:<12} {:<12}", "DRIVE", "TIMESTAMP", "WIRE BYTES", "AVG SPEED");
    for session in &sessions {
        let drive_id = session["drive_id"].as_str().unwrap_or("?");
        let timestamp = session["timestamp_epoch_sec"].as_u64().unwrap_or(0);
        let wire_bytes = session["transfer"]["wire_bytes_sent"].as_u64().unwrap_or(0);
        let avg_speed = session["avg_upload_rate_bps"].as_u64().unwrap_or(0);
        println!(
            "{:<20} {:<20} {:<12} {:<12}",
            drive_id,
            timestamp,
            format_bytes(wire_bytes),
            format_speed(avg_speed)
        );
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_speed(bps: u64) -> String {
    format!("{}/s", format_bytes(bps))
}
