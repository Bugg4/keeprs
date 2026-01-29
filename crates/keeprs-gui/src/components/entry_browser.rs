//! Entry browser component.
//!
//! Displays entry list and entry details with breadcrumb navigation.
//! Uses a two-column layout: entry list on the left, details on the right.

use keeprs_core::{Entry, Group, NavigationPath, NavigationStep};
use gtk4::prelude::*;
use gtk4::gdk;
use gtk4::glib;
use gtk4::cairo::Context;
use keepass::db::TOTP;
use relm4::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use zxcvbn::{zxcvbn, Score};
use url::Url;

/// Minimum width for each column.
const COLUMN_MIN_WIDTH: i32 = 250;

/// Mask characters for hidden fields.
const PASSWORD_MASK: &str = "••••••••";
const TOTP_MASK: &str = "••••••";

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
    /// Toggle password visibility for the current entry.
    TogglePasswordVisible,
    /// Toggle TOTP code visibility for the current entry.
    ToggleTotpVisible,
    /// Copy a field value to clipboard.
    CopyField(String),
    /// Add new entry.
    AddEntry,
    /// Edit current entry (enter inline edit mode).
    EditEntry,
    /// Delete current entry.
    DeleteEntry,
    /// Save an attachment.
    SaveAttachment(String),
    /// Open an attachment.
    OpenAttachment(String),
    /// Open URL in default browser.
    OpenUrl(String),
    /// Enter inline edit mode.
    #[allow(dead_code)]
    EnterEditMode,
    /// Exit edit mode (true = save, false = cancel).
    ExitEditMode(bool),
    /// Edit title field.
    EditTitle(String),
    /// Edit username field.
    EditUsername(String),
    /// Edit password field.
    EditPassword(String),
    /// Edit URL field.
    EditUrl(String),
    /// Edit notes field.
    EditNotes(String),
    /// Favicon loaded for URL.
    FaviconLoaded { #[allow(dead_code)] url: String, data: Vec<u8> },
    /// Set whether we are in trash mode (enables permanent deletion).
    SetTrashMode(bool),
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
    /// Password visibility state.
    password_visible: bool,
    /// Whether we are in inline edit mode.
    editing: bool,
    /// Entry being edited (holds uncommitted changes).
    edited_entry: Option<Entry>,
    /// Whether to show the password entropy bar.
    show_entropy_bar: bool,
    /// Whether to show TOTP codes by default (visible) or hidden.
    show_totp_visible: bool,
    /// Current state of TOTP visibility for displayed entry.
    totp_visible: bool,
    /// Current favicon image widget (for updating when favicon loads).
    favicon_image: Rc<RefCell<Option<gtk4::Image>>>,
    /// Whether we are in trash mode (permanent deletion).
    trash_mode: bool,
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

                #[name = "columns_box"]
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_vexpand: true,
                }
            }
        }
    }

    fn init(
        (show_entropy_bar, show_totp_visible): Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = EntryBrowser {
            root_group: None,
            nav_path: NavigationPath::new(),
            current_entries: Vec::new(),
            selected_entry: None,
            password_visible: false,
            editing: false,
            edited_entry: None,
            show_entropy_bar,
            show_totp_visible,
            totp_visible: show_totp_visible,
            favicon_image: Rc::new(RefCell::new(None)),
            trash_mode: false,
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
                self.password_visible = false;
                self.totp_visible = self.show_totp_visible;
                self.editing = false;
                self.edited_entry = None;
                // Reset trash mode unless explicitly set? No, safer to reset and expect SetTrashMode if needed.
                self.trash_mode = false;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::SelectEntry { uuid, entry } => {
                // Add entry to navigation
                self.nav_path.push_entry(uuid, entry.title.clone());
                self.selected_entry = Some(entry);
                self.password_visible = false;
                self.totp_visible = self.show_totp_visible;
                self.editing = false;
                self.edited_entry = None;
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
                self.password_visible = false;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::TogglePasswordVisible => {
                self.password_visible = !self.password_visible;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::ToggleTotpVisible => {
                self.totp_visible = !self.totp_visible;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::CopyField(value) => {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&value);
                }
            }
            EntryBrowserInput::AddEntry => {
                let _ = sender.output(EntryBrowserOutput::AddEntry);
            }
            EntryBrowserInput::EditEntry => {
                // Enter inline edit mode
                if let Some(ref entry) = self.selected_entry {
                    self.editing = true;
                    self.edited_entry = Some(entry.clone());
                    self.rebuild_columns(widgets, &sender);
                }
            }
            EntryBrowserInput::EnterEditMode => {
                if let Some(ref entry) = self.selected_entry {
                    self.editing = true;
                    self.edited_entry = Some(entry.clone());
                    self.rebuild_columns(widgets, &sender);
                }
            }
            EntryBrowserInput::ExitEditMode(save) => {
                if save {
                    if let Some(ref edited) = self.edited_entry {
                        // Update selected_entry with edits
                        self.selected_entry = Some(edited.clone());
                        let _ = sender.output(EntryBrowserOutput::EntryEdited(edited.clone()));
                    }
                }
                self.editing = false;
                self.edited_entry = None;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::EditTitle(title) => {
                if let Some(ref mut entry) = self.edited_entry {
                    entry.title = title;
                }
            }
            EntryBrowserInput::EditUsername(username) => {
                if let Some(ref mut entry) = self.edited_entry {
                    entry.username = username;
                }
            }
            EntryBrowserInput::EditPassword(password) => {
                if let Some(ref mut entry) = self.edited_entry {
                    entry.password = password;
                }
            }
            EntryBrowserInput::EditUrl(url) => {
                if let Some(ref mut entry) = self.edited_entry {
                    entry.url = url;
                }
            }
            EntryBrowserInput::EditNotes(notes) => {
                if let Some(ref mut entry) = self.edited_entry {
                    entry.notes = notes;
                }
            }
            EntryBrowserInput::DeleteEntry => {
                if let Some(ref entry) = self.selected_entry {
                    if self.trash_mode {
                         let _ = sender.output(EntryBrowserOutput::RequestPermanentDeleteEntry(entry.uuid.clone()));
                    } else {
                         let _ = sender.output(EntryBrowserOutput::DeleteEntry(entry.uuid.clone()));
                    }
                }
            }
            EntryBrowserInput::SetTrashMode(is_trash) => {
                self.trash_mode = is_trash;
                self.rebuild_columns(widgets, &sender);
            }
            EntryBrowserInput::SaveAttachment(filename) => {
                if let Some(ref entry) = self.selected_entry {
                    if let Some(att) = entry.attachments.iter().find(|a| a.filename == filename) {
                        let _ = sender.output(EntryBrowserOutput::SaveAttachment {
                            filename: att.filename.clone(),
                            data: att.data.clone(),
                        });
                    }
                }
            }
            EntryBrowserInput::OpenAttachment(filename) => {
                if let Some(ref entry) = self.selected_entry {
                    if let Some(att) = entry.attachments.iter().find(|a| a.filename == filename) {
                        let _ = sender.output(EntryBrowserOutput::OpenAttachment {
                            filename: att.filename.clone(),
                            data: att.data.clone(),
                        });
                    }
                }
            }
            EntryBrowserInput::OpenUrl(url) => {
                // Open URL in default browser using xdg-open
                std::thread::spawn(move || {
                    if let Err(e) = std::process::Command::new("xdg-open")
                        .arg(&url)
                        .spawn()
                    {
                        tracing::error!("Failed to open URL {}: {}", url, e);
                    }
                });
            }
            EntryBrowserInput::FaviconLoaded { url: _, data } => {
                // Update favicon image if widget reference exists
                if let Some(image_widget) = self.favicon_image.borrow().as_ref() {
                    let bytes = glib::Bytes::from(&data);
                    let stream = gtk4::gio::MemoryInputStream::from_bytes(&bytes);
                    let pixbuf = gdk::gdk_pixbuf::Pixbuf::from_stream(&stream, None::<&gtk4::gio::Cancellable>);
                    
                    if let Ok(pixbuf) = pixbuf {
                        // Scale to reasonable icon size if needed (e.g. 16x16 or 24x24)
                         // But for now let's just use it as is, or maybe scale it twice so it looks good on high dpi
                        let texture = gdk::Texture::for_pixbuf(&pixbuf);
                        image_widget.set_paintable(Some(&texture));
                        image_widget.set_visible(true);
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
                sender_clone.input(EntryBrowserInput::NavigateToDepth(depth));
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

        let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
        add_btn.add_css_class("flat");
        add_btn.set_tooltip_text(Some("Add Entry"));
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

    /// Build the entry detail column.
    fn build_entry_detail_column(&self, entry: &Entry, sender: &ComponentSender<Self>) -> gtk4::Box {
        let column = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        column.set_width_request(COLUMN_MIN_WIDTH);
        column.set_hexpand(true);
        column.set_vexpand(true);

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        toolbar.set_margin_all(8);

        if self.editing {
            // Edit mode: Save and Cancel buttons
            let save_btn = gtk4::Button::with_label("Save");
            save_btn.add_css_class("suggested-action");
            let sender_clone = sender.clone();
            save_btn.connect_clicked(move |_| {
                sender_clone.input(EntryBrowserInput::ExitEditMode(true));
            });
            toolbar.append(&save_btn);

            let cancel_btn = gtk4::Button::with_label("Cancel");
            let sender_clone = sender.clone();
            cancel_btn.connect_clicked(move |_| {
                sender_clone.input(EntryBrowserInput::ExitEditMode(false));
            });
            toolbar.append(&cancel_btn);
        } else {
            // View mode: Edit and Delete buttons
            let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
            edit_btn.add_css_class("flat");
            edit_btn.set_tooltip_text(Some("Edit Entry"));
            let sender_clone = sender.clone();
            edit_btn.connect_clicked(move |_| {
                sender_clone.input(EntryBrowserInput::EditEntry);
            });
            toolbar.append(&edit_btn);

            let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
            delete_btn.add_css_class("flat");
            if self.trash_mode {
                 delete_btn.set_tooltip_text(Some("Delete Permanently"));
                 delete_btn.add_css_class("destructive-action");
            } else {
                 delete_btn.set_tooltip_text(Some("Delete Entry"));
            }
            let sender_clone = sender.clone();
            delete_btn.connect_clicked(move |_| {
                sender_clone.input(EntryBrowserInput::DeleteEntry);
            });
            toolbar.append(&delete_btn);
        }

        column.append(&toolbar);
        column.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        // Details
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let details_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        details_box.set_margin_all(24);
        details_box.set_valign(gtk4::Align::Start);

        if self.editing {
            // Edit mode: render form fields
            let edited = self.edited_entry.as_ref().unwrap_or(entry);

            // Title field
            self.add_edit_field(&details_box, "Title", &edited.title, sender, |_s, text| {
                EntryBrowserInput::EditTitle(text)
            });

            // Username field
            self.add_edit_field(&details_box, "Username", &edited.username, sender, |_s, text| {
                EntryBrowserInput::EditUsername(text)
            });

            // Password field
            self.add_password_edit_field(&details_box, "Password", &edited.password, sender);

            // URL field
            self.add_edit_field(&details_box, "URL", &edited.url, sender, |_s, text| {
                EntryBrowserInput::EditUrl(text)
            });

            // Notes field
            self.add_notes_edit_field(&details_box, "Notes", &edited.notes, sender);

        } else {
            // View mode: render read-only labels
            // Title
            let title = gtk4::Label::new(Some(&entry.title));
            title.add_css_class("title-1");
            title.set_halign(gtk4::Align::Start);
            title.set_selectable(true);
            title.set_wrap(true);
            title.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            details_box.append(&title);

            // Separator after title
            details_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            
            // Authentication Group (implicit)
            if !entry.username.is_empty() || !entry.password.is_empty() || entry.otp.is_some() {
                // Fields
                if !entry.username.is_empty() {
                    self.add_field_row(&details_box, "Username", &entry.username, false, None, sender);
                }

                if !entry.password.is_empty() {
                    self.add_password_row(&details_box, &entry.password, sender);
                }


            if let Some(otp_uri) = &entry.otp {
                if let Ok(totp) = otp_uri.parse::<TOTP>() {
                    if let Ok(code) = totp.value_now() {
                        // Wrap TOTP in Rc to share with closures
                        let totp = Rc::new(totp);

                        // Custom TOTP Row with Animation
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
                        
                        // Show code or mask based on visibility
                        if self.totp_visible {
                            code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", code.code));
                        } else {
                            code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", TOTP_MASK));
                        }
                        value_row.append(&code_label);

                        // Drawing Area for Progress (always visible)
                        let drawing_area = gtk4::DrawingArea::new();
                        drawing_area.set_content_width(24);
                        drawing_area.set_content_height(24);
                        drawing_area.set_margin_end(8);
                        drawing_area.set_has_tooltip(true);

                        // Set initial tooltip
                        {
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

                            // Get theme colors from style context
                            let style_context = area.style_context();
                            
                            // Get background color for the track
                            let bg_color = style_context.lookup_color("unfocused_borders");
                            let (bg_r, bg_g, bg_b) = if let Some(color) = bg_color {
                                (color.red() as f64, color.green() as f64, color.blue() as f64)
                            } else {
                                (0.85, 0.85, 0.85) // Fallback gray
                            };

                            // Get accent color for the progress
                            let accent_color = style_context.lookup_color("accent_bg_color")
                                .or_else(|| style_context.lookup_color("accent_color"))
                                .or_else(|| style_context.lookup_color("theme_selected_bg_color"));
                            let (acc_r, acc_g, acc_b) = if let Some(color) = accent_color {
                                (color.red() as f64, color.green() as f64, color.blue() as f64)
                            } else {
                                (0.2, 0.6, 1.0) // Fallback blue
                            };

                            // Background Circle (theme border color)
                            cr.set_source_rgba(bg_r, bg_g, bg_b, 1.0);
                            cr.arc(center_x, center_y, radius, 0.0, 2.0 * std::f64::consts::PI);
                            cr.fill().expect("Invalid cairo surface state");

                            // Progress Pie (theme accent color)
                            cr.set_source_rgba(acc_r, acc_g, acc_b, 1.0);
                            cr.move_to(center_x, center_y);
                            // Rotate -90 degrees (start at top)
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
                        toggle_btn.set_tooltip_text(Some(if self.totp_visible { "Hide TOTP" } else { "Show TOTP" }));
                        let sender_toggle = sender.clone();
                        toggle_btn.connect_clicked(move |_| {
                            sender_toggle.input(EntryBrowserInput::ToggleTotpVisible);
                        });
                        value_row.append(&toggle_btn);

                        // Copy Button
                        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
                        copy_btn.add_css_class("flat");
                        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
                        let totp_copy = totp.clone();
                        let sender_clone = sender.clone();
                        copy_btn.connect_clicked(move |_| {
                            if let Ok(code) = totp_copy.value_now() {
                                sender_clone.input(EntryBrowserInput::CopyField(code.code));
                            }
                        });
                        value_row.append(&copy_btn);

                        row.append(&value_row);
                        details_box.append(&row);

                        // Timer to update UI
                        let totp_timer = totp.clone();
                        let code_label_weak = code_label.downgrade();
                        let drawing_area_weak = drawing_area.downgrade();
                        glib::timeout_add_local(
                            std::time::Duration::from_millis(100),
                            move || {
                                let Some(code_label) = code_label_weak.upgrade() else {
                                    return glib::ControlFlow::Break;
                                };
                                let Some(drawing_area) = drawing_area_weak.upgrade() else {
                                    return glib::ControlFlow::Break;
                                };
                                
                                if let Ok(code) = totp_timer.value_now() {
                                    // Only update the code text if it's visible (not masked)
                                    // If the label shows the TOTP mask, it means TOTP is hidden - don't update
                                    let current_text = code_label.text();
                                    if current_text != TOTP_MASK && current_text.as_str() != code.code {
                                        code_label.set_markup(&format!("<span font_family=\"monospace\" size=\"large\">{}</span>", code.code));
                                    }
                                    
                                    // Update tooltip with remaining seconds (always, even when hidden)
                                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                    let remaining = totp_timer.period - (now % totp_timer.period);
                                    drawing_area.set_tooltip_text(Some(&format!("{}s remaining", remaining)));
                                    
                                    // Trigger redraw for spinner animation (always)
                                    drawing_area.queue_draw();
                                }
                                glib::ControlFlow::Continue
                            }
                        );
                    }
                }
            }

            // Add separator at end of Authentication group
            details_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            }

            if !entry.url.is_empty() {
                self.add_url_row(&details_box, &entry.url, sender);
                 // Separator after URL
                details_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
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
                notes_text.add_css_class("monospace");
                details_box.append(&notes_text);
                
                // Separator after notes
                details_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            }

            if !entry.attachments.is_empty() {
                let att_label = gtk4::Label::new(Some("Attachments"));
                att_label.add_css_class("dim-label");
                att_label.set_halign(gtk4::Align::Start);
                att_label.set_margin_top(12);
                details_box.append(&att_label);

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
                        sender_clone.input(EntryBrowserInput::SaveAttachment(filename.clone()));
                    });
                    row.append(&save_btn);

                    let open_btn = gtk4::Button::from_icon_name("document-open-symbolic");
                    open_btn.add_css_class("flat");
                    open_btn.set_tooltip_text(Some("Open Attachment"));
                    let filename = att.filename.clone();
                    let sender_clone = sender.clone();
                    open_btn.connect_clicked(move |_| {
                        sender_clone.input(EntryBrowserInput::OpenAttachment(filename.clone()));
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
                sender_clone.input(EntryBrowserInput::TogglePasswordVisible);
            });
            value_row.append(&toggle_btn);
        }

        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
        let copy_value = real_value.unwrap_or(value).to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryBrowserInput::CopyField(copy_value.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);
        container.append(&row);
    }

    /// Add a URL row with copy and open buttons.
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

        // Favicon container
        let favicon_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        favicon_box.add_css_class("favicon-box"); // For theming
        
        let favicon = gtk4::Image::new();
        favicon.set_pixel_size(16);
        favicon.set_visible(false); // Hidden until loaded
        favicon_box.append(&favicon);
        value_row.append(&favicon_box);
        
        // Store reference to update later
        *self.favicon_image.borrow_mut() = Some(favicon);

        // Fetch favicon in background
        let url_string = url.to_string();
        let sender_clone = sender.clone();
        
        std::thread::spawn(move || {
            // TODO: Implement favicon caching to ~/.local/share/keeprs/favicons
            // Check cache first before fetching network
            
            // Very basic favicon fetching: try /favicon.ico at domain root
            // In a real app we'd parse HTML for <link rel="icon">
            if let Ok(parsed) = Url::parse(&url_string) {
                 if let Some(domain) = parsed.domain() {
                    let favicon_url = format!("{}://{}/favicon.ico", parsed.scheme(), domain);
                    
                    // Use reqwest to fetch
                    if let Ok(response) = reqwest::blocking::get(&favicon_url) {
                         if response.status().is_success() {
                             if let Ok(bytes) = response.bytes() {
                                 // TODO: Save to cache here
                                 
                                 sender_clone.input(EntryBrowserInput::FaviconLoaded { 
                                     url: url_string, 
                                     data: bytes.to_vec() 
                                 });
                             }
                         }
                    }
                 }
            }
        });

        let value_label = gtk4::Label::new(Some(url));
        value_label.set_halign(gtk4::Align::Start);
        value_label.set_hexpand(true);
        value_label.set_selectable(true);
        value_row.append(&value_label);

        // Open in browser button
        let open_btn = gtk4::Button::from_icon_name("web-browser-symbolic");
        open_btn.add_css_class("flat");
        open_btn.set_tooltip_text(Some("Open in browser"));
        let url_clone = url.to_string();
        let sender_clone = sender.clone();
        open_btn.connect_clicked(move |_| {
            sender_clone.input(EntryBrowserInput::OpenUrl(url_clone.clone()));
        });
        value_row.append(&open_btn);

        // Copy button
        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
        let url_clone = url.to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryBrowserInput::CopyField(url_clone.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);
        container.append(&row);
    }

    /// Get password strength info using zxcvbn.
    /// Returns (score 0-4, guesses_log10, strength_label, css_class).
    fn get_password_strength(password: &str) -> (u8, f64, &'static str, &'static str) {
        if password.is_empty() {
            return (0, 0.0, "Empty", "error");
        }

        let entropy = zxcvbn(password, &[]);
        let score = entropy.score();
        let guesses_log10 = entropy.guesses_log10();

        // Score is 0-4, map to labels
        let (score_num, label, css_class) = match score {
            Score::Zero => (0, "Very Weak", "error"),
            Score::One => (1, "Weak", "error"),
            Score::Two => (2, "Fair", "warning"),
            Score::Three => (3, "Strong", "success"),
            Score::Four => (4, "Very Strong", "success"),
            _ => (2, "Unknown", "warning"), // Fallback for any future variants
        };

        (score_num, guesses_log10, label, css_class)
    }

    /// Add a password row with visibility toggle, copy button, and entropy bar.
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

        // Use Label with CSS class for monospace font
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

        // Toggle visibility button
        let toggle_btn = gtk4::Button::from_icon_name(
            if self.password_visible { "view-conceal-symbolic" } else { "view-reveal-symbolic" }
        );
        toggle_btn.add_css_class("flat");
        toggle_btn.set_tooltip_text(Some("Toggle visibility"));
        let sender_clone = sender.clone();
        toggle_btn.connect_clicked(move |_| {
            sender_clone.input(EntryBrowserInput::TogglePasswordVisible);
        });
        value_row.append(&toggle_btn);

        // Copy button
        let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
        copy_btn.add_css_class("flat");
        copy_btn.set_tooltip_text(Some("Copy to clipboard"));
        let password_clone = password.to_string();
        let sender_clone = sender.clone();
        copy_btn.connect_clicked(move |_| {
            sender_clone.input(EntryBrowserInput::CopyField(password_clone.clone()));
        });
        value_row.append(&copy_btn);

        row.append(&value_row);

        // Entropy bar (only if enabled in config)
        if self.show_entropy_bar {
            let (score, guesses_log10, strength_label, strength_class) = Self::get_password_strength(password);

            let entropy_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            entropy_row.set_margin_top(2);

            // Progress bar (thin)
            let progress_bar = gtk4::ProgressBar::new();
            progress_bar.set_hexpand(true);
            progress_bar.set_valign(gtk4::Align::Center);
            // Score is 0-4, normalize to 0-1
            let fraction = score as f64 / 4.0;
            progress_bar.set_fraction(fraction);
            
            // Add appropriate CSS class for color
            match strength_class {
                "success" => progress_bar.add_css_class("success"),
                "warning" => progress_bar.add_css_class("warning"),
                "error" => progress_bar.add_css_class("error"),
                _ => {}
            }
            
            entropy_row.append(&progress_bar);

            // Entropy text (show guesses_log10 which represents bits of entropy)
            let entropy_text = gtk4::Label::new(Some(&format!("{:.0} bits", guesses_log10 * 3.32))); // log10 to log2 conversion
            entropy_text.add_css_class("dim-label");
            entropy_text.add_css_class("caption");
            entropy_row.append(&entropy_text);

            // Strength label
            let strength_text = gtk4::Label::new(Some(strength_label));
            strength_text.add_css_class("caption");
            match strength_class {
                "success" => strength_text.add_css_class("success"),
                "warning" => strength_text.add_css_class("warning"),
                "error" => strength_text.add_css_class("error"),
                _ => {}
            }
            entropy_row.append(&strength_text);

            row.append(&entropy_row);
        }
        
        container.append(&row);
    }

    /// Add an editable text field.
    fn add_edit_field<F>(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        sender: &ComponentSender<Self>,
        make_input: F,
    )
    where
        F: Fn(&ComponentSender<Self>, String) -> EntryBrowserInput + 'static,
    {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some(label));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let entry = gtk4::Entry::new();
        entry.set_text(value);
        entry.set_hexpand(true);

        let sender_clone = sender.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            sender_clone.input(make_input(&sender_clone, text));
        });

        row.append(&entry);
        container.append(&row);
    }

    /// Add an editable password field.
    fn add_password_edit_field(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        sender: &ComponentSender<Self>,
    ) {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some(label));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let entry = gtk4::PasswordEntry::new();
        entry.set_text(value);
        entry.set_show_peek_icon(true);
        entry.set_hexpand(true);
        entry.add_css_class("monospace");

        let sender_clone = sender.clone();
        entry.connect_changed(move |e| {
            let text = e.text().to_string();
            sender_clone.input(EntryBrowserInput::EditPassword(text));
        });

        row.append(&entry);
        container.append(&row);
    }

    /// Add an editable notes field (multi-line).
    fn add_notes_edit_field(
        &self,
        container: &gtk4::Box,
        label: &str,
        value: &str,
        sender: &ComponentSender<Self>,
    ) {
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

        let label_widget = gtk4::Label::new(Some(label));
        label_widget.add_css_class("dim-label");
        label_widget.set_halign(gtk4::Align::Start);
        row.append(&label_widget);

        let frame = gtk4::Frame::new(None);
        frame.set_height_request(100);

        let text_view = gtk4::TextView::new();
        text_view.set_wrap_mode(gtk4::WrapMode::Word);
        text_view.set_margin_all(8);
        text_view.add_css_class("monospace");
        text_view.buffer().set_text(value);

        let sender_clone = sender.clone();
        text_view.buffer().connect_changed(move |buf| {
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
            sender_clone.input(EntryBrowserInput::EditNotes(text));
        });

        frame.set_child(Some(&text_view));
        row.append(&frame);
        container.append(&row);
    }
}
