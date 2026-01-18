//! Main application component.

use crate::components::column_view::{ColumnView, ColumnViewInput, ColumnViewOutput};
use crate::components::entry_edit::{EntryEdit, EntryEditInput, EntryEditOutput};
use crate::components::sidebar::{Sidebar, SidebarInput, SidebarOutput};
use crate::components::unlock::{UnlockDialog, UnlockInput, UnlockOutput};
use crate::config::Config;
use crate::database::KeepassDatabase;
use crate::models::{Entry, Group};

use gtk4::prelude::*;
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
    UnlockFailed(String),
    /// Database unlocked successfully.
    DatabaseUnlocked,
    /// Group selected in sidebar.
    GroupSelected(String),
    /// Entry actions.
    EditEntry(Entry),
    DeleteEntry(String),
    AddEntry,
    /// Entry saved from edit dialog.
    EntrySaved(Entry),
    /// Request to save the database.
    SaveDatabase,
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

                // Main view with sidebar and content
                add_child = &gtk4::Paned {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_position: 250,
                    set_shrink_start_child: false,
                    set_shrink_end_child: false,

                    #[wrap(Some)]
                    set_start_child = model.sidebar.widget(),

                    #[wrap(Some)]
                    set_end_child = model.column_view.widget(),
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

        let sidebar = Sidebar::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                SidebarOutput::GroupSelected(uuid) => AppInput::GroupSelected(uuid),
            });

        let column_view = ColumnView::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                ColumnViewOutput::EditEntry(entry) => AppInput::EditEntry(entry),
                ColumnViewOutput::DeleteEntry(uuid) => AppInput::DeleteEntry(uuid),
                ColumnViewOutput::AddEntry => AppInput::AddEntry,
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
            sidebar,
            column_view,
            entry_edit,
        };

        let widgets = view_output!();

        // Start on unlock screen
        widgets.main_stack.set_visible_child_name("unlock");

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

                        // Populate sidebar
                        self.sidebar.emit(SidebarInput::SetRootGroup(root.clone()));

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
            AppInput::UnlockFailed(error) => {
                self.unlock.emit(UnlockInput::ShowError(error));
            }
            AppInput::DatabaseUnlocked => {
                self.state = AppState::Unlocked;
                widgets.main_stack.set_visible_child_name("main");
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
            AppInput::EditEntry(entry) => {
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
                // TODO: Implement save through the database
                // For now, just refresh the view
                if let Some(ref group_uuid) = self.current_group_uuid {
                    sender.input(AppInput::GroupSelected(group_uuid.clone()));
                }
            }
            AppInput::SaveDatabase => {
                if let Some(ref db) = self.database {
                    if let Err(e) = db.borrow().save() {
                        tracing::error!("Failed to save database: {}", e);
                    }
                }
            }
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
