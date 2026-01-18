//! Miller columns navigation view.
//!
//! Displays stacked columns that expand right as user drills down,
//! with breadcrumb fallback when space is limited.

use crate::models::{Entry, Group, NavigationPath, NavigationStep};
use gtk4::prelude::*;
use gtk4::gdk;
use relm4::prelude::*;

/// Minimum width for each column.
const COLUMN_MIN_WIDTH: i32 = 250;

/// Messages for the column view.
#[derive(Debug)]
pub enum ColumnViewInput {
    /// Set the root group data.
    SetRootGroup(Group),
    /// User selected a group in sidebar (start fresh navigation).
    SelectGroup { uuid: String, name: String, group: Group },
    /// User selected an entry.
    SelectEntry { uuid: String, entry: Entry },
    /// Navigate to a specific depth via breadcrumb.
    NavigateToDepth(usize),
    /// Toggle password visibility for the current entry.
    TogglePasswordVisible,
    /// Copy a field value to clipboard.
    CopyField(String),
    /// Add new entry.
    AddEntry,
    /// Edit current entry.
    EditEntry,
    /// Delete current entry.
    DeleteEntry,
}

/// Output messages from column view.
#[derive(Debug, Clone)]
pub enum ColumnViewOutput {
    /// User wants to add an entry.
    AddEntry,
    /// User wants to edit an entry.
    EditEntry(Entry),
    /// User wants to delete an entry.
    DeleteEntry(String),
}

/// Column view model.
pub struct ColumnView {
    /// The full group tree (for lookups).
    root_group: Option<Group>,
    /// Current navigation path.
    nav_path: NavigationPath,
    /// Currently selected group's entries.
    current_entries: Vec<Entry>,
    /// Currently selected entry details.
    selected_entry: Option<Entry>,
    /// Password visibility state.
    password_visible: bool,
}

#[relm4::component(pub)]
impl Component for ColumnView {
    type Init = ();
    type Input = ColumnViewInput;
    type Output = ColumnViewOutput;
    type CommandOutput = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_hexpand: true,
            set_vexpand: true,

            // Breadcrumb bar
            #[name = "breadcrumb_bar"]
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 4,
                set_margin_all: 8,
                add_css_class: "toolbar",

                #[watch]
                set_visible: model.nav_path.depth() > 0,
            },

            gtk4::Separator {
                #[watch]
                set_visible: model.nav_path.depth() > 0,
            },

            // Columns container
            gtk4::ScrolledWindow {
                set_hscrollbar_policy: gtk4::PolicyType::Automatic,
                set_vscrollbar_policy: gtk4::PolicyType::Never,
                set_vexpand: true,
                set_hexpand: true,

                #[name = "columns_box"]
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_vexpand: true,
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ColumnView {
            root_group: None,
            nav_path: NavigationPath::new(),
            current_entries: Vec::new(),
            selected_entry: None,
            password_visible: false,
        };

        let widgets = view_output!();

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
            ColumnViewInput::SetRootGroup(group) => {
                self.root_group = Some(group);
            }
            ColumnViewInput::SelectGroup { uuid, name, group } => {
                // Start fresh navigation from this group
                self.nav_path = NavigationPath::new();
                self.nav_path.push_group(uuid, name);
                self.current_entries = group.entries.clone();
                self.selected_entry = None;
                self.password_visible = false;
                self.rebuild_columns(widgets, &sender);
            }
            ColumnViewInput::SelectEntry { uuid, entry } => {
                // Add entry to navigation
                self.nav_path.push_entry(uuid, entry.title.clone());
                self.selected_entry = Some(entry);
                self.password_visible = false;
                self.rebuild_columns(widgets, &sender);
            }
            ColumnViewInput::NavigateToDepth(depth) => {
                self.nav_path.truncate(depth);
                if depth == 0 {
                    self.current_entries.clear();
                    self.selected_entry = None;
                } else {
                    // Check if last step is an entry - if so, clear selected_entry
                    if let Some(NavigationStep::Group { .. }) = self.nav_path.steps.last() {
                        self.selected_entry = None;
                    }
                }
                self.password_visible = false;
                self.rebuild_columns(widgets, &sender);
            }
            ColumnViewInput::TogglePasswordVisible => {
                self.password_visible = !self.password_visible;
                self.rebuild_columns(widgets, &sender);
            }
            ColumnViewInput::CopyField(value) => {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&value);
                }
            }
            ColumnViewInput::AddEntry => {
                let _ = sender.output(ColumnViewOutput::AddEntry);
            }
            ColumnViewInput::EditEntry => {
                if let Some(ref entry) = self.selected_entry {
                    let _ = sender.output(ColumnViewOutput::EditEntry(entry.clone()));
                }
            }
            ColumnViewInput::DeleteEntry => {
                if let Some(ref entry) = self.selected_entry {
                    let _ = sender.output(ColumnViewOutput::DeleteEntry(entry.uuid.clone()));
                }
            }
        }
    }
}

impl ColumnView {
    /// Rebuild all columns based on current navigation state.
    fn rebuild_columns(&self, widgets: &mut <Self as Component>::Widgets, sender: &ComponentSender<Self>) {
        // Clear existing columns
        while let Some(child) = widgets.columns_box.first_child() {
            widgets.columns_box.remove(&child);
        }

        // Clear and rebuild breadcrumbs
        while let Some(child) = widgets.breadcrumb_bar.first_child() {
            widgets.breadcrumb_bar.remove(&child);
        }

        // Build breadcrumbs
        for (i, step) in self.nav_path.steps.iter().enumerate() {
            if i > 0 {
                let sep = gtk4::Label::new(Some("›"));
                sep.add_css_class("dim-label");
                widgets.breadcrumb_bar.append(&sep);
            }

            let name = match step {
                NavigationStep::Group { name, .. } => name.clone(),
                NavigationStep::Entry { title, .. } => title.clone(),
            };

            let btn = gtk4::Button::with_label(&name);
            btn.add_css_class("flat");
            let depth = i + 1;
            let sender_clone = sender.clone();
            btn.connect_clicked(move |_| {
                sender_clone.input(ColumnViewInput::NavigateToDepth(depth));
            });
            widgets.breadcrumb_bar.append(&btn);
        }

        // Build entry list column (if we have entries)
        if !self.current_entries.is_empty() {
            let column = self.build_entry_list_column(sender);
            widgets.columns_box.append(&column);
        }

        // Build entry detail column (if entry is selected)
        if let Some(ref entry) = self.selected_entry {
            let sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
            widgets.columns_box.append(&sep);

            let column = self.build_entry_detail_column(entry, sender);
            widgets.columns_box.append(&column);
        }
    }

    /// Build the entry list column.
    fn build_entry_list_column(&self, sender: &ComponentSender<Self>) -> gtk4::Box {
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.set_width_request(COLUMN_MIN_WIDTH);
        column.set_vexpand(true);

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        toolbar.set_margin_all(8);
        toolbar.add_css_class("toolbar");

        let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
        add_btn.add_css_class("flat");
        add_btn.set_tooltip_text(Some("Add Entry"));
        let sender_clone = sender.clone();
        add_btn.connect_clicked(move |_| {
            sender_clone.input(ColumnViewInput::AddEntry);
        });
        toolbar.append(&add_btn);

        column.append(&toolbar);
        column.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        // Entry list
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);

        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("boxed-list");
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.set_margin_all(8);

        for entry in &self.current_entries {
            let row = gtk4::ListBoxRow::new();
            row.set_widget_name(&format!("entry-{}", entry.uuid));

            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            hbox.set_margin_all(12);

            let icon = gtk4::Image::from_icon_name("dialog-password-symbolic");
            hbox.append(&icon);

            let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            vbox.set_hexpand(true);

            let title = gtk4::Label::new(Some(&entry.title));
            title.set_halign(gtk4::Align::Start);
            title.add_css_class("heading");
            vbox.append(&title);

            if !entry.username.is_empty() {
                let username = gtk4::Label::new(Some(&entry.username));
                username.set_halign(gtk4::Align::Start);
                username.add_css_class("dim-label");
                vbox.append(&username);
            }

            hbox.append(&vbox);

            // Chevron to indicate selection
            let chevron = gtk4::Image::from_icon_name("go-next-symbolic");
            chevron.add_css_class("dim-label");
            hbox.append(&chevron);

            row.set_child(Some(&hbox));
            list_box.append(&row);
        }

        // Connect row activation
        let entries = self.current_entries.clone();
        let sender_clone = sender.clone();
        list_box.connect_row_activated(move |_, row| {
            if let Some(name) = row.widget_name().as_str().strip_prefix("entry-") {
                if let Some(entry) = entries.iter().find(|e| e.uuid == name) {
                    sender_clone.input(ColumnViewInput::SelectEntry {
                        uuid: entry.uuid.clone(),
                        entry: entry.clone(),
                    });
                }
            }
        });

        scrolled.set_child(Some(&list_box));
        column.append(&scrolled);

        column
    }

    /// Build the entry detail column.
    fn build_entry_detail_column(&self, entry: &Entry, sender: &ComponentSender<Self>) -> gtk4::Box {
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.set_width_request(COLUMN_MIN_WIDTH);
        column.set_hexpand(true);
        column.set_vexpand(true);

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        toolbar.set_margin_all(8);
        toolbar.add_css_class("toolbar");

        let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
        edit_btn.add_css_class("flat");
        edit_btn.set_tooltip_text(Some("Edit Entry"));
        let sender_clone = sender.clone();
        edit_btn.connect_clicked(move |_| {
            sender_clone.input(ColumnViewInput::EditEntry);
        });
        toolbar.append(&edit_btn);

        let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
        delete_btn.add_css_class("flat");
        delete_btn.set_tooltip_text(Some("Delete Entry"));
        let sender_clone = sender.clone();
        delete_btn.connect_clicked(move |_| {
            sender_clone.input(ColumnViewInput::DeleteEntry);
        });
        toolbar.append(&delete_btn);

        column.append(&toolbar);
        column.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        // Details
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let details_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        details_box.set_margin_all(24);
        details_box.set_valign(gtk4::Align::Start);

        // Title
        let title = gtk4::Label::new(Some(&entry.title));
        title.add_css_class("title-1");
        title.set_halign(gtk4::Align::Start);
        details_box.append(&title);

        // Fields
        if !entry.username.is_empty() {
            self.add_field_row(&details_box, "Username", &entry.username, false, None, sender);
        }

        if !entry.password.is_empty() {
            let display_value = if self.password_visible {
                entry.password.clone()
            } else {
                "••••••••".to_string()
            };
            self.add_field_row(&details_box, "Password", &display_value, true, Some(&entry.password), sender);
        }

        if !entry.url.is_empty() {
            self.add_field_row(&details_box, "URL", &entry.url, false, None, sender);
        }

        if !entry.notes.is_empty() {
            let notes_label = gtk4::Label::new(Some("Notes"));
            notes_label.add_css_class("dim-label");
            notes_label.set_halign(gtk4::Align::Start);
            details_box.append(&notes_label);

            let notes_text = gtk4::Label::new(Some(&entry.notes));
            notes_text.set_halign(gtk4::Align::Start);
            notes_text.set_wrap(true);
            notes_text.set_selectable(true);
            details_box.append(&notes_text);
        }

        scrolled.set_child(Some(&details_box));
        column.append(&scrolled);

        column
    }

    /// Add a field row with copy button.
    fn add_field_row(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        is_password: bool,
        real_value: Option<&str>,
        sender: &ComponentSender<Self>,
    ) {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some(label));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let value_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let value_label = gtk4::Label::new(Some(value));
        value_label.set_halign(gtk4::Align::Start);
        value_label.set_hexpand(true);
        value_label.set_selectable(true);
        value_row.append(&value_label);

        if is_password {
            let toggle_btn = gtk4::Button::from_icon_name(
                if self.password_visible { "view-conceal-symbolic" } else { "view-reveal-symbolic" }
            );
            toggle_btn.add_css_class("flat");
            toggle_btn.set_tooltip_text(Some("Toggle visibility"));
            let sender_clone = sender.clone();
            toggle_btn.connect_clicked(move |_| {
                sender_clone.input(ColumnViewInput::TogglePasswordVisible);
            });
            value_row.append(&toggle_btn);
        }

        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
        let copy_value = real_value.unwrap_or(value).to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(ColumnViewInput::CopyField(copy_value.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);
        container.append(&row);
    }
}
