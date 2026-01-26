//! Sidebar component with folder tree.

use keeprs_core::{Entry, Group};
use gtk4::prelude::*;
use relm4::prelude::*;

use std::collections::HashSet;
use gtk4::cairo::Context;

/// Messages for the sidebar.
#[derive(Debug)]
pub enum SidebarInput {
    /// Set the root group to display.
    SetRootGroup(Group),
    /// A group was selected.
    SelectGroup(String),
    /// An entry was selected.
    SelectEntry(String),
    /// Update visual selection.
    UpdateSelection(String),
    /// Toggle expansion of a group.
    ToggleExpand(String),
}

/// Output messages from the sidebar.
#[derive(Debug, Clone)]
pub enum SidebarOutput {
    /// User selected a group.
    GroupSelected(String),
    /// User selected an entry.
    EntrySelected(String),
}

/// Sidebar model.
pub struct Sidebar {
    root_group: Option<Group>,
    selected_uuid: Option<String>,
    expanded_uuids: HashSet<String>,
    hidden_groups: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct SidebarInit {
    pub initial_width: i32,
    pub min_width: i32,
    pub hidden_groups: Vec<String>,
}

#[relm4::component(pub)]
impl Component for Sidebar {
    type Init = SidebarInit;
    type Input = SidebarInput;
    type Output = SidebarOutput;
    type CommandOutput = ();

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk4::PolicyType::Automatic,
            set_vscrollbar_policy: gtk4::PolicyType::Automatic,
            set_vexpand: true,
            set_propagate_natural_width: true,
            set_min_content_width: init.min_width,
            set_max_content_width: init.initial_width,

            #[name = "list_box"]
            gtk4::ListBox {
                add_css_class: "navigation-sidebar",
                set_selection_mode: gtk4::SelectionMode::Single,

                connect_row_activated[sender] => move |_, row| {
                    let name = row.widget_name();
                    if let Some(uuid) = name.as_str().strip_prefix("group-") {
                        sender.input(SidebarInput::SelectGroup(uuid.to_string()));
                    } else if let Some(uuid) = name.as_str().strip_prefix("entry-") {
                        sender.input(SidebarInput::SelectEntry(uuid.to_string()));
                    }
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        tracing::info!("Sidebar::init received initial_width: {}, min_width: {}", init.initial_width, init.min_width);
        let model = Sidebar {
            root_group: None,
            selected_uuid: None,
            expanded_uuids: HashSet::new(),
            hidden_groups: init.hidden_groups.into_iter().collect(),
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
                self.root_group = Some(group);
                self.expanded_uuids.clear();
                 if let Some(root) = &self.root_group {
                    // Always expand root
                    self.expanded_uuids.insert(root.uuid.clone());
                    // Expand top level items? Maybe not enforced.
                }
                self.rebuild_list(widgets, sender);
            }
            SidebarInput::SelectGroup(uuid) => {
                self.selected_uuid = Some(uuid.clone());
                let _ = sender.output(SidebarOutput::GroupSelected(uuid));
            }
            SidebarInput::SelectEntry(uuid) => {
                self.selected_uuid = Some(uuid.clone());
                let _ = sender.output(SidebarOutput::EntrySelected(uuid));
            }
            SidebarInput::UpdateSelection(uuid) => {
                self.selected_uuid = Some(uuid.clone());
                
                // Auto-expand parents
                if let Some(root) = &self.root_group {
                    let mut path = Vec::new();
                    if Sidebar::find_path_recursive(root, &uuid, &mut path) {
                        for p in path {
                             self.expanded_uuids.insert(p);
                        }
                        self.rebuild_list(widgets, sender);
                    }
                }

                self.select_row_by_uuid(&widgets.list_box, &uuid);
            }
            SidebarInput::ToggleExpand(uuid) => {
                if self.expanded_uuids.contains(&uuid) {
                    self.expanded_uuids.remove(&uuid);
                } else {
                    self.expanded_uuids.insert(uuid);
                }
                self.rebuild_list(widgets, sender);
            }
        }
    }
}

impl Sidebar {
    fn rebuild_list(&self, widgets: &mut <Sidebar as Component>::Widgets, sender: ComponentSender<Sidebar>) {
        while let Some(row) = widgets.list_box.row_at_index(0) {
            widgets.list_box.remove(&row);
        }

        if let Some(root) = &self.root_group {
            let mut levels = Vec::new();
            // Root itself is usually hidden in 2-pane abstract, but here we render children of root.
            // Wait, standard sidebar hides the root folder if it's just a container. 
            // Previous implementation rendered children of root.
            // We need to iterate over BOTH children and entries of root.
            
            let total_count = root.children.len() + root.entries.len();
            let mut current_idx = 0;

            for child in &root.children {
                let is_last = current_idx == total_count - 1;
                self.add_group_node(&widgets.list_box, child, &mut levels, is_last, &sender);
                current_idx += 1;
            }
            for entry in &root.entries {
                let is_last = current_idx == total_count - 1;
                self.add_entry_node(&widgets.list_box, entry, &mut levels, is_last);
                current_idx += 1;
            }
        }

        if let Some(uuid) = &self.selected_uuid {
            self.select_row_by_uuid(&widgets.list_box, uuid);
        }
    }

    fn select_row_by_uuid(&self, list_box: &gtk4::ListBox, uuid: &str) {
         let group_name = format!("group-{}", uuid);
         let entry_name = format!("entry-{}", uuid);
         
         let mut child = list_box.first_child();
         while let Some(widget) = child {
             if let Some(row) = widget.downcast_ref::<gtk4::ListBoxRow>() {
                 let name = row.widget_name();
                 if name == group_name || name == entry_name {
                     list_box.select_row(Some(row));
                     row.grab_focus();
                     return;
                 }
             }
             child = widget.next_sibling();
         }
    }

    fn find_path_recursive(group: &Group, target_uuid: &str, path: &mut Vec<String>) -> bool {
        if group.uuid == *target_uuid {
            return true;
        }
        
        // check entries
        for entry in &group.entries {
            if entry.uuid == target_uuid {
                return true;
            }
        }
        
        path.push(group.uuid.clone());
        for child in &group.children {
            if Self::find_path_recursive(child, target_uuid, path) {
                return true;
            }
        }
        path.pop();
        
        false
    }

    fn add_entry_node(
        &self,
        list_box: &gtk4::ListBox,
        entry: &Entry,
        levels: &mut Vec<bool>,
        is_last: bool,
    ) {
        if self.hidden_groups.contains(&entry.title) {
            return;
        }

        let row = gtk4::ListBoxRow::new();
        row.set_widget_name(&format!("entry-{}", entry.uuid));
        row.add_css_class("sidebar-row");

        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        
        // Lines
        let depth = levels.len();
        let indent_width = (depth + 1) * 24; 
        
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(indent_width as i32);
        drawing_area.set_content_height(32);
        drawing_area.set_vexpand(true);
        
        let levels_clone = levels.clone();
        let is_last_clone = is_last;
        
        drawing_area.set_draw_func(move |_area, cr: &Context, _width, height| {
             cr.set_source_rgba(0.6, 0.6, 0.6, 0.5);
             cr.set_line_width(1.0);
             let indent = 24.0;
             let half_indent = 12.0;

             for (i, &parent_is_last) in levels_clone.iter().enumerate() {
                 if !parent_is_last {
                     let x = i as f64 * indent + half_indent;
                     cr.move_to(x, -2.0);
                     cr.line_to(x, height as f64 + 2.0);
                     cr.stroke().expect("Invalid cairo");
                 }
             }

             let current_x = depth as f64 * indent + half_indent;
             cr.move_to(current_x, -2.0);
             if is_last_clone {
                 cr.line_to(current_x, height as f64 / 2.0);
             } else {
                 cr.line_to(current_x, height as f64 + 2.0);
             }
             cr.stroke().expect("Invalid cairo");

             cr.move_to(current_x, height as f64 / 2.0);
             cr.line_to(current_x + half_indent + 4.0, height as f64 / 2.0);
             cr.stroke().expect("Invalid cairo");
        });

        hbox.append(&drawing_area);

        // Icon
        let icon = gtk4::Image::from_icon_name("dialog-password-symbolic"); // Key icon
        icon.set_margin_start(12);
        icon.set_margin_end(8);
        hbox.append(&icon);

        // Name
        let label = gtk4::Label::new(Some(&entry.title));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        hbox.append(&label);

        row.set_child(Some(&hbox));
        list_box.append(&row);
    }

    fn add_group_node(
        &self,
        list_box: &gtk4::ListBox,
        group: &Group,
        levels: &mut Vec<bool>,
        is_last: bool,
        sender: &ComponentSender<Sidebar>,
    ) {
        if self.hidden_groups.contains(&group.name) {
            return;
        }

        let row = gtk4::ListBoxRow::new();
        row.set_widget_name(&format!("group-{}", group.uuid));
        row.add_css_class("sidebar-row");

        let overlay = gtk4::Overlay::new();
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        
        let depth = levels.len();
        let indent_width = (depth + 1) * 24; 
        
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(indent_width as i32);
        drawing_area.set_content_height(32);
        drawing_area.set_vexpand(true);
        
        let levels_clone = levels.clone();
        let is_last_clone = is_last;
        
        drawing_area.set_draw_func(move |_area, cr: &Context, _width, height| {
             cr.set_source_rgba(0.6, 0.6, 0.6, 0.5);
             cr.set_line_width(1.0);
             let indent = 24.0;
             let half_indent = 12.0;

             for (i, &parent_is_last) in levels_clone.iter().enumerate() {
                 if !parent_is_last {
                     let x = i as f64 * indent + half_indent;
                     cr.move_to(x, -2.0);
                     cr.line_to(x, height as f64 + 2.0);
                     cr.stroke().expect("Invalid cairo");
                 }
             }

             let current_x = depth as f64 * indent + half_indent;
             cr.move_to(current_x, -2.0);
             if is_last_clone {
                 cr.line_to(current_x, height as f64 / 2.0);
             } else {
                 cr.line_to(current_x, height as f64 + 2.0);
             }
             cr.stroke().expect("Invalid cairo");

             cr.move_to(current_x, height as f64 / 2.0);
             cr.line_to(current_x + half_indent + 4.0, height as f64 / 2.0);
             cr.stroke().expect("Invalid cairo");
        });

        hbox.append(&drawing_area);

        let icon_name = if self.expanded_uuids.contains(&group.uuid) && (!group.children.is_empty() || !group.entries.is_empty()) {
            "folder-open-symbolic"
        } else {
            "folder-symbolic"
        };
        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_margin_start(12);
        icon.set_margin_end(8);
        hbox.append(&icon);

        let label = gtk4::Label::new(Some(&group.name));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        hbox.append(&label);

        // Badge: Entry count (only if not expanded?)
        // Let's show total count
        let count = group.entries.len();
        if count > 0 {
             let badge = gtk4::Label::new(Some(&count.to_string()));
             badge.add_css_class("dim-label");
             badge.set_margin_end(8);
             hbox.append(&badge);
        }

        overlay.set_child(Some(&hbox));

        // Expander: Show if has children OR entries
        if !group.children.is_empty() || !group.entries.is_empty() {
             let expander_btn = gtk4::Button::new();
             expander_btn.add_css_class("flat");
             expander_btn.add_css_class("circular");
             
             let arrow_icon = if self.expanded_uuids.contains(&group.uuid) {
                 "pan-down-symbolic"
             } else {
                 "pan-end-symbolic"
             };
             expander_btn.set_icon_name(arrow_icon);
             expander_btn.set_halign(gtk4::Align::Start);
             expander_btn.set_valign(gtk4::Align::Center);
             expander_btn.set_margin_start((depth as i32) * 24);
             expander_btn.set_width_request(24);
             expander_btn.set_height_request(24);

             let sender_clone = sender.clone();
             let uuid_clone = group.uuid.clone();
             expander_btn.connect_clicked(move |_| {
                 sender_clone.input(SidebarInput::ToggleExpand(uuid_clone.clone()));
             });
             
             overlay.add_overlay(&expander_btn);
        }

        row.set_child(Some(&overlay));
        list_box.append(&row);

        if self.expanded_uuids.contains(&group.uuid) {
            levels.push(is_last);
            let total_child_count = group.children.len() + group.entries.len();
            let mut current_child_idx = 0;

            for child in &group.children {
                let child_is_last = current_child_idx == total_child_count - 1;
                self.add_group_node(list_box, child, levels, child_is_last, sender);
                current_child_idx += 1;
            }
            for entry in &group.entries {
                let child_is_last = current_child_idx == total_child_count - 1;
                self.add_entry_node(list_box, entry, levels, child_is_last);
                current_child_idx += 1;
            }
            levels.pop();
        }
    }
}
