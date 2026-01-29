//! Group edit dialog component.
//!
//! Simple dialog to creating/editing groups (folders).

use keeprs_core::Group;
use gtk4::prelude::*;
use relm4::prelude::*;

/// Messages for group edit dialog.
#[derive(Debug)]
pub enum GroupEditInput {
    /// Open dialog to add a new group.
    AddNew,
    /// Name changed.
    NameChanged(String),
    /// Save the group.
    Save,
    /// Cancel editing.
    Cancel,
}

/// Output messages from group edit dialog.
#[derive(Debug, Clone)]
pub enum GroupEditOutput {
    /// Group was saved.
    Saved(Group),
    /// Dialog was cancelled.
    Cancelled,
}

/// Group edit model.
pub struct GroupEdit {
    group: Group,
    visible: bool,
}

#[relm4::component(pub)]
impl Component for GroupEdit {
    type Init = ();
    type Input = GroupEditInput;
    type Output = GroupEditOutput;
    type CommandOutput = ();

    view! {
        #[name = "dialog"]
        gtk4::Window {
            set_modal: true,
            set_default_width: 350,
            set_default_height: 200,
            set_title: Some("Add Folder"),
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
                        set_text: "Add Folder",
                        add_css_class: "title",
                    },

                    pack_start = &gtk4::Button {
                        set_label: "Cancel",
                        connect_clicked => GroupEditInput::Cancel,
                    },

                    pack_end = &gtk4::Button {
                        set_label: "Save",
                        add_css_class: "suggested-action",
                        connect_clicked => GroupEditInput::Save,
                    },
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 16,
                    set_margin_all: 24,

                    // Name field
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 4,

                        gtk4::Label {
                            set_text: "Name",
                            set_halign: gtk4::Align::Start,
                            add_css_class: "dim-label",
                        },

                        #[name = "name_entry"]
                        gtk4::Entry {
                            set_placeholder_text: Some("Folder name"),
                            #[watch]
                            set_text: &model.group.name,
                            connect_changed[sender] => move |entry| {
                                sender.input(GroupEditInput::NameChanged(entry.text().to_string()));
                            },
                            connect_activate[sender] => move |_| {
                                sender.input(GroupEditInput::Save);
                            }
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
        let model = GroupEdit {
            group: Group {
                uuid: String::new(),
                name: String::new(),
                children: Vec::new(),
                entries: Vec::new(),
                is_recycle_bin: false,
            },
            visible: false,
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
            GroupEditInput::AddNew => {
                self.group = Group {
                    uuid: String::new(), // new uuid will be assigned by backend
                    name: String::new(),
                    children: Vec::new(),
                    entries: Vec::new(),
                    is_recycle_bin: false,
                };
                self.visible = true;
                widgets.name_entry.set_text("");
                widgets.dialog.present();
                widgets.name_entry.grab_focus();
            }
            GroupEditInput::NameChanged(name) => {
                self.group.name = name;
            }
            GroupEditInput::Save => {
                if !self.group.name.is_empty() {
                    self.visible = false;
                    widgets.dialog.set_visible(false);
                    let _ = sender.output(GroupEditOutput::Saved(self.group.clone()));
                }
            }
            GroupEditInput::Cancel => {
                self.visible = false;
                widgets.dialog.set_visible(false);
                let _ = sender.output(GroupEditOutput::Cancelled);
            }
        }
    }
}
