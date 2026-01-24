//! Main application component.

use crate::components::column_view::{ColumnView, ColumnViewInput, ColumnViewOutput};
use crate::components::entry_edit::{EntryEdit, EntryEditInput, EntryEditOutput};
use crate::components::search_palette::{SearchPalette, SearchPaletteInput, SearchPaletteOutput};
use crate::components::sidebar::{Sidebar, SidebarInit, SidebarInput, SidebarOutput};
use crate::components::unlock::{UnlockDialog, UnlockInput, UnlockOutput};
use crate::config::Config;
use crate::database::KeepassDatabase;
use crate::models::{Entry, Group};

use gtk4::prelude::*;
use gtk4::gdk;
use relm4::prelude::*;
use std::sync::Arc;
use std::cell::RefCell;

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
    /// Toggle search palette visibility.
    ToggleSearch,
    /// No operation.
    NoOp,
}

/// Main application model.
pub struct App {
    state: AppState,
    config: Config,
    database: Option<Arc<RefCell<KeepassDatabase>>>,
    current_group_uuid: Option<String>,
    root_group: Option<Group>,

    // Child components
    unlock: Controller<UnlockDialog>,
    search_palette: Controller<SearchPalette>,
    sidebar: Controller<Sidebar>,
    column_view: Controller<ColumnView>,
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
                add_child = &gtk4::Overlay {
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

                        // Right side: column view
                        #[wrap(Some)]
                        set_end_child = model.column_view.widget(),
                    },

                    add_overlay = model.search_palette.widget(),
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
            .launch(())
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

        let column_view = ColumnView::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                ColumnViewOutput::EntryEdited(entry) => AppInput::EntrySaved(entry),
                ColumnViewOutput::DeleteEntry(uuid) => AppInput::DeleteEntry(uuid),
                ColumnViewOutput::AddEntry => AppInput::AddEntry,
                ColumnViewOutput::SaveAttachment { filename, data } => AppInput::SaveAttachment { filename, data },
                ColumnViewOutput::OpenAttachment { filename, data } => AppInput::OpenAttachment { filename, data },
            });

        let entry_edit = EntryEdit::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                EntryEditOutput::Saved(entry) => AppInput::EntrySaved(entry),
                EntryEditOutput::Cancelled => AppInput::SaveDatabase, // No-op trigger
            });

        let model = App {
            state: AppState::Locked,
            config,
            database: None,
            current_group_uuid: None,
            root_group: None,
            unlock,
            search_palette,
            sidebar,
            column_view,
            entry_edit,
        };

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

        // Start on unlock screen
        widgets.main_stack.set_visible_child_name("unlock");

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
                        self.database = Some(Arc::new(RefCell::new(db)));
                        self.state = AppState::Unlocked;

                        // Populate sidebar and search
                        self.sidebar.emit(SidebarInput::SetRootGroup(root.clone()));
                        self.search_palette.emit(SearchPaletteInput::SetRootGroup(root.clone()));

                        // Set root group in column view
                        self.column_view.emit(ColumnViewInput::SetRootGroup(root.clone()));

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

                // Find the group and show its entries in column view
                if let Some(ref root) = self.root_group {
                    if let Some(group) = find_group_by_uuid(root, &uuid) {
                        self.column_view.emit(ColumnViewInput::SelectGroup {
                            uuid: uuid.clone(),
                            name: group.name.clone(),
                            group: group.clone(),
                        });
                    }
                }
            }
            AppInput::SearchGroupSelected { uuid, name, group } => {
                self.current_group_uuid = Some(uuid.clone());
                // Highlight in sidebar
                self.sidebar.emit(SidebarInput::UpdateSelection(uuid.clone()));
                
                self.column_view.emit(ColumnViewInput::SelectGroup { uuid, name, group });
            }
            AppInput::SearchEntrySelected { entry, group_uuid } => {
                // Select the group first, then the entry
                self.current_group_uuid = Some(group_uuid.clone());
                
                // Highlight in sidebar
                self.sidebar.emit(SidebarInput::UpdateSelection(group_uuid.clone()));
                
                if let Some(ref root) = self.root_group {
                    if let Some(group) = find_group_by_uuid(root, &group_uuid) {
                        self.column_view.emit(ColumnViewInput::SelectGroup {
                            uuid: group_uuid.clone(),
                            name: group.name.clone(),
                            group: group.clone(),
                        });
                        // Then select the entry
                        self.column_view.emit(ColumnViewInput::SelectEntry {
                            uuid: entry.uuid.clone(),
                            entry,
                        });
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
                
                if let Some(ref db) = self.database {
                    let mut db = db.borrow_mut();
                    
                    // Update entry in database
                    if let Err(e) = db.update_entry(&entry) {
                        tracing::error!("Failed to update entry: {}", e);
                        // TODO: Show error dialog
                        return;
                    }

                    // Save database to disk
                    if let Err(e) = db.save() {
                        tracing::error!("Failed to save database: {}", e);
                         // TODO: Show error dialog
                         return;
                    }
                    
                    tracing::info!("Database saved successfully");
                }

                // Refresh the view and re-select the entry
                if let Some(ref group_uuid) = self.current_group_uuid {
                    sender.input(AppInput::SearchEntrySelected { 
                        entry: entry.clone(), 
                        group_uuid: group_uuid.clone() 
                    });
                }
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
                    if let Err(e) = db.borrow().save() {
                        tracing::error!("Failed to save database: {}", e);
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
