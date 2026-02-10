use gtk4::prelude::*;
use gtk4::{glib, subclass::prelude::*};
use std::cell::RefCell;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "CompositeIconCorner")]
pub enum CompositeIconCorner {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

mod imp {
    use super::*;

#[derive(Debug, Default)]
    pub struct CompositeIcon {
        pub base_icon_name: RefCell<String>,
        pub specifier_icon_name: RefCell<String>,
        pub corner: RefCell<CompositeIconCorner>,
        pub x_offset: RefCell<i32>,
        pub y_offset: RefCell<i32>,
        
        pub base_image: RefCell<Option<gtk4::Image>>,
        pub specifier_image: RefCell<Option<gtk4::Image>>,
        // Wrapper for specifier to apply CSS background/halo
        pub specifier_wrapper: RefCell<Option<gtk4::Box>>,
        pub offset_provider: RefCell<Option<gtk4::CssProvider>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CompositeIcon {
        const NAME: &'static str = "CompositeIcon";
        type Type = super::CompositeIcon;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for CompositeIcon {
        fn constructed(&self) {
            self.parent_constructed();
            
            let obj = self.obj();
            
            // Configure box
            obj.set_orientation(gtk4::Orientation::Horizontal);
            
            // Create internal widgets
            let overlay = gtk4::Overlay::new();
            let base_image = gtk4::Image::new();
            
            let specifier_image = gtk4::Image::new();
            
            // Wrap specifier in a box to apply CSS halo
            let specifier_wrapper = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            specifier_wrapper.add_css_class("composite-icon-specifier-wrapper");
            specifier_wrapper.append(&specifier_image);
            
            // Add custom provider for offsets
            let offset_provider = gtk4::CssProvider::new();
            specifier_wrapper.style_context().add_provider(
                &offset_provider, 
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
            );
            
            // Setup structure
            overlay.set_child(Some(&base_image));
            overlay.add_overlay(&specifier_wrapper);
            
            // Add overlay to box
            obj.append(&overlay);
            
            // Keep references
            *self.base_image.borrow_mut() = Some(base_image);
            *self.specifier_image.borrow_mut() = Some(specifier_image);
            *self.specifier_wrapper.borrow_mut() = Some(specifier_wrapper);
            *self.offset_provider.borrow_mut() = Some(offset_provider);
            
            // Apply initial state
            self.update_icons();
            self.update_layout();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use glib::ParamFlags;
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("base-icon-name")
                        .default_value(None)
                        .flags(ParamFlags::READWRITE)
                        .build(),
                    glib::ParamSpecString::builder("specifier-icon-name")
                        .default_value(None)
                        .flags(ParamFlags::READWRITE)
                        .build(),
                    glib::ParamSpecEnum::builder::<CompositeIconCorner>("corner")
                        .default_value(CompositeIconCorner::default())
                        .flags(ParamFlags::READWRITE)
                        .build(),
                    glib::ParamSpecInt::builder("x-offset")
                        .default_value(0)
                        .flags(ParamFlags::READWRITE)
                        .build(),
                    glib::ParamSpecInt::builder("y-offset")
                        .default_value(0)
                        .flags(ParamFlags::READWRITE)
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
             match pspec.name() {
                "base-icon-name" => {
                    let name = value.get().expect("type checked upstream");
                    self.base_icon_name.replace(name);
                    self.update_icons();
                }
                "specifier-icon-name" => {
                    let name = value.get().expect("type checked upstream");
                    self.specifier_icon_name.replace(name);
                    self.update_icons();
                }
                "corner" => {
                    let corner = value.get().expect("type checked upstream");
                    self.corner.replace(corner);
                    self.update_layout();
                }
                "x-offset" => {
                    let offset = value.get().expect("type checked upstream");
                    self.x_offset.replace(offset);
                    self.update_layout();
                }
                "y-offset" => {
                    let offset = value.get().expect("type checked upstream");
                    self.y_offset.replace(offset);
                    self.update_layout();
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "base-icon-name" => self.base_icon_name.borrow().to_value(),
                "specifier-icon-name" => self.specifier_icon_name.borrow().to_value(),
                "corner" => self.corner.borrow().to_value(),
                "x-offset" => self.x_offset.borrow().to_value(),
                "y-offset" => self.y_offset.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    
    impl WidgetImpl for CompositeIcon {}
    impl BoxImpl for CompositeIcon {}
    
    impl CompositeIcon {
        pub fn update_icons(&self) {
            if let Some(img) = self.base_image.borrow().as_ref() {
                img.set_icon_name(Some(self.base_icon_name.borrow().as_str()));
            }
            if let Some(img) = self.specifier_image.borrow().as_ref() {
                img.set_icon_name(Some(self.specifier_icon_name.borrow().as_str()));
                // Increase size to 12px or 14px to fix "skinny" look
                img.set_pixel_size(12); 
            }
        }
        
        pub fn update_layout(&self) {
            // Apply alignment to the WRAPPER, not the image directly (since image is inside wrapper)
            if let Some(wrapper) = self.specifier_wrapper.borrow().as_ref() {
                // Mapping corners to alignment
                let (halign, valign) = match *self.corner.borrow() {
                    CompositeIconCorner::TopLeft => (gtk4::Align::Start, gtk4::Align::Start),
                    CompositeIconCorner::TopRight => (gtk4::Align::End, gtk4::Align::Start),
                    CompositeIconCorner::BottomLeft => (gtk4::Align::Start, gtk4::Align::End),
                    CompositeIconCorner::BottomRight => (gtk4::Align::End, gtk4::Align::End),
                };
                wrapper.set_halign(halign);
                wrapper.set_valign(valign);
                
                // Clear any manual margins
                wrapper.set_margin_start(0);
                wrapper.set_margin_end(0);
                wrapper.set_margin_top(0);
                wrapper.set_margin_bottom(0);
                
                // Use CSS transform for offset to avoid layout warnings about size constraints
                // +x = Right, +y = Up (visual)
                let x = *self.x_offset.borrow();
                let y = *self.y_offset.borrow();
                
                // CSS coordinate system: +y is Down. So +y (Up) -> translate(x, -y)
                let tx = x;
                let ty = -y;
                
                if let Some(provider) = self.offset_provider.borrow().as_ref() {
                    let css = format!(
                        ".composite-icon-specifier-wrapper {{ transform: translate({}px, {}px); }}", 
                        tx, ty
                    );
                    provider.load_from_data(&css);
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct CompositeIcon(ObjectSubclass<imp::CompositeIcon>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Orientable;
}

impl CompositeIcon {
    pub fn new(base_icon: &str, specifier_icon: &str, corner: CompositeIconCorner, x_offset: i32, y_offset: i32) -> Self {
        glib::Object::builder()
            .property("base-icon-name", base_icon)
            .property("specifier-icon-name", specifier_icon)
            .property("corner", corner)
            .property("x-offset", x_offset)
            .property("y-offset", y_offset)
            .build()
    }
}
