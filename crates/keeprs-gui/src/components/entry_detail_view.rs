//! Entry detail view component.
//!
//! Handles displaying entry details and inline editing.

use keeprs_core::Entry;
use gtk4::prelude::*;
use gtk4::gdk;

use gtk4::cairo::Context;
use keepass::db::TOTP;
use relm4::prelude::*;
use std::rc::Rc;

use zxcvbn::{zxcvbn, Score};
use crate::components::common;

/// Minimum width for the column.
const COLUMN_MIN_WIDTH: i32 = 250;
const PASSWORD_MASK: &str = "••••••••";
const TOTP_MASK: &str = "••••••";

#[derive(Debug)]
pub enum EntryDetailViewInput {
    /// Update the displayed entry (or clear it).
    UpdateEntry(Option<Entry>),
    /// Enter inline edit mode.
    EditEntry,
    /// Set trash mode
    SetTrashMode(bool),
    /// Exit edit mode (true = save, false = cancel).
    ExitEditMode(bool),
    /// Toggle password visibility.
    TogglePasswordVisible,
    /// Toggle TOTP visibility.
    ToggleTotpVisible,
    /// Copy a field value.
    CopyField(String),
    /// Edit title.
    EditTitle(String),
    /// Edit username.
    EditUsername(String),
    /// Edit password.
    EditPassword(String),
    /// Edit URL.
    EditUrl(String),
    /// Edit notes.
    EditNotes(String),
    /// Favicon fetched (bytes).
    FaviconFetched(Option<Vec<u8>>),
}

#[derive(Debug, Clone)]
pub enum EntryDetailViewOutput {
    /// Entry was edited and saved.
    EntryEdited(Entry),
    /// Request deletion of an entry.
    DeleteEntry(String),
    /// Delete the entry permanently.
    RequestPermanentDeleteEntry(String),
    /// Restore entry from trash.
    RestoreEntry(String),
    /// Save attachment.
    SaveAttachment { filename: String, data: Vec<u8> },
    /// Open attachment.
    OpenAttachment { filename: String, data: Vec<u8> },
    /// Open URL.
    OpenUrl(String),
}

pub struct EntryDetailView {
    entry: Option<Entry>,
    editing: bool,
    edited_entry: Option<Entry>,
    password_visible: bool,
    totp_visible: bool,
    show_entropy_bar: bool,
    show_totp_default: bool,
    trash_mode: bool,
    favicon: Option<gdk::Texture>,
}


#[relm4::component(pub)]
impl Component for EntryDetailView {
    type Init = (bool, bool); // (show_entropy_bar, show_totp_default)
    type Input = EntryDetailViewInput;
    type Output = EntryDetailViewOutput;
    type CommandOutput = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_width_request: COLUMN_MIN_WIDTH,
            set_hexpand: true,
            set_vexpand: true,

            #[name = "_content_box"]
            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_hexpand: true,
                set_vexpand: true,
            }
        }
    }

    fn init(
        (show_entropy_bar, show_totp_default): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = EntryDetailView {
            entry: None,
            editing: false,
            edited_entry: None,
            password_visible: false,
            totp_visible: show_totp_default,
            show_entropy_bar,
            show_totp_default,
            trash_mode: false,
            favicon: None,
        };

        let widgets = view_output!();
        
        // Initial build (empty state)
        model.rebuild_view(&widgets, &sender);

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
            EntryDetailViewInput::UpdateEntry(entry) => {
                self.entry = entry;
                self.editing = false;
                self.edited_entry = None;
                self.password_visible = false;
                self.totp_visible = self.show_totp_default;
                self.favicon = None;
                
                // Fetch favicon if URL exists
                if let Some(ref e) = self.entry {
                    if !e.url.is_empty() {
                         let url_str = e.url.clone();
                         let sender_clone = sender.clone();
                         std::thread::spawn(move || {
                             let favicon_url = format!("https://www.google.com/s2/favicons?domain_url={}&sz=64", url_str);
                             let result = reqwest::blocking::get(&favicon_url)
                                 .ok()
                                 .and_then(|resp| resp.bytes().ok())
                                 .map(|b| b.to_vec());
                             sender_clone.input(EntryDetailViewInput::FaviconFetched(result));
                         });
                    }
                }

                self.rebuild_view(widgets, &sender);
            }

            EntryDetailViewInput::SetTrashMode(mode) => {
                self.trash_mode = mode;
                self.rebuild_view(widgets, &sender);
            }

            EntryDetailViewInput::EditEntry => {
                if let Some(ref entry) = self.entry {
                    self.editing = true;
                    self.edited_entry = Some(entry.clone());
                    self.rebuild_view(widgets, &sender);
                }
            }
            EntryDetailViewInput::ExitEditMode(save) => {
                if save {
                    if let Some(ref edited) = self.edited_entry {
                        self.entry = Some(edited.clone());
                        let _ = sender.output(EntryDetailViewOutput::EntryEdited(edited.clone()));
                    }
                }
                self.editing = false;
                self.edited_entry = None;
                self.rebuild_view(widgets, &sender);
            }
             EntryDetailViewInput::TogglePasswordVisible => {
                self.password_visible = !self.password_visible;
                self.rebuild_view(widgets, &sender);
            }
            EntryDetailViewInput::ToggleTotpVisible => {
                self.totp_visible = !self.totp_visible;
                self.rebuild_view(widgets, &sender);
            }
            EntryDetailViewInput::CopyField(value) => {
                 if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&value);
                }
            }
            EntryDetailViewInput::EditTitle(title) => {
                if let Some(ref mut entry) = self.edited_entry {
                    entry.title = title;
                }
            }
            EntryDetailViewInput::EditUsername(username) => {
                 if let Some(ref mut entry) = self.edited_entry {
                    entry.username = username;
                }
            }
            EntryDetailViewInput::EditPassword(password) => {
                 if let Some(ref mut entry) = self.edited_entry {
                    entry.password = password;
                }
            }
            EntryDetailViewInput::EditUrl(url) => {
                 if let Some(ref mut entry) = self.edited_entry {
                    entry.url = url;
                }
            }
             EntryDetailViewInput::EditNotes(notes) => {
                 if let Some(ref mut entry) = self.edited_entry {
                    entry.notes = notes;
                }
            }
            EntryDetailViewInput::FaviconFetched(data) => {
                if let Some(bytes) = data {
                    let bytes = gdk::glib::Bytes::from(&bytes);
                    self.favicon = gdk::Texture::from_bytes(&bytes).ok();
                    if !self.editing {
                        self.rebuild_view(widgets, &sender);
                    }
                }
            }
        }
    }
}

impl EntryDetailView {
    fn rebuild_view(&self, widgets: &EntryDetailViewWidgets, sender: &ComponentSender<Self>) {
        // Clear existing content
        while let Some(child) = widgets._content_box.first_child() {
            widgets._content_box.remove(&child);
        }

        if let Some(ref entry) = self.entry {
            let column = self.build_entry_detail_column(entry, sender);
            widgets._content_box.append(&column);
        } else {
            let column = self.build_empty_state_column();
            widgets._content_box.append(&column);
        }
    }

    fn build_empty_state_column(&self) -> gtk4::Box {
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.set_hexpand(true);
        column.set_vexpand(true);
        column.set_valign(gtk4::Align::Center);
        column.set_halign(gtk4::Align::Center);

        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        vbox.set_halign(gtk4::Align::Center);
        
        // Icon
        let icon = gtk4::Image::from_icon_name("text-x-generic-symbolic");
        icon.set_pixel_size(64);
        icon.add_css_class("dim-label");
        vbox.append(&icon);

        // Label
        let label = gtk4::Label::new(None);
        label.set_markup("<i>No entry selected</i>");
        label.add_css_class("dim-label");
        vbox.append(&label);

        column.append(&vbox);
        column
    }

    // ... methods to be copied from EntryBrowser ...
    // Note: I will fill these in the subsequent edits to avoid a massive file write block.
    // For now I define the struct and basic view.
    fn build_entry_detail_column(&self, entry: &Entry, sender: &ComponentSender<Self>) -> gtk4::Box {
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.set_hexpand(true);
        column.set_vexpand(true);

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        toolbar.set_margin_all(8);

        if self.editing {
            let save_btn = gtk4::Button::with_label("Save");
            save_btn.add_css_class("suggested-action");
            let sender_clone = sender.clone();
            save_btn.connect_clicked(move |_| {
                sender_clone.input(EntryDetailViewInput::ExitEditMode(true));
            });
            toolbar.append(&save_btn);

            let cancel_btn = gtk4::Button::with_label("Cancel");
            let sender_clone = sender.clone();
            cancel_btn.connect_clicked(move |_| {
                sender_clone.input(EntryDetailViewInput::ExitEditMode(false));
            });
            toolbar.append(&cancel_btn);
        } else {
            let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
            edit_btn.add_css_class("flat");
            edit_btn.set_tooltip_text(Some("Edit Entry"));
            let sender_clone = sender.clone();
            edit_btn.connect_clicked(move |_| {
                sender_clone.input(EntryDetailViewInput::EditEntry);
            });
            toolbar.append(&edit_btn);

            // Spacer
            let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            spacer.set_hexpand(true);
            toolbar.append(&spacer);

            // Restore button
            if self.trash_mode {
                let restore_btn = gtk4::Button::from_icon_name("edit-undo-symbolic");
                restore_btn.set_tooltip_text(Some("Restore Entry"));
                restore_btn.add_css_class("suggested-action");
                
                let sender_clone = sender.clone();
                let uuid = entry.uuid.to_string();
                restore_btn.connect_clicked(move |_| {
                    sender_clone.output(EntryDetailViewOutput::RestoreEntry(uuid.clone())).unwrap();
                });
                toolbar.append(&restore_btn);
            }

            // Delete / Permanent Delete
            let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
            if self.trash_mode {
                 delete_btn.set_tooltip_text(Some("Delete Permanently"));
                 delete_btn.add_css_class("destructive-action");
            } else {
                 delete_btn.set_tooltip_text(Some("Delete Entry"));
                 delete_btn.add_css_class("flat");
            }
            
            let sender_clone = sender.clone();
            let uuid = entry.uuid.to_string();
            let trash_mode = self.trash_mode;
            delete_btn.connect_clicked(move |_| {
                if trash_mode {
                    sender_clone.output(EntryDetailViewOutput::RequestPermanentDeleteEntry(uuid.clone())).unwrap();
                } else {
                    sender_clone.output(EntryDetailViewOutput::DeleteEntry(uuid.clone())).unwrap();
                }
            });
            toolbar.append(&delete_btn);
        }
        column.append(&toolbar);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let details_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        details_box.set_margin_all(24);

        if self.editing {
             if let Some(ref edited) = self.edited_entry {
                self.add_edit_field(&details_box, "Title", &edited.title, sender, |_, t| EntryDetailViewInput::EditTitle(t));
                self.add_edit_field(&details_box, "Username", &edited.username, sender, |_, t| EntryDetailViewInput::EditUsername(t));
                self.add_password_edit_field(&details_box, "Password", &edited.password, sender);
                self.add_edit_field(&details_box, "URL", &edited.url, sender, |_, t| EntryDetailViewInput::EditUrl(t));
                self.add_notes_edit_field(&details_box, "Notes", &edited.notes, sender);
             }
        } else {
            // Title
            let title = gtk4::Label::new(Some(&entry.title));
            title.add_css_class("title-1");
            title.set_halign(gtk4::Align::Start);
            title.set_selectable(true);
            details_box.append(&title);

             // Username
            if !entry.username.is_empty() {
                self.add_field_row(&details_box, "Username", &entry.username, false, None, sender);
            }

            // Password
            if !entry.password.is_empty() {
                self.add_password_row(&details_box, &entry.password, sender);
            }

            // URL
            if !entry.url.is_empty() {
                self.add_url_row(&details_box, &entry.url, sender);
            }

             // TOTP
             if let Some(ref totp_uri) = entry.otp {
                if !totp_uri.is_empty() {
                    self.add_totp_row(&details_box, totp_uri, sender);
                }
            }

            // Notes
            if !entry.notes.is_empty() {
                self.add_text_row(&details_box, "Notes", &entry.notes);
            }

            // Attachments
             if !entry.attachments.is_empty() {
                let att_label = gtk4::Label::new(Some("Attachments"));
                att_label.add_css_class("title-3");
                att_label.set_halign(gtk4::Align::Start);
                att_label.set_margin_top(16);
                details_box.append(&att_label);

                let att_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
                for attachment in &entry.attachments {
                    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    
                    let name_label = gtk4::Label::new(Some(&attachment.filename));
                    name_label.set_hexpand(true);
                    name_label.set_halign(gtk4::Align::Start);
                    row.append(&name_label);

                    let save_btn = gtk4::Button::from_icon_name("document-save-symbolic");
                    save_btn.add_css_class("flat");
                    save_btn.set_tooltip_text(Some("Save Attachment"));
                    let sender_clone = sender.clone();
                    let name_clone = attachment.filename.clone();
                    let data_clone = attachment.data.clone();
                    save_btn.connect_clicked(move |_| {
                        sender_clone.output(EntryDetailViewOutput::SaveAttachment { filename: name_clone.clone(), data: data_clone.clone() }).unwrap();
                    });
                    row.append(&save_btn);

                    let open_btn = gtk4::Button::from_icon_name("document-open-symbolic");
                    open_btn.add_css_class("flat");
                    open_btn.set_tooltip_text(Some("Open Attachment"));
                    let sender_clone = sender.clone();
                    let name_clone = attachment.filename.clone();
                    let data_clone = attachment.data.clone();
                    open_btn.connect_clicked(move |_| {
                         sender_clone.output(EntryDetailViewOutput::OpenAttachment { filename: name_clone.clone(), data: data_clone.clone() }).unwrap();
                    });
                    row.append(&open_btn);

                    att_box.append(&row);
                }
                details_box.append(&att_box);
             }
        }

        scrolled.set_child(Some(&details_box));
        column.append(&scrolled);

        column
    }

     fn add_edit_field<F>(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        sender: &ComponentSender<Self>,
        make_input: F,
    )
    where
        F: Fn(&ComponentSender<Self>, String) -> EntryDetailViewInput + 'static,
    {
        let (row, entry) = common::create_text_entry_row(label, value);
        let sender_clone = sender.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            sender_clone.input(make_input(&sender_clone, text));
        });
        container.append(&row);
    }

     fn add_password_edit_field(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        sender: &ComponentSender<Self>,
    ) {
        let (row, entry) = common::create_password_entry_row(label, value);
        let sender_clone = sender.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            sender_clone.input(EntryDetailViewInput::EditPassword(text));
        });
        container.append(&row);
    }

    fn add_notes_edit_field(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        sender: &ComponentSender<Self>,
    ) {
         let (row, text_view) = common::create_text_area_row(label, value);
         text_view.add_css_class("monospace");
         let sender_clone = sender.clone();
         text_view.buffer().connect_changed(move |buf| {
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            sender_clone.input(EntryDetailViewInput::EditNotes(text));
        });
        container.append(&row);
    }

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
             let sender_clone = sender.clone();
            toggle_btn.connect_clicked(move |_| {
                sender_clone.input(EntryDetailViewInput::TogglePasswordVisible);
            });
            value_row.append(&toggle_btn);
        }

        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        let copy_value = real_value.unwrap_or(value).to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryDetailViewInput::CopyField(copy_value.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);
        container.append(&row);
    }

     fn add_password_row(
        &self,
        container: &gtk4::Box,
        password: &str,
        sender: &ComponentSender<Self>,
    ) {
         let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some("Password"));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let value_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let value_label = gtk4::Label::new(None);
        value_label.set_halign(gtk4::Align::Start);
        value_label.set_hexpand(true);
        value_label.set_selectable(true);

        if self.password_visible {
            value_label.set_text(password);
            value_label.add_css_class("monospace");
        } else {
            value_label.set_text(PASSWORD_MASK);
            value_label.remove_css_class("monospace");
        }
        
        value_row.append(&value_label);

        let toggle_btn = gtk4::Button::from_icon_name(
            if self.password_visible { "view-conceal-symbolic" } else { "view-reveal-symbolic" }
        );
        toggle_btn.add_css_class("flat");
        let sender_clone = sender.clone();
        toggle_btn.connect_clicked(move |_| {
             sender_clone.input(EntryDetailViewInput::TogglePasswordVisible);
        });
        value_row.append(&toggle_btn);

        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        let password_clone = password.to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryDetailViewInput::CopyField(password_clone.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);

         // Entropy bar (only if enabled in config)
        if self.show_entropy_bar {
            let (score, _guesses, _, strength_class) = Self::get_password_strength(password);

            let entropy_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            entropy_row.set_margin_top(2);

            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_hexpand(true);
            progress_bar.set_valign(gtk4::Align::Center);
            
            // 0-4 score -> fraction 0.25, 0.5, 0.75, 1.0 (with slight min for 0)
            let fraction = if score == 0 { 0.1 } else { score as f64 / 4.0 };
            progress_bar.set_fraction(fraction);
            
            // Add style class for color
             progress_bar.add_css_class(strength_class);

            entropy_row.append(&progress_bar);
            row.append(&entropy_row);
        }

        container.append(&row);
    }

     fn get_password_strength(password: &str) -> (u8, f64, &'static str, &'static str) {
        if password.is_empty() {
            return (0, 0.0, "Empty", "error");
        }
        let entropy = zxcvbn(password, &[]);
        let score = entropy.score();
        let guesses_log10 = entropy.guesses_log10();

         let (score_num, label, css_class) = match score {
            Score::Zero => (0, "Very Weak", "error"),
            Score::One => (1, "Weak", "error"),
            Score::Two => (2, "Fair", "warning"),
            Score::Three => (3, "Strong", "success"),
            Score::Four => (4, "Very Strong", "success"),
            _ => (2, "Unknown", "warning"),
        };
        (score_num, guesses_log10, label, css_class)
    }

    fn add_url_row(
        &self,
        container: &gtk4::Box,
        url: &str,
        sender: &ComponentSender<Self>,
    ) {
         let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some("URL"));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let value_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        // Favicon
        let favicon_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        favicon_box.add_css_class("favicon-box"); 
        let favicon = gtk4::Image::new();
        favicon.set_pixel_size(16);
        
        if let Some(ref texture) = self.favicon {
             favicon.set_paintable(Some(texture));
             favicon.set_visible(true);
        } else {
             // Placeholder or hidden?
             // Use a default globe icon if no favicon yet?
             // Or just hide.
             // Let's use world icon as placeholder
             favicon.set_icon_name(Some("network-server-symbolic"));
             favicon.set_visible(true);
        }
       
        favicon_box.append(&favicon);
        value_row.append(&favicon_box);

        let value_label = gtk4::Label::new(Some(url));
        value_label.set_halign(gtk4::Align::Start);
        value_label.set_hexpand(true);
        value_label.set_selectable(true);
        value_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        value_row.append(&value_label);
        
        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        let url_clone = url.to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryDetailViewInput::CopyField(url_clone.clone()));
        });
        value_row.append(&copy_btn);

        let open_btn = gtk4::Button::from_icon_name("document-open-symbolic");
        open_btn.add_css_class("flat");
        let url_clone = url.to_string();
        let sender_clone = sender.clone();
        open_btn.connect_clicked(move |_| {
            sender_clone.output(EntryDetailViewOutput::OpenUrl(url_clone.clone())).unwrap();
        });
        value_row.append(&open_btn);

        row.append(&value_row);
        container.append(&row);
    }

    fn add_totp_row(
        &self,
        container: &gtk4::Box,
        totp_uri: &str,
        sender: &ComponentSender<Self>,
    ) {
        if let Ok(totp) = totp_uri.parse::<TOTP>() {
            let totp = Rc::new(totp);
            let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

            let label_widget = gtk4::Label::new(Some("TOTP"));
            label_widget.add_css_class("dim-label");
            label_widget.set_halign(gtk4::Align::Start);
            row.append(&label_widget);

            let value_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

            let code_label = gtk4::Label::new(None);
            code_label.set_halign(gtk4::Align::Start);
            code_label.set_hexpand(true);
            code_label.set_selectable(self.totp_visible);
            
            // Initial text
             if let Ok(code) = totp.value_now() {
                if self.totp_visible {
                     code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", code.code));
                } else {
                     code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", TOTP_MASK));
                }
             }
            value_row.append(&code_label);

            // Drawing Area for Progress
            let drawing_area = gtk4::DrawingArea::new();
            drawing_area.set_content_width(24);
            drawing_area.set_content_height(24);
            drawing_area.set_margin_end(8);
            drawing_area.set_has_tooltip(true);

             // Initial tooltip
            if let Ok(_code) = totp.value_now() {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let remaining = totp.period - (now % totp.period);
                drawing_area.set_tooltip_text(Some(&format!("{}s remaining", remaining)));
            }
             
            let totp_draw = totp.clone();
            drawing_area.set_draw_func(move |area: &gtk4::DrawingArea, cr: &Context, width: i32, height: i32| {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let period = totp_draw.period;
                if period == 0 { return; }
                
                let remaining = period - (now % period);
                let progress = remaining as f64 / period as f64;

                let center_x = width as f64 / 2.0;
                let center_y = height as f64 / 2.0;
                let radius = f64::min(center_x, center_y);

                let _style_context = area.style_context();
                
                 // Background (gray)
                let (bg_r, bg_g, bg_b) = (0.85, 0.85, 0.85);
                cr.set_source_rgba(bg_r, bg_g, bg_b, 1.0);
                cr.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
                cr.fill().expect("Invalid cairo surface state");

                // Progress (blue)
                let (acc_r, acc_g, acc_b) = (0.2, 0.6, 1.0);
                cr.set_source_rgba(acc_r, acc_g, acc_b, 1.0);
                cr.move_to(center_x, center_y);
                let start_angle = -std::f64::consts::PI / 2.0;
                let end_angle = start_angle + (2.0 * std::f64::consts::PI * progress);
                cr.arc(center_x, center_y, radius, start_angle, end_angle);
                cr.close_path();
                cr.fill().expect("Invalid cairo surface state");
            });
            value_row.append(&drawing_area);

             // Toggle visibility button
            let toggle_btn = gtk4::Button::from_icon_name(
                if self.totp_visible { "view-conceal-symbolic" } else { "view-reveal-symbolic" }
            );
            toggle_btn.add_css_class("flat");
            let sender_toggle = sender.clone();
            toggle_btn.connect_clicked(move |_| {
                sender_toggle.input(EntryDetailViewInput::ToggleTotpVisible);
            });
            value_row.append(&toggle_btn);

            // Copy Button
            let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
            copy_btn.add_css_class("flat");
            let totp_copy = totp.clone();
            let sender_clone = sender.clone();
            copy_btn.connect_clicked(move |_| {
                if let Ok(code) = totp_copy.value_now() {
                    sender_clone.input(EntryDetailViewInput::CopyField(code.code));
                }
            });
            value_row.append(&copy_btn);

            row.append(&value_row);
            container.append(&row);

            // Timer
             let totp_timer = totp.clone();
            let code_label_weak = code_label.downgrade();
            let drawing_area_weak = drawing_area.downgrade();
            let totp_visible = self.totp_visible;
            
            gtk4::glib::timeout_add_local(
                std::time::Duration::from_millis(100),
                move || {
                    let Some(code_label) = code_label_weak.upgrade() else {
                        return gtk4::glib::ControlFlow::Break;
                    };
                    let Some(drawing_area) = drawing_area_weak.upgrade() else {
                        return gtk4::glib::ControlFlow::Break;
                    };

                    drawing_area.queue_draw();

                     let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                     let remaining = totp_timer.period - (now % totp_timer.period);
                     drawing_area.set_tooltip_text(Some(&format!("{}s remaining", remaining)));

                    if let Ok(code) = totp_timer.value_now() {
                         if totp_visible {
                             code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", code.code));
                        } else {
                             code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", TOTP_MASK));
                        }
                    }

                    gtk4::glib::ControlFlow::Continue
                }
            );
        }
    }

     fn add_text_row(&self, container: &gtk4::Box, label: &str, value: &str) {
        crate::components::common::create_text_area_row(label, value); // Not using helper effectively here as we need read-only
        // Re-implement read only text row
         let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some(label));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let frame = gtk4::Frame::new(None);
        // frame.set_height_request(100); // Allow efficient sizing?

        let text_view = gtk4::TextView::new();
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(gtk4::WrapMode::Word);
        text_view.set_left_margin(8);
        text_view.set_right_margin(8);
        text_view.set_top_margin(8);
        text_view.set_bottom_margin(8);
        text_view.buffer().set_text(value);

        frame.set_child(Some(&text_view));
        row.append(&frame);
        container.append(&row);
    }
}
