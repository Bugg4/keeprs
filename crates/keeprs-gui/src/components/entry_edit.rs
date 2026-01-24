//! Entry edit dialog component.

use keeprs_core::Entry;
use gtk4::prelude::*;
use relm4::prelude::*;

/// Messages for entry edit dialog.
#[derive(Debug)]
pub enum EntryEditInput {
    /// Open dialog to add a new entry.
    AddNew,
    /// Open dialog to edit an existing entry.
    Edit(Entry),
    /// Title changed.
    TitleChanged(String),
    /// Username changed.
    UsernameChanged(String),
    /// Password changed.
    PasswordChanged(String),
    /// URL changed.
    UrlChanged(String),
    /// Notes changed.
    NotesChanged(String),
    /// Save the entry.
    Save,
    /// Cancel editing.
    Cancel,
}

/// Output messages from entry edit dialog.
#[derive(Debug, Clone)]
pub enum EntryEditOutput {
    /// Entry was saved.
    Saved(Entry),
    /// Dialog was cancelled.
    Cancelled,
}

/// Entry edit model.
pub struct EntryEdit {
    entry: Entry,
    is_new: bool,
    visible: bool,
}

#[relm4::component(pub)]
impl Component for EntryEdit {
    type Init = ();
    type Input = EntryEditInput;
    type Output = EntryEditOutput;
    type CommandOutput = ();

    view! {
        #[name = "dialog"]
        gtk4::Window {
            set_modal: true,
            set_default_width: 450,
            set_default_height: 500,
            #[watch]
            set_title: Some(if model.is_new { "Add Entry" } else { "Edit Entry" }),
            #[watch]
            set_visible: model.visible,

            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 0,

                // Header bar
                gtk4::HeaderBar {
                    set_show_title_buttons: false,

                    #[wrap(Some)]
                    set_title_widget = &gtk4::Label {
                        #[watch]
                        set_text: if model.is_new { "Add Entry" } else { "Edit Entry" },
                        add_css_class: "title",
                    },

                    pack_start = &gtk4::Button {
                        set_label: "Cancel",
                        connect_clicked => EntryEditInput::Cancel,
                    },

                    pack_end = &gtk4::Button {
                        set_label: "Save",
                        add_css_class: "suggested-action",
                        connect_clicked => EntryEditInput::Save,
                    },
                },

                gtk4::ScrolledWindow {
                    set_vexpand: true,
                    set_hexpand: true,

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 16,
                        set_margin_all: 24,

                        // Title field
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_text: "Title",
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                            },

                            #[name = "title_entry"]
                            gtk4::Entry {
                                set_placeholder_text: Some("Entry title"),
                                #[watch]
                                set_text: &model.entry.title,
                                connect_changed[sender] => move |entry| {
                                    sender.input(EntryEditInput::TitleChanged(entry.text().to_string()));
                                },
                            },
                        },

                        // Username field
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_text: "Username",
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                            },

                            #[name = "username_entry"]
                            gtk4::Entry {
                                set_placeholder_text: Some("Username"),
                                #[watch]
                                set_text: &model.entry.username,
                                connect_changed[sender] => move |entry| {
                                    sender.input(EntryEditInput::UsernameChanged(entry.text().to_string()));
                                },
                            },
                        },

                        // Password field
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_text: "Password",
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                            },

                            #[name = "password_entry"]
                            gtk4::PasswordEntry {
                                set_placeholder_text: Some("Password"),
                                set_show_peek_icon: true,
                                #[watch]
                                set_text: &model.entry.password,
                                connect_changed[sender] => move |entry| {
                                    sender.input(EntryEditInput::PasswordChanged(entry.text().to_string()));
                                },
                            },
                        },

                        // URL field
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_text: "URL",
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                            },

                            #[name = "url_entry"]
                            gtk4::Entry {
                                set_placeholder_text: Some("https://example.com"),
                                #[watch]
                                set_text: &model.entry.url,
                                connect_changed[sender] => move |entry| {
                                    sender.input(EntryEditInput::UrlChanged(entry.text().to_string()));
                                },
                            },
                        },

                        // Notes field
                        gtk4::Box {
                            set_orientation: gtk4::Orientation::Vertical,
                            set_spacing: 4,

                            gtk4::Label {
                                set_text: "Notes",
                                set_halign: gtk4::Align::Start,
                                add_css_class: "dim-label",
                            },

                            gtk4::Frame {
                                set_height_request: 100,

                                #[name = "notes_view"]
                                gtk4::TextView {
                                    set_wrap_mode: gtk4::WrapMode::Word,
                                    set_margin_all: 8,
                                },
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = EntryEdit {
            entry: Entry::new(),
            is_new: true,
            visible: false,
        };

        let widgets = view_output!();

        // Set up notes buffer change handler
        let buffer = widgets.notes_view.buffer();
        let sender_clone = sender.clone();
        buffer.connect_changed(move |buf| {
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
            sender_clone.input(EntryEditInput::NotesChanged(text.to_string()));
        });

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
            EntryEditInput::AddNew => {
                self.entry = Entry::new();
                self.is_new = true;
                self.visible = true;
                widgets.notes_view.buffer().set_text("");
                widgets.title_entry.set_text("");
                widgets.username_entry.set_text("");
                widgets.password_entry.set_text("");
                widgets.url_entry.set_text("");
                widgets.dialog.present();
            }
            EntryEditInput::Edit(entry) => {
                widgets.notes_view.buffer().set_text(&entry.notes);
                widgets.title_entry.set_text(&entry.title);
                widgets.username_entry.set_text(&entry.username);
                widgets.password_entry.set_text(&entry.password);
                widgets.url_entry.set_text(&entry.url);
                self.entry = entry;
                self.is_new = false;
                self.visible = true;
                widgets.dialog.present();
            }
            EntryEditInput::TitleChanged(title) => {
                self.entry.title = title;
            }
            EntryEditInput::UsernameChanged(username) => {
                self.entry.username = username;
            }
            EntryEditInput::PasswordChanged(password) => {
                self.entry.password = password;
            }
            EntryEditInput::UrlChanged(url) => {
                self.entry.url = url;
            }
            EntryEditInput::NotesChanged(notes) => {
                self.entry.notes = notes;
            }
            EntryEditInput::Save => {
                self.visible = false;
                widgets.dialog.set_visible(false);
                let _ = sender.output(EntryEditOutput::Saved(self.entry.clone()));
            }
            EntryEditInput::Cancel => {
                self.visible = false;
                widgets.dialog.set_visible(false);
                let _ = sender.output(EntryEditOutput::Cancelled);
            }
        }
    }
}
