use std::path::PathBuf;

use clap::{Parser, Subcommand};
use phaneros_core::config::expand_tilde;
use phaneros_core::telemetry::MetricsDatabase;
use phaneros_core::{EngineConfig, PhanerosConfig, SyncEngine};

/// A command-line utility for synchronizing files and directories across
/// multiple devices.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to custom configuration TOML file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Directory to watch and sync.
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Base URL of the remote phaneros-store
    #[arg(long)]
    store_url: Option<String>,

    /// Drive identifier on the remote store
    #[arg(long, default_value = "default")]
    drive_id: String,

    /// Bearer token for authenticating with the remote store
    #[arg(long)]
    token: Option<String>,

    /// Debug: dump the local store state to DIR/local_store_dump.txt after every sync
    #[arg(
        long,
        value_name = "DIR",
        num_args = 0..=1,
        default_missing_value = "target"
    )]
    dump_store: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Display sync efficiency metrics, transfer rates, and historical insights
    Stats {
        /// Filter stats by drive ID
        #[arg(long)]
        drive_id: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Some(Commands::Stats { drive_id, json }) = cli.command {
        print_stats(drive_id.as_deref(), json);
        return;
    }

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
    )
    .with_telemetry(config.daemon.enable_telemetry);

    let engine = SyncEngine::new(engine_config);
    if let Err(err) = engine.run() {
        eprintln!("Phaneros engine error: {err}");
        std::process::exit(1);
    }
}

fn print_stats(drive_id: Option<&str>, json: bool) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("Failed to initialize runtime: {err}");
            std::process::exit(1);
        }
    };

    rt.block_on(async {
        let db = match MetricsDatabase::connect_default().await {
            Ok(db) => db,
            Err(err) => {
                eprintln!("Failed to open metrics database: {err}");
                std::process::exit(1);
            }
        };

        let stats = match db.get_aggregate_stats(drive_id).await {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Failed to query aggregate stats: {err}");
                std::process::exit(1);
            }
        };

        if json {
            println!("{}", serde_json::to_string_pretty(&stats).unwrap());
            return;
        }

        println!("=== Phaneros Sync Telemetry Insights ===");
        if let Some(did) = drive_id {
            println!("Filter Drive ID:        {}", did);
        }
        println!("Total Sync Sessions:     {}", stats.total_syncs);
        println!("Logical Data Processed:  {}", format_bytes(stats.total_raw_bytes));
        println!("Wire Bytes Transferred:  {}", format_bytes(stats.total_wire_bytes));
        println!("Deduplicated Bytes Saved:{}", format_bytes(stats.total_dedup_bytes));
        println!("Compression Efficiency:  {:.2}% savings", stats.overall_compression_ratio_pct);
        println!("Average Upload Speed:    {}", format_speed(stats.avg_upload_rate_bps));
        println!("=========================================");
    });
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
