//! Configuration file handling.
//!
//! Reads from `~/.config/keeprs/keeprs.toml`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the KeePass database file.
    pub database_path: PathBuf,
    /// Initial width for the sidebar on startup.
    #[serde(default = "default_sidebar_initial_width")]
    pub sidebar_initial_width: i32,
    /// Minimum width the sidebar can be shrunk to.
    #[serde(default = "default_sidebar_min_width")]
    pub sidebar_min_width: i32,
}

fn default_sidebar_initial_width() -> i32 {
    280
}

fn default_sidebar_min_width() -> i32 {
    150
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("database.kdbx"),
            sidebar_initial_width: default_sidebar_initial_width(),
            sidebar_min_width: default_sidebar_min_width(),
        }
    }
}

impl Config {
    /// Load configuration from the config file.
    ///
    /// Creates a default config file if it doesn't exist.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // Create default config
            let config = Config::default();
            config.save()?;
            tracing::info!("Created default config: {:?}", config);
            return Ok(config);
        }

        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
        
        tracing::info!("Loaded config from {}: {:?}", config_path.display(), config);
        Ok(config)
    }

    /// Save configuration to the config file.
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))
    }

    /// Get the path to the config file.
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?;

        Ok(config_dir.join("keeprs").join("keeprs.toml"))
    }
}
