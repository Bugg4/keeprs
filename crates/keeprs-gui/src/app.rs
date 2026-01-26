//! Main application component.

use crate::components::entry_browser::{EntryBrowser, EntryBrowserInput, EntryBrowserOutput};
use crate::components::entry_edit::{EntryEdit, EntryEditInput, EntryEditOutput};
use crate::components::search_palette::{SearchPalette, SearchPaletteInput, SearchPaletteOutput};
use crate::components::sidebar::{Sidebar, SidebarInit, SidebarInput, SidebarOutput};
use crate::components::unlock::{UnlockDialog, UnlockInput, UnlockOutput};
use crate::config::Config;
use keeprs_core::{Entry, Group, KeepassDatabase};

use gtk4::prelude::*;
use gtk4::gdk;
use relm4::prelude::*;
use std::sync::{Arc, RwLock};

#[cfg(debug_assertions)]
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
#[derive(Debug, Serialize, Deserialize, Default)]
struct DevState {
    group_uuid: Option<String>,
    entry_uuid: Option<String>,
}

#[cfg(debug_assertions)]
impl DevState {
    fn load() -> Self {
        if let Ok(content) = std::fs::read_to_string(".dev_state.toml") {
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        if let Ok(content) = toml::to_string(self) {
            let _ = std::fs::write(".dev_state.toml", content);
        }
    }
}

/// Application state.
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Waiting for unlock.
    Locked,
    /// Database is unlocked.
    Unlocked,
}

/// Main app messages.
#[derive(Debug)]
pub enum AppInput {
    /// Password submitted from unlock dialog.
    PasswordSubmitted(String),
    /// Unlock failed with error.
    // UnlockFailed(String), // Unused
    /// Database unlocked successfully.
    /// Database unlocked successfully.
    // DatabaseUnlocked, // Unused
    /// Group selected in sidebar.
    GroupSelected(String),
    /// Entry selected in sidebar.
    SidebarEntrySelected(String),
    /// Group selected from search with full data.
    SearchGroupSelected { uuid: String, name: String, group: Group },
    /// Entry selected from search.
    SearchEntrySelected { entry: Entry, group_uuid: String },
    /// Entry actions.
    EditEntry(Entry),
    DeleteEntry(String),
    AddEntry,
    /// Save attachment.
    SaveAttachment { filename: String, data: Vec<u8> },
    /// Open attachment.
    OpenAttachment { filename: String, data: Vec<u8> },
    /// Entry saved from edit dialog.
    EntrySaved(Entry),
    /// Request to save the database.
    SaveDatabase,
    /// Save operation finished.
    SaveFinished(Result<(), String>),
    /// Toggle search palette visibility.
    ToggleSearch,
    /// No operation.
    NoOp,
}

/// Main application model.
pub struct App {
    state: AppState,
    config: Config,
    database: Option<Arc<RwLock<KeepassDatabase>>>,
    db_filename: Option<String>,
    db_size: String,
    entry_count: usize,
    unsaved_changes: bool,
    is_saving: bool,
    last_save_time: String,
    current_group_uuid: Option<String>,
    root_group: Option<Group>,

    // Child components
    unlock: Controller<UnlockDialog>,
    search_palette: Controller<SearchPalette>,
    sidebar: Controller<Sidebar>,
    entry_browser: Controller<EntryBrowser>,
    entry_edit: Controller<EntryEdit>,
}

#[relm4::component(pub)]
impl Component for App {
    type Init = Config;
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[name = "main_window"]
        gtk4::ApplicationWindow {
            set_title: Some("Keeprs"),
            set_default_width: 1100,
            set_default_height: 700,

            #[name = "main_stack"]
            gtk4::Stack {
                set_transition_type: gtk4::StackTransitionType::Crossfade,

                // Unlock view
                add_child = &gtk4::Box {
                    set_halign: gtk4::Align::Center,
                    set_valign: gtk4::Align::Center,

                    model.unlock.widget().clone() {},
                } -> {
                    set_name: "unlock",
                },

                // Main view with sidebar and content, wrapped in Overlay for search palette
                add_child = &gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 0,

                    append = &gtk4::Overlay {
                        set_vexpand: true,
                        #[wrap(Some)]
                        #[name = "main_paned"]
                        set_child = &gtk4::Paned {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_shrink_start_child: false, // Enforce minimum width
                            set_resize_start_child: true, // Allow manual resizing
                            set_resize_end_child: true,
                            set_shrink_end_child: false,
    
                            // Left side: Sidebar (folder tree)
                            #[wrap(Some)]
                            set_start_child = &gtk4::Box {
                                set_orientation: gtk4::Orientation::Vertical,
                                set_vexpand: true,
                                
                                // Folder tree 
                                model.sidebar.widget().clone() {},
                            },
    
                            // Right side: entry browser
                            #[wrap(Some)]
                            set_end_child = model.entry_browser.widget(),
                        },
    
                        add_overlay = model.search_palette.widget(),
                    },

                    // Bottom info bar
                    gtk4::Separator {
                        set_orientation: gtk4::Orientation::Horizontal,
                    },
                    
                    gtk4::CenterBox {
                        set_margin_all: 4,
                        set_margin_start: 8,
                        set_margin_end: 8,
                        set_hexpand: true, // Ensure it fills width for alignment
                        
                        // Left: Filename + Entry Count
                        #[wrap(Some)]
                        set_start_widget = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 16,
                            
                            // Database Name
                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                
                                gtk4::Image {
                                    set_icon_name: Some("folder-open-symbolic"),
                                    add_css_class: "dim-label",
                                },
                                gtk4::Label {
                                    #[watch]
                                    set_label: model.db_filename.as_deref().unwrap_or(""),
                                    add_css_class: "dim-label",
                                },
                            },
                            
                            // Entry Count
                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                
                                gtk4::Image {
                                    set_icon_name: Some("view-list-symbolic"),
                                    add_css_class: "dim-label",
                                },
                                gtk4::Label {
                                    #[watch]
                                    set_label: &format!("{} entries", model.entry_count),
                                    add_css_class: "dim-label",
                                },
                            },
                            
                            // Database Size
                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                
                                gtk4::Image {
                                    set_icon_name: Some("drive-harddisk-symbolic"),
                                    add_css_class: "dim-label",
                                },
                                gtk4::Label {
                                    #[watch]
                                    set_label: &model.db_size,
                                    add_css_class: "dim-label",
                                },
                            },
                        },

                        // Right: Unsaved Indicator + Last saved status
                        #[wrap(Some)]
                        set_end_widget = &gtk4::Box {
                            set_orientation: gtk4::Orientation::Horizontal,
                            set_spacing: 12,

                            // Unsaved changes indicator (only show dot when there are unsaved changes)
                            gtk4::Label {
                                #[watch]
                                set_label: if model.unsaved_changes { "●" } else { "" },
                                add_css_class: "dim-label",
                            },

                            // Last Save Status - always visible
                            gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_spacing: 6,
                                
                                // Last saved text first (left side)
                                gtk4::Label {
                                    #[watch]
                                    set_label: &if model.is_saving {
                                        "Saving".to_string()
                                    } else if model.last_save_time.is_empty() {
                                        "No changes".to_string()
                                    } else {
                                        format!("Last save: {}", model.last_save_time)
                                    },
                                    add_css_class: "dim-label",
                                },
                                
                                // Status icon on right: spinner (saving), checkmark (saved), dash (not saved yet)
                                gtk4::Spinner {
                                    set_size_request: (16, 16),
                                    set_spinning: true,
                                    #[watch]
                                    set_visible: model.is_saving,
                                },
                                gtk4::Image {
                                    #[watch]
                                    set_icon_name: Some(if model.last_save_time.is_empty() {
                                        "content-loading-symbolic" // dash/empty icon for "not saved yet"
                                    } else {
                                        "object-select-symbolic" // standard GTK checkmark for saved
                                    }),
                                    add_css_class: "dim-label",
                                    #[watch]
                                    set_visible: !model.is_saving,
                                },
                            }
                        }
                    }
                } -> {
                    set_name: "main",
                },
            },
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize child components
        let unlock = UnlockDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                UnlockOutput::Unlocked(password) => AppInput::PasswordSubmitted(password),
            });

        let search_palette = SearchPalette::builder()
            .launch(config.hidden_groups.clone())
            .forward(sender.input_sender(), |output| match output {
                SearchPaletteOutput::GroupSelected { uuid, name, group } => {
                    AppInput::SearchGroupSelected { uuid, name, group }
                }
                SearchPaletteOutput::EntrySelected { uuid: _, entry, group_uuid } => {
                    AppInput::SearchEntrySelected { entry, group_uuid }
                }
                SearchPaletteOutput::Closed => {
                    AppInput::NoOp
                }
            });

        tracing::info!("Initializing Sidebar with initial_width: {}, min_width: {}", config.sidebar_initial_width, config.sidebar_min_width);
        let sidebar = Sidebar::builder()
            .launch(SidebarInit {
                initial_width: config.sidebar_initial_width,
                min_width: config.sidebar_min_width,
                hidden_groups: config.hidden_groups.clone(),
            })
            .forward(sender.input_sender(), |output| match output {
                SidebarOutput::GroupSelected(uuid) => AppInput::GroupSelected(uuid),
                SidebarOutput::EntrySelected(uuid) => {
                    // We need to find the parent group UUID for this entry
                    // Since we can't easily query the model here without access to it, 
                    // we'll pass a special message or handle it in update() if we passed more info.
                    // Actually, let's just use a new AppInput that does the lookup.
                    AppInput::SidebarEntrySelected(uuid)
                }
            });

        let entry_browser = EntryBrowser::builder()
            .launch(config.show_entropy_bar)
            .forward(sender.input_sender(), |output| match output {
                EntryBrowserOutput::EntryEdited(entry) => AppInput::EntrySaved(entry),
                EntryBrowserOutput::DeleteEntry(uuid) => AppInput::DeleteEntry(uuid),
                EntryBrowserOutput::AddEntry => AppInput::AddEntry,
                EntryBrowserOutput::SaveAttachment { filename, data } => AppInput::SaveAttachment { filename, data },
                EntryBrowserOutput::OpenAttachment { filename, data } => AppInput::OpenAttachment { filename, data },
            });

        let entry_edit = EntryEdit::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                EntryEditOutput::Saved(entry) => AppInput::EntrySaved(entry),
                EntryEditOutput::Cancelled => AppInput::SaveDatabase, // No-op trigger
            });

        let mut model = App {
            state: AppState::Locked,
            config,
            database: None,
            db_filename: None,
            db_size: String::new(),
            entry_count: 0,
            unsaved_changes: false,
            is_saving: false,
            last_save_time: String::new(),
            current_group_uuid: None,
            root_group: None,
            unlock,
            search_palette,
            sidebar,
            entry_browser,
            entry_edit,
        };
        
        // Auto-unlock in dev mode
        #[cfg(debug_assertions)]
        {
            // Load .env.dev if it exists
            let _ = dotenvy::from_filename(".env.dev");
            
            if let Ok(password) = std::env::var("DB_PASSWORD") {
                tracing::info!("Found DB_PASSWORD in env, attempting auto-unlock");
                match KeepassDatabase::unlock(&model.config.database_path, &password) {
                    Ok(db) => {
                         let root = db.root_group();
                         model.root_group = Some(root.clone());
                         model.database = Some(Arc::new(RwLock::new(db)));
                         model.state = AppState::Unlocked;
                         model.entry_count = count_entries(&root);
                         model.db_filename = std::path::Path::new(&model.config.database_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());
                            
                         model.db_size = std::fs::metadata(&model.config.database_path)
                            .map(|m| format_size(m.len()))
                            .unwrap_or_else(|_| "Unknown".to_string());
                         
                         // We need to send these signals *after* widgets are created, 
                         // but we can't emit to controllers before they are fully initialized/mapped sometimes?
                         // Actually Relm4 components are initialized here.
                         // However, the `sender` we have is for AppInput. 
                         // We can't easily emit to child components' inputs from `init` directly referencing `model.sidebar`
                         // because `model` is being built.
                         // But we created `sidebar` controller above.
                         
                         model.sidebar.emit(SidebarInput::SetRootGroup(root.clone()));
                         model.search_palette.emit(SearchPaletteInput::SetRootGroup(root.clone()));
                         model.entry_browser.emit(EntryBrowserInput::SetRootGroup(root.clone()));
                         
                         tracing::info!("Auto-unlock successful");

                         // Restore previous state
                         let state = DevState::load();
                         if let Some(group_uuid) = state.group_uuid {
                             tracing::info!("Restoring group: {}", group_uuid);
                             // Select group
                             // We don't have update logic here easily without duplicating it.
                             // But we can manually set the current group and emit.
                             model.current_group_uuid = Some(group_uuid.clone());
                             
                             if let Some(group) = find_group_by_uuid(&root, &group_uuid) {
                                 model.sidebar.emit(SidebarInput::UpdateSelection(group_uuid.clone()));
                                 model.entry_browser.emit(EntryBrowserInput::SelectGroup { 
                                     uuid: group_uuid.clone(), 
                                     name: group.name.clone(), 
                                     group: group.clone() 
                                 });

                                 // Restore entry
                                 if let Some(entry_uuid) = state.entry_uuid {
                                     tracing::info!("Restoring entry: {}", entry_uuid);
                                     if let Some(entry) = group.entries.iter().find(|e| e.uuid == entry_uuid) {
                                         model.entry_browser.emit(EntryBrowserInput::SelectEntry { 
                                             uuid: entry_uuid, 
                                             entry: entry.clone() 
                                         });
                                     }
                                 }
                             }
                         }
                    }
                    Err(e) => {
                        tracing::warn!("Auto-unlock failed: {}", e);
                    }
                }
            }
        }

        let widgets = view_output!();

        // Load CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("style.css"));
        
        if let Some(display) = gtk4::gdk::Display::default() {
             gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }



        // Register global keyboard shortcuts controller for the window
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
        
        let sender_clone = sender.clone();
        key_controller.connect_key_pressed(move |_, key, _keycode, state| {
            // Check for Ctrl+S
            if (key == gdk::Key::s || key == gdk::Key::S) 
                && state.contains(gdk::ModifierType::CONTROL_MASK) {
                sender_clone.input(AppInput::SaveDatabase);
                return gtk4::glib::Propagation::Stop;
            }
            // Check for Ctrl+P
            if (key == gdk::Key::p || key == gdk::Key::P) 
                && state.contains(gdk::ModifierType::CONTROL_MASK) {
                sender_clone.input(AppInput::ToggleSearch);
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        
        widgets.main_window.add_controller(key_controller);

        // Set initial sidebar width from config
        widgets.main_paned.set_position(model.config.sidebar_initial_width);
        tracing::info!("Set main_paned position to: {}", model.config.sidebar_initial_width);

        // Start on unlock screen vs main depending on state
        if model.state == AppState::Unlocked {
            widgets.main_stack.set_visible_child_name("main");
        } else {
            widgets.main_stack.set_visible_child_name("unlock");
        }

        // Connect dialogs to main window
        model.entry_edit.widget().set_transient_for(Some(&widgets.main_window));

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
            AppInput::PasswordSubmitted(password) => {
                // Attempt to unlock database
                match KeepassDatabase::unlock(&self.config.database_path, &password) {
                    Ok(db) => {
                        let root = db.root_group();
                        self.root_group = Some(root.clone());
                        self.database = Some(Arc::new(RwLock::new(db)));
                        self.state = AppState::Unlocked;
                        self.entry_count = count_entries(&root);
                        self.db_filename = std::path::Path::new(&self.config.database_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());
                            
                        self.db_size = std::fs::metadata(&self.config.database_path)
                            .map(|m| format_size(m.len()))
                            .unwrap_or_else(|_| "Unknown".to_string());
                            
                        // Populate sidebar and search
                        self.sidebar.emit(SidebarInput::SetRootGroup(root.clone()));
                        self.search_palette.emit(SearchPaletteInput::SetRootGroup(root.clone()));

                        // Set root group in entry browser
                        self.entry_browser.emit(EntryBrowserInput::SetRootGroup(root.clone()));

                        // Switch to main view
                        widgets.main_stack.set_visible_child_name("main");
                    }
                    Err(e) => {
                        self.unlock.emit(UnlockInput::ShowError(format!("Failed to unlock: {}", e)));
                    }
                }
            }

            AppInput::ToggleSearch => {
                self.search_palette.emit(SearchPaletteInput::Toggle);
            }
            AppInput::GroupSelected(uuid) => {
                self.current_group_uuid = Some(uuid.clone());

                // Find the group and show its entries in entry browser
                if let Some(ref root) = self.root_group {
                    if let Some(group) = find_group_by_uuid(root, &uuid) {
                        self.entry_browser.emit(EntryBrowserInput::SelectGroup {
                            uuid: uuid.clone(),
                            name: group.name.clone(),
                            group: group.clone(),
                        });

                        #[cfg(debug_assertions)]
                        {
                            let mut state = DevState::load();
                            state.group_uuid = Some(uuid.clone());
                            state.entry_uuid = None; // clear entry selection
                            state.save();
                        }
                    }
                }
            }
            AppInput::SearchGroupSelected { uuid, name, group } => {
                self.current_group_uuid = Some(uuid.clone());
                // Highlight in sidebar
                self.sidebar.emit(SidebarInput::UpdateSelection(uuid.clone()));
                
                self.entry_browser.emit(EntryBrowserInput::SelectGroup { uuid, name, group });
            }
            AppInput::SearchEntrySelected { entry, group_uuid } => {
                // Select the group first, then the entry
                self.current_group_uuid = Some(group_uuid.clone());
                
                // Highlight in sidebar
                self.sidebar.emit(SidebarInput::UpdateSelection(group_uuid.clone()));
                
                if let Some(ref root) = self.root_group {
                    if let Some(group) = find_group_by_uuid(root, &group_uuid) {
                        self.entry_browser.emit(EntryBrowserInput::SelectGroup {
                            uuid: group_uuid.clone(),
                            name: group.name.clone(),
                            group: group.clone(),
                        });
                        // Then select the entry
                        self.entry_browser.emit(EntryBrowserInput::SelectEntry {
                            uuid: entry.uuid.clone(),
                            entry: entry.clone(),
                        });

                        #[cfg(debug_assertions)]
                        {
                            let mut state = DevState::load();
                            state.group_uuid = Some(group_uuid.clone());
                            state.entry_uuid = Some(entry.uuid.clone()); // use entry.uuid assuming it is available in scope or passed
                            state.save();
                        }
                    }
                }
            }
            AppInput::EditEntry(entry) => {
                tracing::info!("Opening edit dialog for entry: {}", entry.title);
                self.entry_edit.emit(EntryEditInput::Edit(entry));
            }
            AppInput::DeleteEntry(uuid) => {
                // TODO: Implement delete through the database
                tracing::info!("Delete entry: {}", uuid);
                if let Some(ref _db) = self.database {
                    // For now, just refresh the view
                    if let Some(ref group_uuid) = self.current_group_uuid {
                        sender.input(AppInput::GroupSelected(group_uuid.clone()));
                    }
                }
            }
            AppInput::AddEntry => {
                self.entry_edit.emit(EntryEditInput::AddNew);
            }
            AppInput::EntrySaved(entry) => {
                tracing::info!("Entry saved: {}", entry.title);
                
                let mut refreshed_root = None;

                if let Some(ref db) = self.database {
                    // Lock for writing
                    if let Ok(mut db) = db.write() {
                        // Update entry in database
                        if let Err(e) = db.update_entry(&entry) {
                            tracing::error!("Failed to update entry: {}", e);
                            // TODO: Show error dialog
                            return;
                        }

                        // DO NOT auto-save to disk
                        // if let Err(e) = db.save() { ... }
                        
                        tracing::info!("Entry updated in memory");

                        // Refresh root group to reflect changes
                        refreshed_root = Some(db.root_group().clone());
                    }
                }

                if let Some(root) = refreshed_root {
                    self.root_group = Some(root);
                }
                
                // Mark as unsaved
                self.unsaved_changes = true;
                
                // Update count? If we added/removed (not yet supported via this message), we'd need to recount.
                // For safety, let's recount.
                if let Some(ref root) = self.root_group {
                     // Wait, self.root_group might be stale if we don't reload it from DB?
                     // keeprs-core keeps them in sync? 
                     // Usually we need to reload the group tree from the DB or update the in-memory tree.
                     // Assuming `db.update_entry` updates the in-memory structure referenced by `self.root_group`?
                     // Actually `db.root_group()` returns a clone in `init` and `PasswordSubmitted`.
                     // The `self.root_group` is a detached clone. `db` has its own copy.
                     // We need to update `self.root_group` to reflect changes if we want the UI tree to update.
                     // But strictly for *counting*, we can ask the DB.
                     
                     // For now, let's just assume count didn't change on EDIT.
                     // On ADD/DELETE we will handle it elsewhere or recount.
                }

                // Refresh the view and re-select the entry
                if let Some(ref group_uuid) = self.current_group_uuid {
                    sender.input(AppInput::SearchEntrySelected { 
                        entry: entry.clone(), 
                        group_uuid: group_uuid.clone() 
                    });
                }
                
                // Auto-save
                sender.input(AppInput::SaveDatabase);
            }
            AppInput::SaveAttachment { filename, data } => {
                let file_chooser = gtk4::FileChooserNative::new(
                    Some("Save Attachment"),
                    Some(&widgets.main_window),
                    gtk4::FileChooserAction::Save,
                    Some("Save"),
                    Some("Cancel"),
                );
                
                file_chooser.set_current_name(&filename);
                
                file_chooser.connect_response(move |dialog, response| {
                    if response == gtk4::ResponseType::Accept {
                        if let Some(file) = dialog.file() {
                             if let Some(path) = file.path() {
                                 let data = data.clone();
                                 std::thread::spawn(move || {
                                     if let Err(e) = std::fs::write(&path, data) {
                                         tracing::error!("Failed to save attachment to {}: {}", path.display(), e);
                                     } else {
                                         tracing::info!("Saved attachment to {}", path.display());
                                     }
                                 });
                             }
                        }
                    }
                    dialog.destroy();
                });
                
                file_chooser.show();
            }
            AppInput::OpenAttachment { filename, data } => {
                std::thread::spawn(move || {
                    let temp_dir = std::env::temp_dir();
                    let path = temp_dir.join(&filename);
                    
                    if let Err(e) = std::fs::write(&path, data) {
                        tracing::error!("Failed to write temp file {}: {}", path.display(), e);
                        return;
                    }
                    
                    tracing::info!("Opening attachment: {}", path.display());
                    
                    // Use xdg-open on Linux
                    if let Err(e) = std::process::Command::new("xdg-open")
                        .arg(&path)
                        .spawn() 
                    {
                        tracing::error!("Failed to open file {}: {}", path.display(), e);
                    }
                });
            }
            AppInput::SaveDatabase => {
                if let Some(ref db) = self.database {
                    if self.is_saving {
                        tracing::debug!("[SPINNER] SaveDatabase called but already saving, returning");
                        return;
                    }
                    
                    tracing::info!("[SPINNER] Setting is_saving = true");
                    self.is_saving = true;
                    
                    // Clone Arc for thread (cheap)
                    let db_arc = db.clone();
                    let sender_clone = sender.clone();
                    
                    std::thread::spawn(move || {
                        let start = std::time::Instant::now();
                        tracing::info!("[SPINNER] Background thread started, beginning save...");
                        
                        // Lock for reading in the thread
                        let res = if let Ok(db) = db_arc.read() {
                             db.save().map_err(|e| e.to_string())
                        } else {
                             Err("Failed to acquire database lock".to_string())
                        };
                        
                        let save_duration = start.elapsed();
                        tracing::info!("[SPINNER] Save operation took {:?}", save_duration);
                        
                        // Enforce at least 500ms delay for better UI UX (optional, but requested "freeze for a second or so" -> "Saving..." spinner)
                        // User complained about FREEZE, not speed. But if it's too fast, spinner flicks.
                        let min_display_time = std::time::Duration::from_millis(500);
                        if save_duration < min_display_time {
                            let remaining = min_display_time - save_duration;
                            tracing::info!("[SPINNER] Sleeping for additional {:?} to ensure spinner is visible", remaining);
                            std::thread::sleep(remaining);
                        }
                        
                        let total_duration = start.elapsed();
                        tracing::info!("[SPINNER] Total save+delay took {:?}, sending SaveFinished", total_duration);
                        
                        sender_clone.input(AppInput::SaveFinished(res));
                    });
                    
                    tracing::info!("[SPINNER] SaveDatabase handler returning, is_saving = {}", self.is_saving);
                }
            }
            AppInput::SaveFinished(result) => {
                tracing::info!("[SPINNER] SaveFinished received, setting is_saving = false");
                self.is_saving = false;
                match result {
                    Ok(_) => {
                        self.unsaved_changes = false;
                        // Use locale-aware time formatting with seconds (time only, no date)
                        // %X = locale's time (includes seconds)
                        self.last_save_time = chrono::Local::now()
                            .format_localized("%X", get_system_locale())
                            .to_string();
                        
                        // Update size
                        self.db_size = std::fs::metadata(&self.config.database_path)
                            .map(|m| format_size(m.len()))
                            .unwrap_or_else(|_| "Unknown".to_string());
                            
                        tracing::info!("Database saved successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to save database: {}", e);
                        // TODO: Show error
                    }
                }
            }
            AppInput::SidebarEntrySelected(entry_uuid) => {
                // Find parent group and entry
                if let Some(ref root) = self.root_group {
                    if let Some((group, entry)) = find_entry_and_group(root, &entry_uuid) {
                        sender.input(AppInput::SearchEntrySelected {
                            entry: entry.clone(),
                            group_uuid: group.uuid.clone(),
                        });
                    }
                }
            }
            AppInput::NoOp => {}
        }
        
        // IMPORTANT: Must call update_view to trigger #[watch] updates when using update_with_view
        self.update_view(widgets, sender);
    }
}

/// Find a group by UUID recursively.
fn find_group_by_uuid<'a>(group: &'a Group, uuid: &str) -> Option<&'a Group> {
    if group.uuid == uuid {
        return Some(group);
    }
    for child in &group.children {
        if let Some(found) = find_group_by_uuid(child, uuid) {
            return Some(found);
        }
    }
    None
}

/// Find an entry and its parent group by entry UUID.
fn find_entry_and_group<'a>(group: &'a Group, entry_uuid: &str) -> Option<(&'a Group, &'a Entry)> {
    for entry in &group.entries {
        if entry.uuid == entry_uuid {
            return Some((group, entry));
        }
    }
    for child in &group.children {
        if let Some(found) = find_entry_and_group(child, entry_uuid) {
            return Some(found);
        }
    }
    None
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Recursively count entries in a group.
fn count_entries(group: &Group) -> usize {
    let mut count = group.entries.len();
    for child in &group.children {
        count += count_entries(child);
    }
    count
}

/// Get the system locale for date/time formatting.
/// Reads from LC_TIME or LANG environment variables and maps to chrono::Locale.
fn get_system_locale() -> chrono::Locale {
    use chrono::Locale;
    
    // Try LC_TIME first, then LANG
    let lang = std::env::var("LC_TIME")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en_US".to_string());
    
    // Extract the language code (e.g., "it_IT.UTF-8" -> "it_IT" -> match to locale)
    let lang_code = lang.split('.').next().unwrap_or("en_US");
    
    // Map common locale codes to chrono::Locale
    match lang_code {
        s if s.starts_with("it") => Locale::it_IT,
        s if s.starts_with("de") => Locale::de_DE,
        s if s.starts_with("fr") => Locale::fr_FR,
        s if s.starts_with("es") => Locale::es_ES,
        s if s.starts_with("pt") => Locale::pt_PT,
        s if s.starts_with("nl") => Locale::nl_NL,
        s if s.starts_with("pl") => Locale::pl_PL,
        s if s.starts_with("ru") => Locale::ru_RU,
        s if s.starts_with("ja") => Locale::ja_JP,
        s if s.starts_with("zh") => Locale::zh_CN,
        s if s.starts_with("ko") => Locale::ko_KR,
        s if s.starts_with("en_GB") => Locale::en_GB,
        _ => Locale::en_US, // Default fallback
    }
}
