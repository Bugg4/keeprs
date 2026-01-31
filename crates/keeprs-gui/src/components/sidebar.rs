//! Sidebar component with folder tree.

use keeprs_core::{Entry, Group};
use gtk4::prelude::*;
use relm4::prelude::*;
use crate::components::common::create_primary_button;

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
    /// Request to add a new group.
    AddGroup,
    /// Request to delete a group.
    DeleteGroup(String),
    DeleteEntry(String),
    /// Request to empty the recycle bin.
    EmptyRecycleBin(String),
    /// Request to permanently delete a group.
    PermanentDeleteGroup(String),
    /// Request to permanently delete an entry.
    PermanentDeleteEntry(String),
}

/// Output messages from the sidebar.
#[derive(Debug, Clone)]
pub enum SidebarOutput {
    /// User selected a group.
    GroupSelected(String),
    /// User selected an entry.
    EntrySelected(String),
    /// User requested to add a group.
    RequestAddGroup,
    /// User requested to delete a group.
    RequestDeleteGroup(String),
    /// User requested to delete an entry.
    RequestDeleteEntry(String),
    RequestEmptyRecycleBin(String),
    /// User requested to permanently delete a group.
    RequestPermanentDeleteGroup(String),
    /// User requested to permanently delete an entry.
    RequestPermanentDeleteEntry(String),
}

/// Sidebar model.
pub struct Sidebar {
    root_group: Option<Group>,
    selected_uuid: Option<String>,
    expanded_uuids: HashSet<String>,
    hidden_groups: HashSet<String>,
    context_menu: gtk4::PopoverMenu,
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
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 0,
            
            // Header
            gtk4::Box {
                set_orientation: gtk4::Orientation::Horizontal,
                set_spacing: 8,
                set_margin_all: 8,
                

                append = &create_primary_button("New Folder", "folder-symbolic") {
                    set_halign: gtk4::Align::Start,
                    set_tooltip_text: Some("New Folder"),
                    connect_clicked => SidebarInput::AddGroup,
                },
            },
            
            gtk4::Separator {
                set_orientation: gtk4::Orientation::Horizontal,
            },

            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk4::PolicyType::Automatic,
                set_vscrollbar_policy: gtk4::PolicyType::Automatic,
                set_vexpand: true,
                set_propagate_natural_width: true,
                set_min_content_width: init.min_width,
                set_max_content_width: init.initial_width,

            #[name = "_list_box"]
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
        
        let context_menu = gtk4::PopoverMenu::from_model(None::<&gtk4::gio::MenuModel>);
        context_menu.set_has_arrow(true);
        context_menu.set_position(gtk4::PositionType::Bottom);

        let model = Sidebar {
            root_group: None,
            selected_uuid: None,
            expanded_uuids: HashSet::new(),
            hidden_groups: init.hidden_groups.into_iter().collect(),
            context_menu,
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
                // Do NOT clear expanded_uuids here. We want to preserve state.
                // If new groups appear, they will be collapsed by default.
                // If old groups disappear, they remain in the set but won't be rendered (no harm).
                
                // Ensure root is expanded if it wasn't?
                 if let Some(root) = &self.root_group {
                    self.expanded_uuids.insert(root.uuid.clone());
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

                self.select_row_by_uuid(&widgets._list_box, &uuid);
            }
            SidebarInput::ToggleExpand(uuid) => {
                if self.expanded_uuids.contains(&uuid) {
                    self.expanded_uuids.remove(&uuid);
                } else {
                    self.expanded_uuids.insert(uuid);
                }
                self.rebuild_list(widgets, sender);
            }
            SidebarInput::AddGroup => {
                let _ = sender.output(SidebarOutput::RequestAddGroup);
            }
            SidebarInput::DeleteGroup(uuid) => {
                let _ = sender.output(SidebarOutput::RequestDeleteGroup(uuid));
            }
            SidebarInput::DeleteEntry(uuid) => {
                let _ = sender.output(SidebarOutput::RequestDeleteEntry(uuid));
            }
            SidebarInput::EmptyRecycleBin(uuid) => {
                let _ = sender.output(SidebarOutput::RequestEmptyRecycleBin(uuid));
            }
            SidebarInput::PermanentDeleteGroup(uuid) => {
                let _ = sender.output(SidebarOutput::RequestPermanentDeleteGroup(uuid));
            }
            SidebarInput::PermanentDeleteEntry(uuid) => {
                let _ = sender.output(SidebarOutput::RequestPermanentDeleteEntry(uuid));
            }
        }
    }
}

impl Sidebar {
    fn rebuild_list(&self, widgets: &mut <Sidebar as Component>::Widgets, sender: ComponentSender<Sidebar>) {
        // Ensure context menu is not parented to any row that is about to be destroyed
        self.context_menu.unparent();

        while let Some(row) = widgets._list_box.row_at_index(0) {
            widgets._list_box.remove(&row);
        }

        if let Some(root) = &self.root_group {
            let count = root.children.len() + root.entries.len();
            tracing::info!("Sidebar::rebuild_list root found. Children: {}, Entries: {}, Total: {}", root.children.len(), root.entries.len(), count);
            let mut levels = Vec::new();
            // Root itself is usually hidden in 2-pane abstract, but here we render children of root.
            // Wait, standard sidebar hides the root folder if it's just a container. 
            // Previous implementation rendered children of root.
            // We need to iterate over BOTH children and entries of root.
            
            let total_count = root.children.len() + root.entries.len();
            let mut current_idx = 0;

            for child in &root.children {
                let is_last = current_idx == total_count - 1;
                // Root children not under recycle bin unless root IS recycle bin? (Unlikely for root)
                let is_under_bin = root.is_recycle_bin;
                self.add_group_node(&widgets._list_box, child, &mut levels, is_last, &sender, is_under_bin, &self.context_menu);
                current_idx += 1;
            }
            for entry in &root.entries {
                let is_last = current_idx == total_count - 1;
                let is_under_bin = root.is_recycle_bin;
                self.add_entry_node(&widgets._list_box, entry, &mut levels, is_last, &sender, is_under_bin, &self.context_menu);
                current_idx += 1;
            }
        }

        if let Some(uuid) = &self.selected_uuid {
            self.select_row_by_uuid(&widgets._list_box, uuid);
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
        sender: &ComponentSender<Sidebar>,
        is_under_recycle_bin: bool,
        context_menu: &gtk4::PopoverMenu,
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

        // Right-click context menu
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(3); // Right mouse button
        let sender_clone = sender.clone();
        let uuid_clone = entry.uuid.clone();
        let context_menu_clone = context_menu.clone();
        gesture.connect_released(move |gesture, _n_press, x, y| {
            if let Some(widget) = gesture.widget() {
                Self::show_context_menu(&widget, x, y, &uuid_clone, false, &sender_clone, false, is_under_recycle_bin, &context_menu_clone);
            }
        });
        row.add_controller(gesture);

        list_box.append(&row);
    }

    fn add_placeholder_node(
        &self,
        list_box: &gtk4::ListBox,
        levels: &[bool],
    ) {
        let row = gtk4::ListBoxRow::new();
        row.set_activatable(false);
        row.set_selectable(false);
        row.add_css_class("sidebar-row");
        row.add_css_class("sidebar-placeholder");

        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        
        let depth = levels.len();
        let indent_width = (depth + 1) * 24; 
        
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(indent_width as i32);
        drawing_area.set_content_height(32);
        drawing_area.set_vexpand(true);
        
        let levels_clone = levels.to_vec();
        
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

             // Last branch
             let current_x = depth as f64 * indent + half_indent;
             cr.move_to(current_x, -2.0);
             cr.line_to(current_x, height as f64 / 2.0);
             cr.stroke().expect("Invalid cairo");

             cr.move_to(current_x, height as f64 / 2.0);
             cr.line_to(current_x + half_indent + 4.0, height as f64 / 2.0);
             cr.stroke().expect("Invalid cairo");
        });

        hbox.append(&drawing_area);

        let label = gtk4::Label::new(Some("- Empty -"));
        label.add_css_class("dim-label");
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(12);
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
        is_under_recycle_bin: bool,
        context_menu: &gtk4::PopoverMenu,
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
        } else if group.is_recycle_bin {
            "user-trash-symbolic"
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

        // Expander: Show always for groups (unless hidden logic applies, but here we show for all)
        // User requested expander even for empty groups
        // But maybe not for Recycle Bin leaf if it shouldn't expand? 
        // User said: "Do show expader arrow for recycle bin leafs too"
        {
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

        // Right-click context menu for groups
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(3); // Right mouse button
        let sender_clone = sender.clone();
        let uuid_clone = group.uuid.clone();
        let is_recycle_bin = group.is_recycle_bin;
        
        let in_bin_context = is_under_recycle_bin; // If we are under bin
        let context_menu_clone = context_menu.clone();

        gesture.connect_released(move |gesture, _n_press, x, y| {
            if let Some(widget) = gesture.widget() {
                Self::show_context_menu(&widget, x, y, &uuid_clone, true, &sender_clone, is_recycle_bin, in_bin_context, &context_menu_clone);
            }
        });
        row.add_controller(gesture);

        list_box.append(&row);

        if self.expanded_uuids.contains(&group.uuid) {
            levels.push(is_last);
            let total_child_count = group.children.len() + group.entries.len();
            
            if total_child_count == 0 {
                // Show placeholder
                self.add_placeholder_node(list_box, levels);
            } else {
                let mut current_child_idx = 0;
                
                let next_under_bin = is_under_recycle_bin || group.is_recycle_bin;

                for child in &group.children {
                    let child_is_last = current_child_idx == total_child_count - 1;
                    self.add_group_node(list_box, child, levels, child_is_last, sender, next_under_bin, context_menu);
                    current_child_idx += 1;
                }
                for entry in &group.entries {
                    let child_is_last = current_child_idx == total_child_count - 1;
                    self.add_entry_node(list_box, entry, levels, child_is_last, sender, next_under_bin, context_menu);
                    current_child_idx += 1;
                }
            }
            levels.pop();
        }
    }

    fn show_context_menu(
        widget: &gtk4::Widget,
        x: f64,
        y: f64,
        uuid: &str,
        is_group: bool,
        sender: &ComponentSender<Sidebar>,
        is_recycle_bin: bool,
        is_under_recycle_bin: bool,
        popover: &gtk4::PopoverMenu,
    ) {
        let menu_model = gtk4::gio::Menu::new();
        if is_recycle_bin {
             menu_model.append(Some("Empty Recycle Bin"), Some("ctx.empty"));
        } else if is_under_recycle_bin {
             menu_model.append(Some("Delete Permanently"), Some("ctx.delete_perm"));
        } else {
             menu_model.append(Some("Delete"), Some("ctx.delete"));
        }

        popover.set_menu_model(Some(&menu_model));
        
        // Parent the popover to the clicked widget OR the listbox row.
        // If we parent to the widget (e.g. overlay), coordinates are local. 
        // Important: Popover must have a parent to be realized.
        
        popover.unparent();
        popover.set_parent(widget);
        
        let target_x = x;
        let target_y = y;
        
        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(target_x as i32, target_y as i32, 1, 1)));
        
        // Define actions
        let action_group = gtk4::gio::SimpleActionGroup::new();
        
        let sender_clone = sender.clone();
        let uuid_clone = uuid.to_string();
        let action = gtk4::gio::SimpleAction::new("delete", None);
        action.connect_activate(move |_, _| {
             if is_recycle_bin {
                  sender_clone.input(SidebarInput::EmptyRecycleBin(uuid_clone.clone())); 
             } else if is_group {
                sender_clone.input(SidebarInput::DeleteGroup(uuid_clone.clone()));
            } else {
                sender_clone.input(SidebarInput::DeleteEntry(uuid_clone.clone()));
            }
        });
        action_group.add_action(&action);

        if is_recycle_bin || is_under_recycle_bin {
             let sender_clone = sender.clone();
             let uuid_clone = uuid.to_string();
             
             if is_recycle_bin {
                 let action = gtk4::gio::SimpleAction::new("empty", None);
                 action.connect_activate(move |_, _| {
                     sender_clone.input(SidebarInput::EmptyRecycleBin(uuid_clone.clone()));
                 });
                 action_group.add_action(&action);
             } else {
                 let action = gtk4::gio::SimpleAction::new("delete_perm", None);
                 action.connect_activate(move |_, _| {
                     if is_group {
                        sender_clone.input(SidebarInput::PermanentDeleteGroup(uuid_clone.clone()));
                     } else {
                        sender_clone.input(SidebarInput::PermanentDeleteEntry(uuid_clone.clone()));
                     }
                 });
                 action_group.add_action(&action);
             }
        }
        
        popover.insert_action_group("ctx", Some(&action_group));

        popover.popup();
    }
}
