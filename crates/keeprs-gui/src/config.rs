//! Configuration file handling.
//!
//! Reads from `~/.config/keeprs/keeprs.toml`

use anyhow::{Context, Result};
use gtk4::gdk;
use gtk4::glib::translate::FromGlib;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configurable keyboard shortcuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    /// Save database shortcut (e.g., "Ctrl+S")
    #[serde(default = "default_save_database")]
    pub save_database: String,
    /// Toggle search palette shortcut (e.g., "Ctrl+P")
    #[serde(default = "default_toggle_search")]
    pub toggle_search: String,
    /// Navigate up in search results (e.g., "Up")
    #[serde(default = "default_navigate_up")]
    pub navigate_up: String,
    /// Navigate down in search results (e.g., "Down")
    #[serde(default = "default_navigate_down")]
    pub navigate_down: String,
    /// Close/cancel action (e.g., "Escape")
    #[serde(default = "default_close")]
    pub close: String,
    /// Confirm/select action (e.g., "Return")
    #[serde(default = "default_confirm")]
    pub confirm: String,
}

fn default_save_database() -> String { "Ctrl+S".to_string() }
fn default_toggle_search() -> String { "Ctrl+P".to_string() }
fn default_navigate_up() -> String { "Up".to_string() }
fn default_navigate_down() -> String { "Down".to_string() }
fn default_close() -> String { "Escape".to_string() }
fn default_confirm() -> String { "Return".to_string() }

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            save_database: default_save_database(),
            toggle_search: default_toggle_search(),
            navigate_up: default_navigate_up(),
            navigate_down: default_navigate_down(),
            close: default_close(),
            confirm: default_confirm(),
        }
    }
}

impl Keybindings {
    /// Parse a keybinding string like "Ctrl+S" into (Key, ModifierType).
    /// Returns None if parsing fails.
    pub fn parse(binding: &str) -> Option<(gdk::Key, gdk::ModifierType)> {
        let parts: Vec<&str> = binding.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = gdk::ModifierType::empty();
        let key_part = parts.last()?;

        // Parse modifiers (all parts except the last one)
        for part in parts.iter().take(parts.len() - 1) {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= gdk::ModifierType::CONTROL_MASK,
                "alt" => modifiers |= gdk::ModifierType::ALT_MASK,
                "shift" => modifiers |= gdk::ModifierType::SHIFT_MASK,
                "super" | "mod" | "meta" => modifiers |= gdk::ModifierType::SUPER_MASK,
                _ => return None, // Unknown modifier
            }
        }

        // Parse the key
        let key = Self::parse_key(key_part)?;
        Some((key, modifiers))
    }

    fn parse_key(key_str: &str) -> Option<gdk::Key> {
        // Handle named keys first
        match key_str.to_lowercase().as_str() {
            "escape" | "esc" => Some(gdk::Key::Escape),
            "return" | "enter" => Some(gdk::Key::Return),
            "tab" => Some(gdk::Key::Tab),
            "space" => Some(gdk::Key::space),
            "backspace" => Some(gdk::Key::BackSpace),
            "delete" | "del" => Some(gdk::Key::Delete),
            "up" => Some(gdk::Key::Up),
            "down" => Some(gdk::Key::Down),
            "left" => Some(gdk::Key::Left),
            "right" => Some(gdk::Key::Right),
            "home" => Some(gdk::Key::Home),
            "end" => Some(gdk::Key::End),
            "pageup" | "page_up" => Some(gdk::Key::Page_Up),
            "pagedown" | "page_down" => Some(gdk::Key::Page_Down),
            "insert" => Some(gdk::Key::Insert),
            "f1" => Some(gdk::Key::F1),
            "f2" => Some(gdk::Key::F2),
            "f3" => Some(gdk::Key::F3),
            "f4" => Some(gdk::Key::F4),
            "f5" => Some(gdk::Key::F5),
            "f6" => Some(gdk::Key::F6),
            "f7" => Some(gdk::Key::F7),
            "f8" => Some(gdk::Key::F8),
            "f9" => Some(gdk::Key::F9),
            "f10" => Some(gdk::Key::F10),
            "f11" => Some(gdk::Key::F11),
            "f12" => Some(gdk::Key::F12),
            _ => {
                // Handle single character keys
                if key_str.len() == 1 {
                    let c = key_str.chars().next()?;
                    // Convert to lowercase for matching
                    let keyval = gdk::unicode_to_keyval(c.to_ascii_lowercase() as u32);
                    Some(unsafe { gdk::Key::from_glib(keyval) })
                } else {
                    None
                }
            }
        }
    }
    
    /// Check if a key event matches a keybinding string.
    pub fn matches(binding: &str, key: gdk::Key, state: gdk::ModifierType) -> bool {
        if let Some((expected_key, expected_mods)) = Self::parse(binding) {
            // Normalize the key to lowercase for comparing
            // Convert to u32, use unicode_to_keyval, then convert back to Key using from_glib
            let key_char = key.to_unicode();
            let key_lower: gdk::Key = if let Some(c) = key_char {
                 unsafe { gdk::Key::from_glib(gdk::unicode_to_keyval(c.to_ascii_lowercase() as u32)) }
            } else {
                key
            };
            
            let expected_char = expected_key.to_unicode();
            let expected_lower: gdk::Key = if let Some(c) = expected_char {
                 unsafe { gdk::Key::from_glib(gdk::unicode_to_keyval(c.to_ascii_lowercase() as u32)) }
            } else {
                expected_key
            };
            
            // For special keys (arrows, escape, etc.), compare directly
            let key_matches = if key_char.is_none() || expected_char.is_none() {
                key == expected_key
            } else {
                key_lower == expected_lower
            };
            
            // Check modifiers - only compare the modifiers we care about
            let relevant_mods = gdk::ModifierType::CONTROL_MASK 
                | gdk::ModifierType::ALT_MASK 
                | gdk::ModifierType::SHIFT_MASK 
                | gdk::ModifierType::SUPER_MASK;
            let state_relevant = state & relevant_mods;
            
            key_matches && state_relevant == expected_mods
        } else {
            false
        }
    }
}

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
    /// Keyboard shortcuts.
    #[serde(default)]
    pub keybindings: Keybindings,
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
            keybindings: Keybindings::default(),
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
