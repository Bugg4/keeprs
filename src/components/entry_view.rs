//! Entry detail view component.

use crate::models::Entry;
use gtk4::prelude::*;
use gtk4::gdk;
use relm4::prelude::*;

/// Messages for entry view.
#[derive(Debug)]
pub enum EntryViewInput {
    /// Display an entry.
    ShowEntry(Entry),
    /// Show entries for a group.
    ShowEntries(Vec<Entry>),
    /// Select an entry from the list.
    SelectEntry(String),
    /// Toggle password visibility.
    TogglePasswordVisible,
    /// Copy field to clipboard.
    CopyField(String),
    /// User wants to edit the entry.
    EditEntry,
    /// User wants to delete the entry.
    DeleteEntry,
    /// User wants to add a new entry.
    AddEntry,
    /// Clear the view.
    Clear,
    /// Save an attachment.
    SaveAttachment(String),
}

/// Output messages from entry view.
#[derive(Debug, Clone)]
pub enum EntryViewOutput {
    /// User wants to edit an entry.
    EditEntry(Entry),
    /// User wants to delete an entry.
    DeleteEntry(String),
    /// User wants to add a new entry.
    AddEntry,
    /// User wants to save an attachment.
    SaveAttachment { filename: String, data: Vec<u8> },
}

/// Entry view model.
pub struct EntryView {
    entries: Vec<Entry>,
    selected_entry: Option<Entry>,
    password_visible: bool,
}

#[relm4::component(pub)]
impl Component for EntryView {
    type Init = ();
    type Input = EntryViewInput;
    type Output = EntryViewOutput;
    type CommandOutput = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 0,
            set_hexpand: true,
            set_vexpand: true,

            // Toolbar
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                add_css_class: "toolbar",
                set_margin_all: 8,

                gtk4::Button {
                    set_icon_name: "list-add-symbolic",
                    set_tooltip_text: Some("Add Entry"),
                    add_css_class: "flat",
                    connect_clicked => EntryViewInput::AddEntry,
                },

                gtk4::Separator {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_margin_start: 4,
                    set_margin_end: 4,
                },

                #[name = "edit_btn"]
                gtk4::Button {
                    set_icon_name: "document-edit-symbolic",
                    set_tooltip_text: Some("Edit Entry"),
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: model.selected_entry.is_some(),
                    connect_clicked => EntryViewInput::EditEntry,
                },

                #[name = "delete_btn"]
                gtk4::Button {
                    set_icon_name: "user-trash-symbolic",
                    set_tooltip_text: Some("Delete Entry"),
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: model.selected_entry.is_some(),
                    connect_clicked => EntryViewInput::DeleteEntry,
                },
            },

            gtk4::Separator {},

            // Main content area with box layout
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_vexpand: true,
                set_hexpand: true,

                // Entry list (left side - fixed width)
                gtk4::ScrolledWindow {
                    set_hscrollbar_policy: gtk4::PolicyType::Never,
                    set_vexpand: true,
                    set_width_request: 300,

                    #[name = "entry_list"]
                    gtk4::ListBox {
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk4::SelectionMode::Single,
                        set_margin_all: 12,
                    }
                },

                gtk4::Separator {
                    set_orientation: gtk4::Orientation::Vertical,
                },

                // Entry details (right side - fills remaining space)
                gtk4::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,

                    #[name = "detail_box"]
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 16,
                        set_margin_all: 24,
                        set_valign: gtk4::Align::Start,

                        #[watch]
                        set_visible: model.selected_entry.is_some(),
                    }
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = EntryView {
            entries: Vec::new(),
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
        root: &Self::Root,
    ) {
        match message {
            EntryViewInput::ShowEntries(entries) => {
                self.entries = entries;
                self.selected_entry = None;
                self.refresh_entry_list(widgets, &sender);
                self.clear_details(widgets);
            }
            EntryViewInput::ShowEntry(entry) => {
                self.selected_entry = Some(entry.clone());
                self.password_visible = false;
                self.refresh_details(widgets, &entry, &sender);
            }
            EntryViewInput::SelectEntry(uuid) => {
                if let Some(entry) = self.entries.iter().find(|e| e.uuid == uuid) {
                    sender.input(EntryViewInput::ShowEntry(entry.clone()));
                }
            }
            EntryViewInput::TogglePasswordVisible => {
                self.password_visible = !self.password_visible;
                if let Some(ref entry) = self.selected_entry {
                    self.refresh_details(widgets, entry, &sender);
                }
            }
            EntryViewInput::CopyField(value) => {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&value);
                }
            }
            EntryViewInput::EditEntry => {
                if let Some(ref entry) = self.selected_entry {
                    let _ = sender.output(EntryViewOutput::EditEntry(entry.clone()));
                }
            }
            EntryViewInput::DeleteEntry => {
                if let Some(ref entry) = self.selected_entry {
                    let _ = sender.output(EntryViewOutput::DeleteEntry(entry.uuid.clone()));
                }
            }
            EntryViewInput::AddEntry => {
                let _ = sender.output(EntryViewOutput::AddEntry);
            }
            EntryViewInput::Clear => {
                self.entries.clear();
                self.selected_entry = None;
                self.clear_details(widgets);
                // Clear list
                while let Some(row) = widgets.entry_list.row_at_index(0) {
                    widgets.entry_list.remove(&row);
                }
            }
            EntryViewInput::SaveAttachment(filename) => {
                if let Some(ref entry) = self.selected_entry {
                    if let Some(att) = entry.attachments.iter().find(|a| a.filename == filename) {
                        let _ = sender.output(EntryViewOutput::SaveAttachment {
                            filename: att.filename.clone(),
                            data: att.data.clone(),
                        });
                    }
                }
            }
        }
    }
}

impl EntryView {
    fn refresh_entry_list(&self, widgets: &mut <Self as Component>::Widgets, sender: &ComponentSender<Self>) {
        // Clear existing rows
        while let Some(row) = widgets.entry_list.row_at_index(0) {
            widgets.entry_list.remove(&row);
        }

        // Add entry rows
        for entry in &self.entries {
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
            row.set_child(Some(&hbox));

            let uuid = entry.uuid.clone();
            let sender_clone = sender.clone();
            widgets.entry_list.connect_row_activated(move |_, activated_row| {
                if let Some(name) = activated_row.widget_name().as_str().strip_prefix("entry-") {
                    sender_clone.input(EntryViewInput::SelectEntry(name.to_string()));
                }
            });

            widgets.entry_list.append(&row);
        }
    }

    fn refresh_details(&self, widgets: &mut <Self as Component>::Widgets, entry: &Entry, sender: &ComponentSender<Self>) {
        self.clear_details(widgets);

        // Title
        let title = gtk4::Label::new(Some(&entry.title));
        title.add_css_class("title-1");
        title.set_halign(gtk4::Align::Start);
        widgets.detail_box.append(&title);

        // Field rows
        if !entry.username.is_empty() {
            self.add_field_row(widgets, "Username", &entry.username, false, sender);
        }

        if !entry.password.is_empty() {
            let display_value = if self.password_visible {
                entry.password.clone()
            } else {
                "••••••••".to_string()
            };
            self.add_field_row(widgets, "Password", &display_value, true, sender);
        }

        if !entry.url.is_empty() {
            self.add_field_row(widgets, "URL", &entry.url, false, sender);
        }

        if !entry.notes.is_empty() {
            let notes_label = gtk4::Label::new(Some("Notes"));
            notes_label.add_css_class("dim-label");
            notes_label.set_halign(gtk4::Align::Start);
            widgets.detail_box.append(&notes_label);

            let notes_text = gtk4::Label::new(Some(&entry.notes));
            notes_text.set_halign(gtk4::Align::Start);
            notes_text.set_wrap(true);
            notes_text.set_selectable(true);
            widgets.detail_box.append(&notes_text);
        }

        if !entry.attachments.is_empty() {
            let att_label = gtk4::Label::new(Some("Attachments"));
            att_label.add_css_class("dim-label");
            att_label.set_halign(gtk4::Align::Start);
            att_label.set_margin_top(12);
            widgets.detail_box.append(&att_label);

            let att_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
            
            for att in &entry.attachments {
                let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                
                let icon = gtk4::Image::from_icon_name("mail-attachment-symbolic");
                row.append(&icon);

                let name_label = gtk4::Label::new(Some(&att.filename));
                name_label.set_hexpand(true);
                name_label.set_halign(gtk4::Align::Start);
                row.append(&name_label);

                // Size
                let size_kb = att.data.len() as f64 / 1024.0;
                let size_label = gtk4::Label::new(Some(&format!("{:.1} KB", size_kb)));
                size_label.add_css_class("dim-label");
                row.append(&size_label);

                let save_btn = gtk4::Button::from_icon_name("document-save-symbolic");
                save_btn.add_css_class("flat");
                save_btn.set_tooltip_text(Some("Save Attachment"));
                let filename = att.filename.clone();
                let sender_clone = sender.clone();
                save_btn.connect_clicked(move |_| {
                    sender_clone.input(EntryViewInput::SaveAttachment(filename.clone()));
                });
                row.append(&save_btn);

                att_box.append(&row);
            }
            widgets.detail_box.append(&att_box);
        }
    }

    fn add_field_row(
        &self,
        widgets: &mut <Self as Component>::Widgets,
        label: &str,
        value: &str,
        is_password: bool,
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
                sender_clone.input(EntryViewInput::TogglePasswordVisible);
            });
            value_row.append(&toggle_btn);
        }

        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
        let copy_value = if is_password && !self.password_visible {
            if let Some(ref entry) = self.selected_entry {
                entry.password.clone()
            } else {
                value.to_string()
            }
        } else {
            value.to_string()
        };
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryViewInput::CopyField(copy_value.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);
        widgets.detail_box.append(&row);
    }

    fn clear_details(&self, widgets: &mut <Self as Component>::Widgets) {
        while let Some(child) = widgets.detail_box.first_child() {
            widgets.detail_box.remove(&child);
        }
    }
}
