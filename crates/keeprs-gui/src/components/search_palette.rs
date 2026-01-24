//! Search palette component - VSCode-style fuzzy search overlay.

use keeprs_core::{Entry, Group};
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
        path: String,  // e.g., "Root / Websites"
        score: i64,
    },
    /// An entry result.
    Entry {
        uuid: String,
        title: String,
        username: String,
        path: String,  // e.g., "Root / Websites"
        group_uuid: String,
        score: i64,
    },
}

impl SearchResult {
    fn score(&self) -> i64 {
        match self {
            SearchResult::Group { score, .. } => *score,
            SearchResult::Entry { score, .. } => *score,
        }
    }
}

/// Messages for search palette.
#[derive(Debug)]
pub enum SearchPaletteInput {
    /// Show the search palette.
    Show,
    /// Hide the search palette.
    Hide,
    /// Toggle visibility.
    Toggle,
    /// Set the database root for searching.
    SetRootGroup(Group),
    /// Query text changed.
    QueryChanged(String),
    /// Move selection up.
    SelectPrevious,
    /// Move selection down.
    SelectNext,
    /// Confirm selection.
    ConfirmSelection,
    /// Key pressed (for escape handling).
    KeyPressed(gdk::Key),
}

/// Output messages from search palette.
#[derive(Debug, Clone)]
pub enum SearchPaletteOutput {
    /// User selected a group.
    GroupSelected { uuid: String, name: String, group: Group },
    /// User selected an entry.
    EntrySelected { uuid: String, entry: Entry, group_uuid: String },
    /// Palette was closed.
    Closed,
}

/// Search palette model.
pub struct SearchPalette {
    visible: bool,
    query: String,
    root_group: Option<Group>,
    results: Vec<SearchResult>,
    selected_index: usize,
    matcher: SkimMatcherV2,
}

#[relm4::component(pub)]
impl Component for SearchPalette {
    type Init = ();
    type Input = SearchPaletteInput;
    type Output = SearchPaletteOutput;
    type CommandOutput = ();

    view! {
        #[name = "overlay"]
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_halign: gtk4::Align::Center,
            set_valign: gtk4::Align::Start,
            set_margin_top: 60,
            set_width_request: 600,

            #[watch]
            set_visible: model.visible,

            // Search box container with shadow and background
            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                add_css_class: "popover", // Gives it the floating window look
                add_css_class: "background", // Ensures opaque background
                add_css_class: "search-palette", // Custom styling

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 0,

                    // Search entry
                    #[name = "search_entry"]
                    gtk4::SearchEntry {
                        set_placeholder_text: Some("Search entries and folders..."),
                        set_margin_all: 12,

                        connect_search_changed[sender] => move |entry| {
                            sender.input(SearchPaletteInput::QueryChanged(entry.text().to_string()));
                        },

                        connect_activate[sender] => move |_| {
                            sender.input(SearchPaletteInput::ConfirmSelection);
                        },
                    },

                    // Results list
                    gtk4::ScrolledWindow {
                        set_max_content_height: 400,
                        set_propagate_natural_height: true,
                        set_hscrollbar_policy: gtk4::PolicyType::Never,

                        #[name = "results_box"]
                        gtk4::ListBox {
                            set_selection_mode: gtk4::SelectionMode::Single,
                            add_css_class: "boxed-list",

                            connect_row_activated[sender] => move |_, _| {
                                sender.input(SearchPaletteInput::ConfirmSelection);
                            },
                        }
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SearchPalette {
            visible: false,
            query: String::new(),
            root_group: None,
            results: Vec::new(),
            selected_index: 0,
            matcher: SkimMatcherV2::default(),
        };

        let widgets = view_output!();

        // Set up key controller for the search entry
        let key_controller = gtk4::EventControllerKey::new();
        let sender_clone = sender.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            sender_clone.input(SearchPaletteInput::KeyPressed(key));
            match key {
                gdk::Key::Escape => gtk4::glib::Propagation::Stop,
                gdk::Key::Up | gdk::Key::Down => gtk4::glib::Propagation::Stop,
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
            SearchPaletteInput::Show => {
                self.visible = true;
                // Manually set visibility
                widgets.overlay.set_visible(true);
                
                self.query.clear();
                self.results.clear();
                self.selected_index = 0;
                widgets.search_entry.set_text("");
                widgets.search_entry.grab_focus();
                self.rebuild_results(widgets);
            }
            SearchPaletteInput::Hide => {
                self.visible = false;
                // Manually set visibility
                widgets.overlay.set_visible(false);
                
                let _ = sender.output(SearchPaletteOutput::Closed);
            }
            SearchPaletteInput::Toggle => {
                if self.visible {
                    sender.input(SearchPaletteInput::Hide);
                } else {
                    sender.input(SearchPaletteInput::Show);
                }
            }
            SearchPaletteInput::SetRootGroup(group) => {
                self.root_group = Some(group);
            }
            SearchPaletteInput::QueryChanged(query) => {
                self.query = query;
                self.selected_index = 0;
                self.perform_search();
                self.rebuild_results(widgets);
            }
            SearchPaletteInput::SelectPrevious => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_selection(widgets);
                }
            }
            SearchPaletteInput::SelectNext => {
                if self.selected_index + 1 < self.results.len() {
                    self.selected_index += 1;
                    self.update_selection(widgets);
                }
            }
            SearchPaletteInput::ConfirmSelection => {
                if let Some(result) = self.results.get(self.selected_index) {
                    // Hide immediately
                    self.visible = false;
                    widgets.overlay.set_visible(false);
                    
                    match result {
                        SearchResult::Group { uuid, name, .. } => {
                            if let Some(ref root) = self.root_group {
                                if let Some(group) = Self::find_group_by_uuid(root, uuid) {
                                    let _ = sender.output(SearchPaletteOutput::GroupSelected {
                                        uuid: uuid.clone(),
                                        name: name.clone(),
                                        group: group.clone(),
                                    });
                                }
                            }
                        }
                        SearchResult::Entry { uuid, group_uuid, .. } => {
                            if let Some(ref root) = self.root_group {
                                if let Some(entry) = Self::find_entry_by_uuid(root, uuid) {
                                    let _ = sender.output(SearchPaletteOutput::EntrySelected {
                                        uuid: uuid.clone(),
                                        entry,
                                        group_uuid: group_uuid.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            SearchPaletteInput::KeyPressed(key) => {
                match key {
                    gdk::Key::Escape => {
                        // Directly hide to avoid round-trip latency if possible
                        sender.input(SearchPaletteInput::Hide);
                    }
                    gdk::Key::Up => {
                        sender.input(SearchPaletteInput::SelectPrevious);
                    }
                    gdk::Key::Down => {
                        sender.input(SearchPaletteInput::SelectNext);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl SearchPalette {
    /// Perform fuzzy search on the database.
    fn perform_search(&mut self) {
        self.results.clear();

        let Some(ref root) = self.root_group else {
            return;
        };

        // Collect all searchable items
        let mut items = Vec::new();
        self.collect_items(root, "", &mut items);

        // If query is empty, show recent/all items (limited)
        if self.query.is_empty() {
            self.results = items.into_iter().take(10).collect();
            return;
        }

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
                        SearchResult::Entry { uuid, title, username, path, group_uuid, .. } => {
                            SearchResult::Entry { uuid, title, username, path, group_uuid, score }
                        }
                    }
                })
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.score().cmp(&a.score()));

        // Take top results
        self.results = scored.into_iter().take(15).collect();
    }

    /// Collect all groups and entries recursively.
    fn collect_items(&self, group: &Group, path: &str, items: &mut Vec<SearchResult>) {
        let current_path = if path.is_empty() {
            group.name.clone()
        } else {
            format!("{} / {}", path, group.name)
        };

        // Add this group
        items.push(SearchResult::Group {
            uuid: group.uuid.clone(),
            name: group.name.clone(),
            path: path.to_string(),
            score: 0,
        });

        // Add entries
        for entry in &group.entries {
            items.push(SearchResult::Entry {
                uuid: entry.uuid.clone(),
                title: entry.title.clone(),
                username: entry.username.clone(),
                path: current_path.clone(),
                group_uuid: group.uuid.clone(),
                score: 0,
            });
        }

        // Recurse into children
        for child in &group.children {
            self.collect_items(child, &current_path, items);
        }
    }

    /// Rebuild the results list UI.
    fn rebuild_results(&self, widgets: &mut <Self as Component>::Widgets) {
        // Clear existing
        while let Some(row) = widgets.results_box.row_at_index(0) {
            widgets.results_box.remove(&row);
        }

        for (i, result) in self.results.iter().enumerate() {
            let row = gtk4::ListBoxRow::new();

            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            hbox.set_margin_all(12);

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

                    // Type badge
                    let badge = gtk4::Label::new(Some("Folder"));
                    badge.add_css_class("dim-label");
                    badge.add_css_class("caption");
                    hbox.append(&badge);
                }
                SearchResult::Entry { title, username, path, .. } => {
                    // Key/password icon
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
                    vbox.append(&subtitle_label);

                    hbox.append(&vbox);

                    // Type badge - no badge for entries, they're the "default"
                }
            }

            row.set_child(Some(&hbox));
            widgets.results_box.append(&row);

            // Select first item
            if i == self.selected_index {
                widgets.results_box.select_row(Some(&row));
            }
        }
    }

    /// Update selection highlight.
    fn update_selection(&self, widgets: &mut <Self as Component>::Widgets) {
        if let Some(row) = widgets.results_box.row_at_index(self.selected_index as i32) {
            widgets.results_box.select_row(Some(&row));
            // Scroll to visible
            row.grab_focus();
        }
    }

    /// Find a group by UUID.
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

    /// Find an entry by UUID.
    fn find_entry_by_uuid(group: &Group, uuid: &str) -> Option<Entry> {
        for entry in &group.entries {
            if entry.uuid == uuid {
                return Some(entry.clone());
            }
        }
        for child in &group.children {
            if let Some(found) = Self::find_entry_by_uuid(child, uuid) {
                return Some(found);
            }
        }
        None
    }
}
