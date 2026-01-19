//! Sidebar component with folder tree.

use crate::models::Group;
use gtk4::prelude::*;
use relm4::prelude::*;

use std::collections::HashSet;
use gtk4::cairo::Context;

/// Messages for the sidebar.
#[derive(Debug)]
pub enum SidebarInput {
    /// Set the root group to display.
    SetRootGroup(Group),
    /// A group was selected (internal or external).
    SelectGroup(String),
    /// Update visual selection (without emitting output).
    UpdateSelection(String),
    /// Toggle expansion of a group.
    ToggleExpand(String),
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
    selected_uuid: Option<String>,
    expanded_uuids: HashSet<String>,
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
            set_vexpand: true,

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
            selected_uuid: None,
            expanded_uuids: HashSet::new(),
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
                // Expand root children by default? Let's just expand the root itself (which is invisible)
                // Actually, let's start with clean slate.
                self.expanded_uuids.clear();
                // We might want to expand the top-level groups by default
                 if let Some(root) = &self.root_group {
                    for child in &root.children {
                        self.expanded_uuids.insert(child.uuid.clone());
                    }
                }
                self.rebuild_list(widgets, sender);
            }
            SidebarInput::SelectGroup(uuid) => {
                self.selected_uuid = Some(uuid.clone());
                let _ = sender.output(SidebarOutput::GroupSelected(uuid));
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
        // Clear existing
        while let Some(row) = widgets.list_box.row_at_index(0) {
            widgets.list_box.remove(&row);
        }

        if let Some(root) = &self.root_group {
            let mut levels = Vec::new();
            let count = root.children.len();
            for (i, child) in root.children.iter().enumerate() {
                let is_last = i == count - 1;
                self.add_node(&widgets.list_box, child, &mut levels, is_last, &sender);
            }
        }

        // Restore selection
        if let Some(uuid) = &self.selected_uuid {
            self.select_row_by_uuid(&widgets.list_box, uuid);
        }
    }

    fn select_row_by_uuid(&self, list_box: &gtk4::ListBox, uuid: &str) {
         let row_name = format!("group-{}", uuid);
         let mut child = list_box.first_child();
         while let Some(widget) = child {
             if let Some(row) = widget.downcast_ref::<gtk4::ListBoxRow>() {
                 if row.widget_name() == row_name {
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
        
        path.push(group.uuid.clone());
        for child in &group.children {
            if Self::find_path_recursive(child, target_uuid, path) {
                return true;
            }
        }
        path.pop();
        
        false
    }

    fn add_node(
        &self,
        list_box: &gtk4::ListBox,
        group: &Group,
        levels: &mut Vec<bool>, // stores is_last_child for each level
        is_last: bool,
        sender: &ComponentSender<Sidebar>,
    ) {
        let row = gtk4::ListBoxRow::new();
        row.set_widget_name(&format!("group-{}", group.uuid));
        row.add_css_class("sidebar-row");

        // Use Overlay to potentiall place expander on top of lines
        let overlay = gtk4::Overlay::new();

        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        
        // 1. Indentation & Lines Drawing Area
        // Width: (depth) * 20 (lines) + 20 (icon space)
        let depth = levels.len();
        let indent_width = (depth + 1) * 24; 
        
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(indent_width as i32);
        drawing_area.set_content_height(32); // Fixed row height approx
        drawing_area.set_vexpand(true); // Ensure it fills the row vertically to connect lines
        
        let levels_clone = levels.clone();
        let is_last_clone = is_last;
        
        drawing_area.set_draw_func(move |_area, cr: &Context, width, height| {
             cr.set_source_rgba(0.6, 0.6, 0.6, 0.5); // Grey lines
             cr.set_line_width(1.0);
             let indent = 24.0;
             let half_indent = 12.0;

             // Draw vertical lines for parent levels
             for (i, &parent_is_last) in levels_clone.iter().enumerate() {
                 if !parent_is_last {
                     let x = i as f64 * indent + half_indent;
                     cr.move_to(x, -2.0); // Extend up
                     cr.line_to(x, height as f64 + 2.0); // Extend down
                     cr.stroke().expect("Invalid cairo");
                 }
             }

             // Draw connectivity for current node
             let current_x = depth as f64 * indent + half_indent;
             
             // Vertical part
             cr.move_to(current_x, -2.0); // Connect to parent above
             if is_last_clone {
                 cr.line_to(current_x, height as f64 / 2.0);
             } else {
                 cr.line_to(current_x, height as f64 + 2.0); // Connect to next sibling
             }
             cr.stroke().expect("Invalid cairo");

             // Horizontal part (T-junction)
             cr.move_to(current_x, height as f64 / 2.0);
             cr.line_to(current_x + half_indent + 4.0, height as f64 / 2.0);
             cr.stroke().expect("Invalid cairo");
        });

        hbox.append(&drawing_area);

        // Icon
        let icon_name = if self.expanded_uuids.contains(&group.uuid) && !group.children.is_empty() {
            "folder-open-symbolic"
        } else {
            "folder-symbolic"
        };
        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_margin_start(12); // Avoid overlap with expander hover
        icon.set_margin_end(8);
        hbox.append(&icon);

        // Name
        let label = gtk4::Label::new(Some(&group.name));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        hbox.append(&label);

        // Badge
        if !group.entries.is_empty() {
             let badge = gtk4::Label::new(Some(&group.entries.len().to_string()));
             badge.add_css_class("dim-label");
             badge.set_margin_end(8);
             hbox.append(&badge);
        }

        overlay.set_child(Some(&hbox));

        // Expander Button (if children exist)
        if !group.children.is_empty() {
             let expander_btn = gtk4::Button::new();
             expander_btn.add_css_class("flat");
             expander_btn.add_css_class("circular");
             
             let arrow_icon = if self.expanded_uuids.contains(&group.uuid) {
                 "pan-down-symbolic"
             } else {
                 "pan-end-symbolic"
             };
             expander_btn.set_icon_name(arrow_icon);
             
             // Position it over the junction
             // We use Margin Start to position it
             expander_btn.set_halign(gtk4::Align::Start);
             expander_btn.set_valign(gtk4::Align::Center);
             // Depth * 24 is where the junction is.
             // We want button distinct?
             // Actually, usually the arrow IS the junction or next to it.
             // Let's put it at `depth * 24`.
             expander_btn.set_margin_start((depth as i32) * 24);
             // Make it small
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

        // Recursion
        if self.expanded_uuids.contains(&group.uuid) {
            levels.push(is_last);
            let child_count = group.children.len();
            for (i, child) in group.children.iter().enumerate() {
                self.add_node(list_box, child, levels, i == child_count - 1, sender);
            }
            levels.pop();
        }
    }
}
