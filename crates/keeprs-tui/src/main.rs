//! Keeprs TUI - Terminal UI for KeePass
//!
//! Built with Ratatui and crossterm.

mod app;
mod handlers;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

use app::{App, AppState};

/// Keeprs TUI - Terminal UI for KeePass databases
#[derive(Parser, Debug)]
#[command(name = "keeprs-tui")]
#[command(about = "A terminal UI for KeePass databases")]
struct Args {
    /// Path to the KeePass database file
    #[arg(short, long)]
    database: PathBuf,
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("keeprs_tui=info".parse()?))
        .with_writer(std::io::stderr) // Write logs to stderr to not interfere with TUI
        .init();

    let args = Args::parse();
    tracing::info!("Starting Keeprs TUI with database: {:?}", args.database);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(args.database);

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        // Poll for events with timeout for smooth updates
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if handlers::handle_key(app, key) {
                    break;
                }
            }
        }

        // Check if we should quit
        if matches!(app.state, AppState::Quit) {
            break;
        }
    }

    Ok(())
}
