//! Entry browser component.
//!
//! Displays entry list and entry details with breadcrumb navigation.
//! Uses a two-column layout: entry list on the left, details on the right.

use keeprs_core::{Entry, Group, NavigationPath, NavigationStep};
use gtk4::prelude::*;


use relm4::prelude::*;
use crate::components::entry_detail_view::{EntryDetailView, EntryDetailViewInput, EntryDetailViewOutput};

/// Minimum width for each column.
const COLUMN_MIN_WIDTH: i32 = 250;

/// Messages for the entry browser.
#[derive(Debug)]
pub enum EntryBrowserInput {
    /// Set the root group data.
    SetRootGroup(Group),
    /// User selected a group in sidebar (start fresh navigation).
    SelectGroup { uuid: String, name: String, group: Group },
    /// User selected an entry.
    SelectEntry { uuid: String, entry: Entry },
    /// Navigate to a specific depth via breadcrumb.
    NavigateToDepth(usize),
    /// Add new entry.
    AddEntry,
    /// Set whether we are in trash mode (enables permanent deletion).
    SetTrashMode(bool),
    /// Message from the detail view sub-component.
    DetailViewMessage(EntryDetailViewOutput),
}

/// Output messages from entry browser.
#[derive(Debug, Clone)]
pub enum EntryBrowserOutput {
    /// User wants to add an entry.
    AddEntry,
    /// User wants to delete an entry.
    DeleteEntry(String),
    /// User wants to permanently delete an entry.
    RequestPermanentDeleteEntry(String),
    /// User wants to save an attachment.
    SaveAttachment { filename: String, data: Vec<u8> },
    /// User wants to open an attachment.
    OpenAttachment { filename: String, data: Vec<u8> },
    /// Entry was edited inline and saved.
    EntryEdited(Entry),
}

/// Entry browser model.
pub struct EntryBrowser {
    /// The full group tree (for lookups).
    root_group: Option<Group>,
    /// Current navigation path.
    nav_path: NavigationPath,
    /// Currently selected group's entries.
    current_entries: Vec<Entry>,
    /// Currently selected entry details.
    selected_entry: Option<Entry>,

    /// Whether we are in trash mode (permanent deletion).
    trash_mode: bool,
    /// Controller for the entry detail view.
    detail_view: Controller<EntryDetailView>,
}

#[relm4::component(pub)]
impl Component for EntryBrowser {
    type Init = (bool, bool); // (show_entropy_bar, show_totp_visible)
    type Input = EntryBrowserInput;
    type Output = EntryBrowserOutput;
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

                #[name = "_columns_box"]
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_vexpand: true,
                }
            }
        }
    }

    fn init(
        (show_entropy_bar, show_totp_visible): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let detail_view = EntryDetailView::builder()
            .launch((show_entropy_bar, show_totp_visible))
            .forward(sender.input_sender(), EntryBrowserInput::DetailViewMessage);

        let model = EntryBrowser {
            root_group: None,
            nav_path: NavigationPath::new(),
            current_entries: Vec::new(),
            selected_entry: None,

            trash_mode: false,
            detail_view,
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
            EntryBrowserInput::SetRootGroup(group) => {
                self.root_group = Some(group);
            }
            EntryBrowserInput::SelectGroup { uuid, name, group } => {
                // Start fresh navigation from this group
                self.nav_path = NavigationPath::new();
                self.nav_path.push_group(uuid, name);
                self.current_entries = group.entries.clone();
                self.selected_entry = None;
                self.trash_mode = false;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::SelectEntry { uuid, entry } => {
                // Add entry to navigation
                self.nav_path.push_entry(uuid, entry.title.clone());
                self.selected_entry = Some(entry.clone());
                self.detail_view.emit(EntryDetailViewInput::UpdateEntry(Some(entry)));
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::NavigateToDepth(depth) => {
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
                // self.password_visible = false; // Removed
                self.rebuild_columns(widgets, &sender);
            }

            EntryBrowserInput::AddEntry => {
                let _ = sender.output(EntryBrowserOutput::AddEntry);
            }


            EntryBrowserInput::SetTrashMode(is_trash) => {
                self.trash_mode = is_trash;
                self.detail_view.emit(EntryDetailViewInput::SetTrashMode(is_trash));
                self.rebuild_columns(widgets, &sender);
            }


            EntryBrowserInput::DetailViewMessage(msg) => {
                match msg {
                    EntryDetailViewOutput::EntryEdited(entry) => {
                         // Update selected_entry with edits
                        self.selected_entry = Some(entry.clone());
                        let _ = sender.output(EntryBrowserOutput::EntryEdited(entry));
                    }
                    EntryDetailViewOutput::OpenUrl(url) => {
                        std::thread::spawn(move || {
                            if let Err(e) = std::process::Command::new("xdg-open")
                                .arg(&url)
                                .spawn()
                            {
                                tracing::error!("Failed to open URL {}: {}", url, e);
                            }
                        });
                    }
                    EntryDetailViewOutput::SaveAttachment { filename, data } => {
                         let _ = sender.output(EntryBrowserOutput::SaveAttachment { filename, data });
                    }
                    EntryDetailViewOutput::OpenAttachment { filename, data } => {
                        let _ = sender.output(EntryBrowserOutput::OpenAttachment { filename, data });
                    }
                    EntryDetailViewOutput::DeleteEntry(uuid) => {
                         let _ = sender.output(EntryBrowserOutput::DeleteEntry(uuid));
                    }
                    EntryDetailViewOutput::RequestPermanentDeleteEntry(uuid) => {
                         let _ = sender.output(EntryBrowserOutput::RequestPermanentDeleteEntry(uuid));
                    }
                }
            }
        }
    }
}

impl EntryBrowser {
    /// Rebuild all columns based on current navigation state.
    fn rebuild_columns(&self, widgets: &mut <Self as Component>::Widgets, sender: &ComponentSender<Self>) {
        // Clear existing columns
        while let Some(child) = widgets._columns_box.first_child() {
            widgets._columns_box.remove(&child);
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
                sender_clone.input(EntryBrowserInput::NavigateToDepth(depth));
            });
            widgets.breadcrumb_bar.append(&btn);
        }

        // Build entry list column (always visible to allow adding entries)
        let column = self.build_entry_list_column(sender);
        widgets._columns_box.append(&column);

        // Always show divider
        let sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
        widgets._columns_box.append(&sep);

        // Build entry detail column (managed by sub-component)
        widgets._columns_box.append(self.detail_view.widget());
    }

    /// Build the entry list column.
    fn build_entry_list_column(&self, sender: &ComponentSender<Self>) -> gtk4::Box {
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.set_width_request(COLUMN_MIN_WIDTH);
        column.set_vexpand(true);

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        toolbar.set_margin_all(8);

        let add_btn = crate::components::common::create_composite_add_button("Add Entry", "dialog-password-symbolic");

        let sender_clone = sender.clone();
        add_btn.connect_clicked(move |_| {
            sender_clone.input(EntryBrowserInput::AddEntry);
        });
        toolbar.append(&add_btn);

        column.append(&toolbar);
        column.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        // Entry list
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);

        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("navigation-sidebar");
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
            title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
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

            // Select if it matches selected entry
            if let Some(ref selected) = self.selected_entry {
                if entry.uuid == selected.uuid {
                    list_box.select_row(Some(&row));
                    let row_clone = row.clone();
                    gtk4::glib::idle_add_local(move || {
                        row_clone.grab_focus();
                        gtk4::glib::ControlFlow::Break
                    });
                }
            }
        }

        // Connect row activation
        let entries = self.current_entries.clone();
        let sender_clone = sender.clone();
        list_box.connect_row_activated(move |_, row| {
            if let Some(name) = row.widget_name().as_str().strip_prefix("entry-") {
                if let Some(entry) = entries.iter().find(|e| e.uuid == name) {
                    sender_clone.input(EntryBrowserInput::SelectEntry {
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
}
