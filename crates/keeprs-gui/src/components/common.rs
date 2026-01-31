use gtk4::prelude::*;

/// Create a standardized "Add" button with a composite icon.
/// 
/// The button will NOT have the "flat" class, ensuring a standard button background/border.
/// It contains a horizontal box with the primary icon + a "plus" symbol.
pub fn create_composite_add_button(tooltip: &str, primary_icon_name: &str) -> gtk4::Button {
    let btn = gtk4::Button::new();
    btn.set_tooltip_text(Some(tooltip));
    
    // Create composite icon content
    let box_layout = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    
    let icon1 = gtk4::Image::from_icon_name(primary_icon_name);
    box_layout.append(&icon1);
    
    let icon2 = gtk4::Image::from_icon_name("list-add-symbolic");
    box_layout.append(&icon2);
    
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
