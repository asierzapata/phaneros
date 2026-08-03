use std::path::PathBuf;

use clap::Parser;
use phaneros_core::config::expand_tilde;
use phaneros_core::{EngineConfig, PhanerosConfig, SyncEngine};

/// A command-line utility for synchronizing files and directories across
/// multiple devices.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to custom configuration TOML file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Directory to watch and sync. If omitted, uses the drive path from configuration.
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
    let cli = Cli::parse();

    let (config, config_path) = match PhanerosConfig::load_or_default(cli.config.as_deref()) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("Configuration error: {err}");
            std::process::exit(1);
        }
    };

    let configured_drive = config.drives.get(&cli.drive_id);

    let sync_path = match cli.path {
        Some(p) => expand_tilde(&p),
        None => match configured_drive {
            Some(drive) => drive.expanded_path(),
            None => {
                eprintln!(
                    "Error: No local PATH specified and drive '{}' not found in configuration file ({})",
                    cli.drive_id,
                    config_path.display()
                );
                std::process::exit(1);
            }
        },
    };

    let store_url = match cli.store_url {
        Some(url) => url,
        None => match configured_drive {
            Some(drive) => drive.get_effective_store_url(&config.daemon.store_url).to_string(),
            None => config.daemon.store_url.clone(),
        },
    };

    let token = match cli.token {
        Some(t) => t,
        None => match configured_drive {
            Some(drive) => drive.token.clone(),
            None => String::new(),
        },
    };

    println!("Using config file: {}", config_path.display());
    println!("Sync target path: {}", sync_path.display());
    println!("Remote store URL: {}", store_url);

    let engine_config = EngineConfig::new(
        sync_path,
        store_url,
        cli.drive_id,
        token,
        cli.dump_store,
    );

    let engine = SyncEngine::new(engine_config);
    if let Err(err) = engine.run() {
        eprintln!("Phaneros engine error: {err}");
        std::process::exit(1);
    }
}
