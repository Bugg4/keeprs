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
    /// Whether to show the password entropy bar.
    #[serde(default = "default_show_entropy_bar")]
    pub show_entropy_bar: bool,
    /// Whether to show TOTP codes by default (visible) or hidden.
    #[serde(default = "default_show_totp_visible")]
    pub show_totp_visible: bool,
    /// List of group/entry names to hide from the UI.
    #[serde(default)]
    pub hidden_groups: Vec<String>,
}

fn default_sidebar_initial_width() -> i32 {
    280
}

fn default_sidebar_min_width() -> i32 {
    150
}

fn default_show_entropy_bar() -> bool {
    true
}

fn default_show_totp_visible() -> bool {
    false
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("database.kdbx"),
            sidebar_initial_width: default_sidebar_initial_width(),
            sidebar_min_width: default_sidebar_min_width(),
            show_entropy_bar: default_show_entropy_bar(),
            show_totp_visible: default_show_totp_visible(),
            hidden_groups: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from the config file.
    ///
    /// If `custom_path` is provided, load from that path.
    /// Otherwise, load from the default XDG config location.
    /// Creates a default config file if it doesn't exist (only for default path).
    pub fn load(custom_path: Option<PathBuf>) -> Result<Self> {
        let is_custom = custom_path.is_some();
        let config_path = match custom_path {
            Some(path) => path,
            None => Self::config_path()?,
        };

        if !config_path.exists() {
            // Only create default config for the default path
            if !is_custom {
                let config = Config::default();
                config.save()?;
                tracing::info!("Created default config: {:?}", config);
                return Ok(config);
            } else {
                anyhow::bail!("Config file not found: {}", config_path.display());
            }
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
