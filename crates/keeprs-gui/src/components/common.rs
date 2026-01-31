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
