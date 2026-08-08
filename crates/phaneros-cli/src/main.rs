mod commands;
mod ui;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use commands::client::default_socket_path;
use commands::daemon::DaemonCommands;

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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let socket_path = cli
        .socket
        .clone()
        .or_else(default_socket_path)
        .unwrap_or_else(|| {
            eprintln!(
                "Could not determine a default daemon socket path; pass --socket explicitly."
            );
            std::process::exit(1);
        });

    let client = commands::client::SocketIpcCaller::new(&socket_path);

    match cli.command {
        Commands::List => commands::list::handle(&client).await,
        Commands::Status { drive_id } => commands::status::handle(&client, drive_id).await,
        Commands::Start { drive_id } => commands::start::handle(&client, drive_id).await,
        Commands::Stop { drive_id } => commands::stop::handle(&client, drive_id).await,
        Commands::Add {
            drive_id,
            path,
            store_url,
            token,
            disabled,
        } => commands::add::handle(&client, drive_id, path, store_url, token, disabled).await,
        Commands::Setup {
            path,
            store_url,
            token,
            drive_id,
            config,
            disabled,
        } => {
            commands::setup::handle(
                &socket_path,
                path,
                store_url,
                token,
                drive_id,
                config,
                disabled,
            )
            .await
        }
        Commands::Remove { drive_id } => commands::remove::handle(&client, drive_id).await,
        Commands::Sync { drive_id } => commands::sync::handle(&client, drive_id).await,
        Commands::Watch { drive_id } => commands::watch::handle(&socket_path, drive_id).await,
        Commands::Stats { drive_id, json } => {
            commands::stats::handle(&client, drive_id, json).await
        }
        Commands::Activity {
            drive_id,
            limit,
            json,
        } => commands::activity::handle(&client, drive_id, limit, json).await,
        Commands::Daemon { command } => commands::daemon::handle(&client, &socket_path, command).await,
    }
}
