//! Sidebar component with folder tree.

use crate::models::Group;
use gtk4::prelude::*;
use relm4::prelude::*;

/// Messages for the sidebar.
#[derive(Debug)]
pub enum SidebarInput {
    /// Set the root group to display.
    SetRootGroup(Group),
    /// A group was selected.
    SelectGroup(String),
}

/// Output messages from the sidebar.
#[derive(Debug, Clone)]
pub enum SidebarOutput {
    /// User selected a group.
    GroupSelected(String),
}

/// Sidebar model.
pub struct Sidebar {
    root_group: Option<Group>,
}

#[relm4::component(pub)]
impl Component for Sidebar {
    type Init = ();
    type Input = SidebarInput;
    type Output = SidebarOutput;
    type CommandOutput = ();

    view! {
        gtk4::ScrolledWindow {
            set_hscrollbar_policy: gtk4::PolicyType::Never,
            set_vscrollbar_policy: gtk4::PolicyType::Automatic,
            set_width_request: 250,

            #[name = "list_box"]
            gtk4::ListBox {
                add_css_class: "navigation-sidebar",
                set_selection_mode: gtk4::SelectionMode::Single,

                connect_row_activated[sender] => move |_, row| {
                    // Get the UUID from the row's name
                    if let Some(name) = row.widget_name().as_str().strip_prefix("group-") {
                        sender.input(SidebarInput::SelectGroup(name.to_string()));
                    }
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Sidebar {
            root_group: None,
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
            SidebarInput::SetRootGroup(group) => {
                // Clear existing rows
                while let Some(row) = widgets.list_box.row_at_index(0) {
                    widgets.list_box.remove(&row);
                }

                // Add root group and children
                self.add_group_to_listbox(&widgets.list_box, &group, 0);
                self.root_group = Some(group);
            }
            SidebarInput::SelectGroup(uuid) => {
                let _ = sender.output(SidebarOutput::GroupSelected(uuid));
            }
        }
    }
}

impl Sidebar {
    fn add_group_to_listbox(
        &self,
        list_box: &gtk4::ListBox,
        group: &Group,
        depth: i32,
    ) {
        let row = gtk4::ListBoxRow::new();
        row.set_widget_name(&format!("group-{}", group.uuid));

        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        hbox.set_margin_start(12 + depth * 16);
        hbox.set_margin_end(12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        let icon = gtk4::Image::from_icon_name("folder-symbolic");
        hbox.append(&icon);

        let label = gtk4::Label::new(Some(&group.name));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        hbox.append(&label);

        // Show entry count badge
        if !group.entries.is_empty() {
            let badge = gtk4::Label::new(Some(&group.entries.len().to_string()));
            badge.add_css_class("dim-label");
            hbox.append(&badge);
        }

        row.set_child(Some(&hbox));
        list_box.append(&row);

        // Recursively add children
        for child in &group.children {
            self.add_group_to_listbox(list_box, child, depth + 1);
        }
    }
}
