//! Keeprs - A minimalist KeePass database manager
//!
//! Built with GTK4 and Relm4.

mod app;
mod components;
mod config;

use anyhow::Result;
use relm4::prelude::*;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("keeprs=info".parse()?))
        .init();

    tracing::info!("Starting Keeprs");

    // Load configuration
    let config = config::Config::load()?;
    tracing::info!("Database path: {}", config.database_path.display());

    // Run GTK application
    let app = RelmApp::new("io.github.keeprs");
    app.run::<app::App>(config);

    Ok(())
}
