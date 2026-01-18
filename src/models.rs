//! Shared data types for the application.

use std::collections::HashMap;

/// Represents a group (folder) in the database tree.
#[derive(Debug, Clone)]
pub struct Group {
    pub uuid: String,
    pub name: String,
    pub children: Vec<Group>,
    pub entries: Vec<Entry>,
}

/// Represents a password entry.
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub uuid: String,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    pub custom_fields: HashMap<String, String>,
}

impl Entry {
    /// Create an empty entry for new entry creation.
    pub fn new() -> Self {
        Self::default()
    }
}
