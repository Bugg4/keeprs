//! Keeprs - A minimalist KeePass database manager
//!
//! Built with GTK4 and Relm4.

mod app;
mod components;
mod config;
mod widgets;

use anyhow::Result;
use clap::Parser;
use relm4::prelude::*;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

/// Keeprs - A minimalist KeePass database manager
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Path to the KeePass database file (overrides config)
    #[arg(short, long, value_name = "FILE")]
    database: Option<PathBuf>,
}

fn main() -> Result<()> {
    // Parse CLI arguments BEFORE initializing logging/GTK
    // This consumes the args so GTK won't see them
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("keeprs=info".parse()?))
        .init();

    tracing::info!("Starting Keeprs");

    // Load configuration
    let mut config = config::Config::load(args.config)?;
    
    // Override database path if provided via CLI
    if let Some(database_path) = args.database {
        tracing::info!("Overriding database path from CLI: {}", database_path.display());
        config.database_path = database_path;
    }
    
    tracing::info!("Database path: {}", config.database_path.display());

    // Run GTK application with empty args to prevent GTK from seeing our CLI args
    let app = RelmApp::new("io.github.keeprs");
    // Set empty args so GTK doesn't complain about unknown options
    app.with_args(Vec::<String>::new()).run::<app::App>(config);

    Ok(())
}
