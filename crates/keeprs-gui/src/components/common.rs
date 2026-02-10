use gtk4::prelude::*;


/// Create a standardized "Primary" action button (e.g. Add).
/// 
/// Returns a button with the "suggested-action" class, the given label, and icon.
pub fn create_primary_button(label: &str, primary_icon_name: &str) -> gtk4::Button {
    let btn = gtk4::Button::new();
    btn.add_css_class("suggested-action");
    
    let box_layout = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    box_layout.set_halign(gtk4::Align::Center);
    
    // Primary Icon
    let icon1 = gtk4::Image::from_icon_name(primary_icon_name);
    box_layout.append(&icon1);
    
    // Plus Icon
    // let icon2 = gtk4::Image::from_icon_name("list-add-symbolic");
    // box_layout.append(&icon2);
    
    // Label
    let label_widget = gtk4::Label::new(Some(label));
    box_layout.append(&label_widget);
    
    btn.set_child(Some(&box_layout));
    
    btn
}

/// Create a standardized "Primary" action button with a composite icon.
pub fn create_composite_button(
    label: &str, 
    base_icon: &str, 
    specifier_icon: &str,
    corner: crate::widgets::composite_icon::CompositeIconCorner,
    x_offset: i32,
    y_offset: i32
) -> gtk4::Button {
    let btn = gtk4::Button::new();
    btn.add_css_class("suggested-action");
    
    let box_layout = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    box_layout.set_halign(gtk4::Align::Center);
    
    // Composite Icon
    let icon = crate::widgets::composite_icon::CompositeIcon::new(
        base_icon,
        specifier_icon,
        corner,
        x_offset,
        y_offset
    );
    box_layout.append(&icon);
    
    // Label
    let label_widget = gtk4::Label::new(Some(label));
    box_layout.append(&label_widget);
    
    btn.set_child(Some(&box_layout));
    
    btn
}
/// Create a standardized "Label + Text Entry" row.
/// Returns the container Box and the Entry widget (so signals can be connected).
pub fn create_text_entry_row(label: &str, value: &str) -> (gtk4::Box, gtk4::Entry) {
    let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    let label_widget = gtk4::Label::new(Some(label));
    label_widget.add_css_class("dim-label");
    label_widget.set_halign(gtk4::Align::Start);
    row.append(&label_widget);

    let entry = gtk4::Entry::new();
    entry.set_text(value);
    entry.set_hexpand(true);

    row.append(&entry);
    (row, entry)
}

/// Create a standardized "Label + Password Entry" row.
/// Returns the container Box and the PasswordEntry widget.
pub fn create_password_entry_row(label: &str, value: &str) -> (gtk4::Box, gtk4::PasswordEntry) {
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

    row.append(&entry);
    (row, entry)
}

/// Create a standardized "Label + Details/Notes" row.
/// Returns the container Box and the TextView widget.
pub fn create_text_area_row(label: &str, value: &str) -> (gtk4::Box, gtk4::TextView) {
    let row = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    let label_widget = gtk4::Label::new(Some(label));
    label_widget.add_css_class("dim-label");
    label_widget.set_halign(gtk4::Align::Start);
    row.append(&label_widget);

    let frame = gtk4::Frame::new(None);
    frame.set_height_request(100);

    let text_view = gtk4::TextView::new();
    text_view.set_wrap_mode(gtk4::WrapMode::Word);
    text_view.set_left_margin(8);
    text_view.set_right_margin(8);
    text_view.set_top_margin(8);
    text_view.set_bottom_margin(8);
    
    text_view.buffer().set_text(value);

    frame.set_child(Some(&text_view));
    row.append(&frame);
    
    (row, text_view)
}
