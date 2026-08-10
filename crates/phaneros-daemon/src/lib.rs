//! Library surface for `phaneros-daemon`, exposed alongside the `phanerosd`
//! binary so other crates (the desktop app, a future CLI subcommand) can
//! reuse daemon-specific, platform-specific concerns like OS service
//! registration without duplicating them.

use std::path::PathBuf;

use clap::Parser;
use phaneros_core::PhanerosConfig;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

mod ipc;
mod notify;
mod state;

use state::{Command, DaemonState};

#[cfg(target_os = "macos")]
pub mod launchd;

pub fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("phaneros")
}

/// Phaneros background synchronization daemon.
#[derive(Parser)]
#[command(version, about)]
pub struct DaemonCli {
    /// Path to custom configuration TOML file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
}

pub async fn run_daemon() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = DaemonCli::parse();

    let (config, config_path) = match PhanerosConfig::load_or_default(args.config.as_deref()) {
        Ok(res) => res,
        Err(err) => {
            tracing::error!(error = %err, "configuration error");
            std::process::exit(1);
        }
    };

    let Some(socket_path) = config.resolve_ipc_socket_path() else {
        tracing::error!(
            "could not determine an IPC socket path (no daemon.ipc_socket configured and no default data directory available)"
        );
        std::process::exit(1);
    };

    tracing::info!(
        config_path = %config_path.display(),
        socket = %socket_path.display(),
        "starting phanerosd"
    );

    let daemon_cancel = CancellationToken::new();
    let (broadcast_tx, _) = broadcast::channel(256);
    let (command_tx, mut command_rx) = mpsc::channel::<Command>(64);

    let mut daemon_state = DaemonState::new(
        config,
        config_path,
        broadcast_tx.clone(),
        daemon_cancel.clone(),
    );
    daemon_state.start_enabled_drives().await;

    let actor_join = tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            let is_shutdown = matches!(command, Command::Shutdown(_));
            daemon_state.handle(command).await;
            if is_shutdown {
                break;
            }
        }
    });

    let listener = match ipc::bind(&socket_path).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, "failed to bind IPC socket");
            std::process::exit(1);
        }
    };

    let listener_join = tokio::spawn(ipc::serve(
        listener,
        command_tx.clone(),
        broadcast_tx,
        daemon_cancel.clone(),
    ));

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => tracing::info!("received SIGINT"),
        _ = sigterm.recv() => tracing::info!("received SIGTERM"),
        _ = daemon_cancel.cancelled() => {}
    }

    if !daemon_cancel.is_cancelled() {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = command_tx.send(Command::Shutdown(reply_tx)).await;
        let _ = reply_rx.await;
    }

    let _ = listener_join.await;
    let _ = actor_join.await;
    let _ = std::fs::remove_file(&socket_path);

    tracing::info!("phanerosd stopped");
}
