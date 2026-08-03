use std::path::PathBuf;

use clap::Parser;
use phaneros_core::config::expand_tilde;
use phaneros_core::{EngineConfig, PhanerosConfig, SyncEngine};

/// Phaneros background synchronization daemon.
#[derive(Parser)]
#[command(version, about)]
struct DaemonCli {
    /// Path to custom configuration TOML file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Directory to watch and sync. Optional if specified in config.toml.
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Base URL of the remote phaneros-store (e.g. http://localhost:8080)
    #[arg(long)]
    store_url: Option<String>,

    /// Drive identifier on the remote store
    #[arg(long, default_value = "default")]
    drive_id: String,

    /// Bearer token for authenticating with the remote store
    #[arg(long)]
    token: Option<String>,

    /// Debug: dump the local store state to DIR/local_store_dump.txt after
    /// every sync
    #[arg(
        long,
        value_name = "DIR",
        num_args = 0..=1,
        default_missing_value = "target"
    )]
    dump_store: Option<PathBuf>,
}

fn main() {
    let args = DaemonCli::parse();

    println!("Starting Phaneros daemon (phanerosd)...");

    let (config, config_path) = match PhanerosConfig::load_or_default(args.config.as_deref()) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("Configuration error: {err}");
            std::process::exit(1);
        }
    };

    println!("Loaded daemon configuration from {}", config_path.display());

    let configured_drive = config.drives.get(&args.drive_id);

    let sync_path = match args.path {
        Some(p) => expand_tilde(&p),
        None => match configured_drive {
            Some(drive) => drive.expanded_path(),
            None => {
                eprintln!(
                    "Error: No local PATH specified and drive '{}' not found in configuration file ({})",
                    args.drive_id,
                    config_path.display()
                );
                std::process::exit(1);
            }
        },
    };

    let store_url = match args.store_url {
        Some(url) => url,
        None => match configured_drive {
            Some(drive) => drive.get_effective_store_url(&config.daemon.store_url).to_string(),
            None => config.daemon.store_url.clone(),
        },
    };

    let token = match args.token {
        Some(t) => t,
        None => match configured_drive {
            Some(drive) => drive.token.clone(),
            None => String::new(),
        },
    };

    println!("Syncing drive '{}' at {}", args.drive_id, sync_path.display());
    println!("Remote store: {}", store_url);

    let engine_config = EngineConfig::new(
        sync_path,
        store_url,
        args.drive_id,
        token,
        args.dump_store,
    );

    let engine = SyncEngine::new(engine_config);
    if let Err(err) = engine.run() {
        eprintln!("Phaneros daemon error: {err}");
        std::process::exit(1);
    }
}
