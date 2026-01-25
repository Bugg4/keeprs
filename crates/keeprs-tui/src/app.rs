//! Application state management.

use keeprs_core::{Entry, Group, KeepassDatabase};
use std::collections::HashSet;
use std::path::PathBuf;

/// Application state.
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Waiting for password entry.
    Locked,
    /// Database is unlocked.
    Unlocked,
    /// Application should quit.
    Quit,
}

/// Input mode for the application.
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal navigation mode.
    Normal,
    /// Password entry mode.
    PasswordEntry,
    /// Search mode.
    Search,
}

/// Focus area within the unlocked view.
#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    /// Sidebar tree is focused.
    Sidebar,
    /// Entry detail view is focused.
    EntryView,
}

/// Main application model.
pub struct App {
    /// Current application state.
    pub state: AppState,
    /// Current input mode.
    pub input_mode: InputMode,
    /// Current focus area.
    pub focus: Focus,
    /// Path to the database file.
    pub database_path: PathBuf,
    /// The opened database (if unlocked).
    pub database: Option<KeepassDatabase>,
    /// Root group of the database.
    pub root_group: Option<Group>,
    /// Currently selected entry.
    pub selected_entry: Option<Entry>,

    // Sidebar state
    /// UUIDs of expanded groups.
    pub expanded_groups: HashSet<String>,
    /// Index of the selected item in the flattened tree.
    pub sidebar_selected_index: usize,
    /// Cached flattened tree items for rendering.
    pub tree_items: Vec<TreeItem>,

    // Password entry state
    /// Password being entered.
    pub password_input: String,
    /// Error message to display.
    pub error_message: Option<String>,

    // Search state
    /// Search query.
    pub search_query: String,
    /// Search results.
    pub search_results: Vec<SearchResult>,
    /// Selected search result index.
    pub search_selected_index: usize,
}

/// A flattened tree item for rendering.
#[derive(Debug, Clone)]
pub struct TreeItem {
    /// Depth level for indentation.
    pub depth: usize,
    /// Whether this is a group or entry.
    pub kind: TreeItemKind,
    /// UUID of the item.
    pub uuid: String,
    /// Display name.
    pub name: String,
    /// Whether group is expanded (only for groups).
    pub is_expanded: bool,
    /// Whether group has children.
    pub has_children: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeItemKind {
    Group,
    Entry,
}

/// A search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matched entry.
    pub entry: Entry,
    /// Path to the entry (for display).
    pub path: String,
    /// Parent group UUID.
    pub group_uuid: String,
    /// Match score for sorting.
    pub score: i64,
}

impl App {
    /// Create a new application instance.
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            state: AppState::Locked,
            input_mode: InputMode::PasswordEntry,
            focus: Focus::Sidebar,
            database_path,
            database: None,
            root_group: None,
            selected_entry: None,
            expanded_groups: HashSet::new(),
            sidebar_selected_index: 0,
            tree_items: Vec::new(),
            password_input: String::new(),
            error_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected_index: 0,
        }
    }

    /// Attempt to unlock the database with the current password.
    pub fn try_unlock(&mut self) -> bool {
        match KeepassDatabase::unlock(&self.database_path, &self.password_input) {
            Ok(db) => {
                let root = db.root_group();
                // Expand root by default
                self.expanded_groups.insert(root.uuid.clone());
                self.root_group = Some(root);
                self.database = Some(db);
                self.state = AppState::Unlocked;
                self.input_mode = InputMode::Normal;
                self.error_message = None;
                self.rebuild_tree();
                true
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to unlock: {}", e));
                self.password_input.clear();
                false
            }
        }
    }

    /// Rebuild the flattened tree items from the current group hierarchy.
    pub fn rebuild_tree(&mut self) {
        self.tree_items.clear();
        if let Some(root) = self.root_group.clone() {
            self.flatten_group(&root, 0);
        }
    }

    fn flatten_group(&mut self, group: &Group, depth: usize) {
        // Add folder only if not root (depth > 0) or always for non-root
        let is_expanded = self.expanded_groups.contains(&group.uuid);
        let has_children = !group.children.is_empty() || !group.entries.is_empty();

        // Skip adding root group as an item but process its children
        if depth > 0 {
            self.tree_items.push(TreeItem {
                depth,
                kind: TreeItemKind::Group,
                uuid: group.uuid.clone(),
                name: group.name.clone(),
                is_expanded,
                has_children,
            });
        }

        // If expanded (or root), add children and entries
        if is_expanded || depth == 0 {
            for child in &group.children {
                self.flatten_group(child, depth + 1);
            }
            for entry in &group.entries {
                self.tree_items.push(TreeItem {
                    depth: depth + 1,
                    kind: TreeItemKind::Entry,
                    uuid: entry.uuid.clone(),
                    name: entry.title.clone(),
                    is_expanded: false,
                    has_children: false,
                });
            }
        }
    }

    /// Toggle expansion of a group.
    pub fn toggle_expand(&mut self, uuid: &str) {
        if self.expanded_groups.contains(uuid) {
            self.expanded_groups.remove(uuid);
        } else {
            self.expanded_groups.insert(uuid.to_string());
        }
        self.rebuild_tree();
    }

    /// Select the current tree item.
    pub fn select_current_item(&mut self) {
        if let Some(item) = self.tree_items.get(self.sidebar_selected_index) {
            match item.kind {
                TreeItemKind::Group => {
                    self.toggle_expand(&item.uuid.clone());
                }
                TreeItemKind::Entry => {
                    // Find and select the entry
                    if let Some(ref db) = self.database {
                        self.selected_entry = db.find_entry(&item.uuid);
                        self.focus = Focus::EntryView;
                    }
                }
            }
        }
    }

    /// Move selection up in the sidebar.
    pub fn move_up(&mut self) {
        if self.sidebar_selected_index > 0 {
            self.sidebar_selected_index -= 1;
        }
    }

    /// Move selection down in the sidebar.
    pub fn move_down(&mut self) {
        if self.sidebar_selected_index + 1 < self.tree_items.len() {
            self.sidebar_selected_index += 1;
        }
    }

    /// Expand the current item (group) or select entry.
    pub fn expand_current(&mut self) {
        if let Some(item) = self.tree_items.get(self.sidebar_selected_index).cloned() {
            match item.kind {
                TreeItemKind::Group => {
                    if !self.expanded_groups.contains(&item.uuid) {
                        self.expanded_groups.insert(item.uuid);
                        self.rebuild_tree();
                    }
                }
                TreeItemKind::Entry => {
                    // Select the entry and show details
                    if let Some(ref db) = self.database {
                        self.selected_entry = db.find_entry(&item.uuid);
                        self.focus = Focus::EntryView;
                    }
                }
            }
        }
    }

    /// Collapse the current group.
    pub fn collapse_current(&mut self) {
        if let Some(item) = self.tree_items.get(self.sidebar_selected_index).cloned() {
            match item.kind {
                TreeItemKind::Group => {
                    if self.expanded_groups.contains(&item.uuid) {
                        self.expanded_groups.remove(&item.uuid);
                        self.rebuild_tree();
                    }
                }
                TreeItemKind::Entry => {
                    // For entries, could optionally jump to parent group
                }
            }
        }
    }

    /// Start search mode.
    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_query.clear();
        self.search_results.clear();
        self.search_selected_index = 0;
    }

    /// Exit search mode.
    pub fn exit_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.search_results.clear();
    }

    /// Perform fuzzy search on the database.
    pub fn perform_search(&mut self) {
        use fuzzy_matcher::skim::SkimMatcherV2;
        use fuzzy_matcher::FuzzyMatcher;

        self.search_results.clear();

        let Some(ref root) = self.root_group else {
            return;
        };

        if self.search_query.trim().is_empty() {
            return;
        }

        let matcher = SkimMatcherV2::default();

        // Collect all entries with their paths
        let mut items = Vec::new();
        self.collect_entries(root, "", &mut items);

        // Score and sort
        let mut scored: Vec<_> = items
            .into_iter()
            .filter_map(|(entry, path, group_uuid)| {
                let search_text = format!("{} {} {}", entry.title, entry.username, path);
                matcher
                    .fuzzy_match(&search_text, &self.search_query)
                    .map(|score| SearchResult {
                        entry,
                        path,
                        group_uuid,
                        score,
                    })
            })
            .collect();

        scored.sort_by(|a, b| b.score.cmp(&a.score));
        self.search_results = scored.into_iter().take(10).collect();
        self.search_selected_index = 0;
    }

    fn collect_entries(&self, group: &Group, path: &str, items: &mut Vec<(Entry, String, String)>) {
        let current_path = if path.is_empty() {
            group.name.clone()
        } else {
            format!("{} / {}", path, group.name)
        };

        for entry in &group.entries {
            items.push((entry.clone(), current_path.clone(), group.uuid.clone()));
        }

        for child in &group.children {
            self.collect_entries(child, &current_path, items);
        }
    }

    /// Select a search result.
    pub fn select_search_result(&mut self) {
        let Some(result) = self.search_results.get(self.search_selected_index).cloned() else {
            return;
        };

        // Expand path to the entry
        if let Some(root) = self.root_group.clone() {
            let mut path = Vec::new();
            if Self::find_path_to_entry_static(&root, &result.entry.uuid, &mut path) {
                for uuid in path {
                    self.expanded_groups.insert(uuid);
                }
            }
        }

        let entry_uuid = result.entry.uuid.clone();
        self.selected_entry = Some(result.entry);
        self.rebuild_tree();

        // Find and select the entry in the tree
        for (i, item) in self.tree_items.iter().enumerate() {
            if item.uuid == entry_uuid {
                self.sidebar_selected_index = i;
                break;
            }
        }

        self.exit_search();
        self.focus = Focus::EntryView;
    }

    fn find_path_to_entry_static(group: &Group, entry_uuid: &str, path: &mut Vec<String>) -> bool {
        path.push(group.uuid.clone());

        for entry in &group.entries {
            if entry.uuid == entry_uuid {
                return true;
            }
        }

        for child in &group.children {
            if Self::find_path_to_entry_static(child, entry_uuid, path) {
                return true;
            }
        }
        path.pop();

        false
    }
}
