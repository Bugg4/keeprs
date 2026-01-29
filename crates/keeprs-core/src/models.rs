//! Shared data types for the application.

use std::collections::HashMap;

/// Represents a group (folder) in the database tree.
#[derive(Debug, Clone)]
pub struct Group {
    pub uuid: String,
    pub name: String,
    pub children: Vec<Group>,
    pub entries: Vec<Entry>,
    pub is_recycle_bin: bool,
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
    pub otp: Option<String>,
    pub attachments: Vec<Attachment>,
}

/// Represents a binary attachment.
#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub _mime_type: Option<String>,
    pub data: Vec<u8>,
}

impl Entry {
    /// Create an empty entry for new entry creation.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Represents a step in the navigation path (for Miller columns).
#[derive(Debug, Clone)]
pub enum NavigationStep {
    /// A group/folder was selected.
    Group { _uuid: String, name: String },
    /// An entry was selected.
    Entry { _uuid: String, title: String },
}

/// Navigation path tracking the current drill-down state.
#[derive(Debug, Clone, Default)]
pub struct NavigationPath {
    /// The steps in the navigation (each click adds a step).
    pub steps: Vec<NavigationStep>,
}

impl NavigationPath {
    /// Create a new empty navigation path.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a group selection to the path.
    pub fn push_group(&mut self, uuid: String, name: String) {
        self.steps.push(NavigationStep::Group { _uuid: uuid, name });
    }

    /// Add an entry selection to the path.
    pub fn push_entry(&mut self, uuid: String, title: String) {
        self.steps.push(NavigationStep::Entry { _uuid: uuid, title });
    }

    /// Truncate the path to the given depth (0 = clear all).
    pub fn truncate(&mut self, depth: usize) {
        self.steps.truncate(depth);
    }

    /// Get the current depth.
    pub fn depth(&self) -> usize {
        self.steps.len()
    }
}
