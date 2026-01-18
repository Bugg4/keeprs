//! Search bar component - always visible fuzzy search.

use crate::models::{Entry, Group};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use gtk4::prelude::*;
use gtk4::gdk;
use relm4::prelude::*;

/// A search result item.
#[derive(Debug, Clone)]
pub enum SearchResult {
    /// A group/folder result.
    Group {
        uuid: String,
        name: String,
        path: String,
        score: i64,
    },
    /// An entry result.
    Entry {
        uuid: String,
        title: String,
        username: String,
        path: String,
        group_uuid: String,
        entry: Entry,
        score: i64,
    },
}

/// Messages for search bar.
#[derive(Debug)]
pub enum SearchBarInput {
    /// Set the root group for searching.
    SetRootGroup(Group),
    /// Query text changed.
    QueryChanged(String),
    /// Result selected at index.
    SelectResult(usize),
    /// Clear search and close results.
    ClearSearch,
    /// Move selection up.
    SelectPrevious,
    /// Move selection down.
    SelectNext,
    /// Confirm current selection.
    ConfirmSelection,
}

/// Output messages from search bar.
#[derive(Debug, Clone)]
pub enum SearchBarOutput {
    /// User selected a group.
    GroupSelected { uuid: String, name: String, group: Group },
    /// User selected an entry.
    EntrySelected { entry: Entry, group_uuid: String },
}

/// Search bar model.
pub struct SearchBar {
    root_group: Option<Group>,
    query: String,
    results: Vec<SearchResult>,
    selected_index: usize,
    matcher: SkimMatcherV2,
}

#[relm4::component(pub)]
impl Component for SearchBar {
    type Init = ();
    type Input = SearchBarInput;
    type Output = SearchBarOutput;
    type CommandOutput = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 0,

            // Search entry
            #[name = "search_entry"]
            gtk4::SearchEntry {
                set_placeholder_text: Some("Search..."),
                set_margin_all: 8,

                connect_search_changed[sender] => move |entry| {
                    sender.input(SearchBarInput::QueryChanged(entry.text().to_string()));
                },
            },

            // Results list (only visible when searching)
            #[name = "results_scroll"]
            gtk4::ScrolledWindow {
                set_vexpand: false,
                set_hscrollbar_policy: gtk4::PolicyType::Never,
                set_max_content_height: 300,
                set_propagate_natural_height: true,

                #[watch]
                set_visible: !model.results.is_empty(),

                #[name = "results_box"]
                gtk4::ListBox {
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk4::SelectionMode::Single,
                    set_margin_start: 8,
                    set_margin_end: 8,
                    set_margin_bottom: 8,

                    connect_row_activated[sender] => move |_, row| {
                        // Get the index from the row's name
                        if let Some(idx_str) = row.widget_name().as_str().strip_prefix("result-") {
                            if let Ok(index) = idx_str.parse::<usize>() {
                                sender.input(SearchBarInput::SelectResult(index));
                            }
                        }
                    },
                }
            },

            gtk4::Separator {
                #[watch]
                set_visible: !model.results.is_empty(),
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SearchBar {
            root_group: None,
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
            matcher: SkimMatcherV2::default(),
        };

        let widgets = view_output!();

        // Set up key controller for the search entry
        let key_controller = gtk4::EventControllerKey::new();
        let sender_clone = sender.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                gdk::Key::Escape => {
                    sender_clone.input(SearchBarInput::ClearSearch);
                    gtk4::glib::Propagation::Stop
                }
                gdk::Key::Down => {
                    sender_clone.input(SearchBarInput::SelectNext);
                    gtk4::glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    sender_clone.input(SearchBarInput::SelectPrevious);
                    gtk4::glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    sender_clone.input(SearchBarInput::ConfirmSelection);
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        widgets.search_entry.add_controller(key_controller);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            SearchBarInput::SetRootGroup(group) => {
                tracing::info!("SearchBar: SetRootGroup received, group name: {}", group.name);
                self.root_group = Some(group);
            }
            SearchBarInput::QueryChanged(query) => {
                tracing::info!("SearchBar: Query changed to: '{}'", query);
                self.query = query;
                self.selected_index = 0;
                self.perform_search();
                tracing::info!("SearchBar: Found {} results", self.results.len());
                self.rebuild_results(widgets);
            }
            SearchBarInput::SelectResult(index) => {
                self.selected_index = index;
                self.activate_selected(&sender);
                // Clear search after selection
                self.clear_search(widgets);
            }
            SearchBarInput::ClearSearch => {
                self.clear_search(widgets);
            }
            SearchBarInput::SelectPrevious => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_selection(widgets);
                }
            }
            SearchBarInput::SelectNext => {
                if !self.results.is_empty() && self.selected_index + 1 < self.results.len() {
                    self.selected_index += 1;
                    self.update_selection(widgets);
                }
            }
            SearchBarInput::ConfirmSelection => {
                if !self.results.is_empty() {
                    self.activate_selected(&sender);
                    self.clear_search(widgets);
                }
            }
        }
    }
}

impl SearchBar {
    /// Clear the search and hide results.
    fn clear_search(&mut self, widgets: &mut <Self as Component>::Widgets) {
        self.query.clear();
        self.results.clear();
        self.selected_index = 0;
        widgets.search_entry.set_text("");
        widgets.results_scroll.set_visible(false);
    }

    /// Activate the currently selected result.
    fn activate_selected(&self, sender: &ComponentSender<Self>) {
        if let Some(result) = self.results.get(self.selected_index) {
            match result {
                SearchResult::Group { uuid, name, .. } => {
                    if let Some(ref root) = self.root_group {
                        if let Some(group) = Self::find_group_by_uuid(root, uuid) {
                            let _ = sender.output(SearchBarOutput::GroupSelected {
                                uuid: uuid.clone(),
                                name: name.clone(),
                                group: group.clone(),
                            });
                        }
                    }
                }
                SearchResult::Entry { entry, group_uuid, .. } => {
                    let _ = sender.output(SearchBarOutput::EntrySelected {
                        entry: entry.clone(),
                        group_uuid: group_uuid.clone(),
                    });
                }
            }
        }
    }

    /// Update the selection highlight in the list.
    fn update_selection(&self, widgets: &mut <Self as Component>::Widgets) {
        if let Some(row) = widgets.results_box.row_at_index(self.selected_index as i32) {
            widgets.results_box.select_row(Some(&row));
        }
    }

    /// Perform fuzzy search on the database.
    fn perform_search(&mut self) {
        self.results.clear();

        let Some(ref root) = self.root_group else {
            return;
        };

        // If query is empty, clear results
        if self.query.trim().is_empty() {
            return;
        }

        // Collect all searchable items
        let mut items = Vec::new();
        self.collect_items(root, "", &mut items);

        // Fuzzy match and sort by score
        let mut scored: Vec<SearchResult> = items
            .into_iter()
            .filter_map(|item| {
                let search_text = match &item {
                    SearchResult::Group { name, path, .. } => format!("{} {}", name, path),
                    SearchResult::Entry { title, username, path, .. } => {
                        format!("{} {} {}", title, username, path)
                    }
                };

                self.matcher.fuzzy_match(&search_text, &self.query).map(|score| {
                    match item {
                        SearchResult::Group { uuid, name, path, .. } => {
                            SearchResult::Group { uuid, name, path, score }
                        }
                        SearchResult::Entry { uuid, title, username, path, group_uuid, entry, .. } => {
                            SearchResult::Entry { uuid, title, username, path, group_uuid, entry, score }
                        }
                    }
                })
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| {
            let sa = match a { SearchResult::Group { score, .. } | SearchResult::Entry { score, .. } => *score };
            let sb = match b { SearchResult::Group { score, .. } | SearchResult::Entry { score, .. } => *score };
            sb.cmp(&sa)
        });

        // Take top results
        self.results = scored.into_iter().take(10).collect();
    }

    /// Collect all groups and entries recursively.
    fn collect_items(&self, group: &Group, path: &str, items: &mut Vec<SearchResult>) {
        let current_path = if path.is_empty() {
            group.name.clone()
        } else {
            format!("{} / {}", path, group.name)
        };

        // Add this group (but not the root)
        if !path.is_empty() {
            items.push(SearchResult::Group {
                uuid: group.uuid.clone(),
                name: group.name.clone(),
                path: path.to_string(),
                score: 0,
            });
        }

        // Add entries
        for entry in &group.entries {
            items.push(SearchResult::Entry {
                uuid: entry.uuid.clone(),
                title: entry.title.clone(),
                username: entry.username.clone(),
                path: current_path.clone(),
                group_uuid: group.uuid.clone(),
                entry: entry.clone(),
                score: 0,
            });
        }

        // Recurse into children
        for child in &group.children {
            self.collect_items(child, &current_path, items);
        }
    }

    /// Rebuild the results list UI.
    fn rebuild_results(&mut self, widgets: &mut <Self as Component>::Widgets) {
        // Clear existing
        while let Some(row) = widgets.results_box.row_at_index(0) {
            widgets.results_box.remove(&row);
        }

        // Manually set visibility since #[watch] doesn't trigger on internal state changes
        let has_results = !self.results.is_empty();
        widgets.results_scroll.set_visible(has_results);

        for (i, result) in self.results.iter().enumerate() {
            let row = gtk4::ListBoxRow::new();
            row.set_widget_name(&format!("result-{}", i));

            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            hbox.set_margin_all(8);

            match result {
                SearchResult::Group { name, path, .. } => {
                    // Folder icon
                    let icon = gtk4::Image::from_icon_name("folder-symbolic");
                    icon.add_css_class("dim-label");
                    hbox.append(&icon);

                    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    vbox.set_hexpand(true);

                    let name_label = gtk4::Label::new(Some(name));
                    name_label.set_halign(gtk4::Align::Start);
                    name_label.add_css_class("heading");
                    vbox.append(&name_label);

                    if !path.is_empty() {
                        let path_label = gtk4::Label::new(Some(path));
                        path_label.set_halign(gtk4::Align::Start);
                        path_label.add_css_class("dim-label");
                        path_label.add_css_class("caption");
                        vbox.append(&path_label);
                    }

                    hbox.append(&vbox);
                }
                SearchResult::Entry { title, username, path, .. } => {
                    // Key icon
                    let icon = gtk4::Image::from_icon_name("dialog-password-symbolic");
                    hbox.append(&icon);

                    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    vbox.set_hexpand(true);

                    let title_label = gtk4::Label::new(Some(title));
                    title_label.set_halign(gtk4::Align::Start);
                    title_label.add_css_class("heading");
                    vbox.append(&title_label);

                    let subtitle = if !username.is_empty() {
                        format!("{} • {}", username, path)
                    } else {
                        path.clone()
                    };
                    let subtitle_label = gtk4::Label::new(Some(&subtitle));
                    subtitle_label.set_halign(gtk4::Align::Start);
                    subtitle_label.add_css_class("dim-label");
                    subtitle_label.add_css_class("caption");
                    subtitle_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                    vbox.append(&subtitle_label);

                    hbox.append(&vbox);
                }
            }

            row.set_child(Some(&hbox));
            widgets.results_box.append(&row);
        }

        // Select first row by default
        if !self.results.is_empty() {
            self.selected_index = 0;
            if let Some(row) = widgets.results_box.row_at_index(0) {
                widgets.results_box.select_row(Some(&row));
            }
        }
    }

    /// Find a group by UUID recursively.
    fn find_group_by_uuid(group: &Group, uuid: &str) -> Option<Group> {
        if group.uuid == uuid {
            return Some(group.clone());
        }
        for child in &group.children {
            if let Some(found) = Self::find_group_by_uuid(child, uuid) {
                return Some(found);
            }
        }
        None
    }
}
