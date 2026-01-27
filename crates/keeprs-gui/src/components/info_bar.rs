//! Bottom info bar component.
//! 
//! Displays database info (filename, entry count, size) on the left
//! and save status (unsaved indicator, last save time, spinner) on the right.

use gtk4::prelude::*;
use relm4::prelude::*;

/// Get the system locale for date/time formatting.
/// Reads from LC_TIME or LANG environment variables and maps to chrono::Locale.
fn get_system_locale() -> chrono::Locale {
    use chrono::Locale;
    
    // Try LC_TIME first, then LANG
    let lang = std::env::var("LC_TIME")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en_US".to_string());
    
    // Extract the language code (e.g., "it_IT.UTF-8" -> "it_IT" -> match to locale)
    let lang_code = lang.split('.').next().unwrap_or("en_US");
    
    // Map common locale codes to chrono::Locale
    match lang_code {
        s if s.starts_with("it") => Locale::it_IT,
        s if s.starts_with("de") => Locale::de_DE,
        s if s.starts_with("fr") => Locale::fr_FR,
        s if s.starts_with("es") => Locale::es_ES,
        s if s.starts_with("pt") => Locale::pt_PT,
        s if s.starts_with("nl") => Locale::nl_NL,
        s if s.starts_with("pl") => Locale::pl_PL,
        s if s.starts_with("ru") => Locale::ru_RU,
        s if s.starts_with("ja") => Locale::ja_JP,
        s if s.starts_with("zh") => Locale::zh_CN,
        s if s.starts_with("ko") => Locale::ko_KR,
        s if s.starts_with("en_GB") => Locale::en_GB,
        _ => Locale::en_US, // Default fallback
    }
}

/// Format the current time for display in the info bar.
pub fn format_save_time() -> String {
    chrono::Local::now()
        .format_localized("%X", get_system_locale())
        .to_string()
}

/// Input messages for the info bar.
#[derive(Debug)]
pub enum InfoBarInput {
    /// Update database filename.
    SetFilename(Option<String>),
    /// Update full database path (for tooltip).
    SetFullPath(String),
    /// Update entry count.
    SetEntryCount(usize),
    /// Update database size string.
    SetDbSize(String),
    /// Set unsaved changes flag.
    SetUnsavedChanges(bool),
    /// Set saving state.
    SetSaving(bool),
    /// Set last save time.
    SetLastSaveTime(String),
}

/// Info bar model state.
pub struct InfoBar {
    db_filename: Option<String>,
    db_full_path: String,
    entry_count: usize,
    db_size: String,
    unsaved_changes: bool,
    is_saving: bool,
    last_save_time: String,
}

#[relm4::component(pub)]
impl Component for InfoBar {
    type Init = ();
    type Input = InfoBarInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 0,
            
            // Separator line
            gtk4::Separator {
                set_orientation: gtk4::Orientation::Horizontal,
            },
            
            // Main info bar content
            gtk4::CenterBox {
                set_margin_all: 4,
                set_margin_start: 8,
                set_margin_end: 8,
                set_hexpand: true,
                
                // Left: Filename + Entry Count + Size
                #[wrap(Some)]
                set_start_widget = &gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 16,
                    
                    // Database Name
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 6,
                        
                        gtk4::Image {
                            set_icon_name: Some("folder-open-symbolic"),
                            add_css_class: "dim-label",
                        },
                        gtk4::Label {
                            #[watch]
                            set_label: model.db_filename.as_deref().unwrap_or(""),
                            add_css_class: "dim-label",
                            #[watch]
                            set_tooltip_text: if model.db_full_path.is_empty() { None } else { Some(&model.db_full_path) },
                        },
                    },
                    
                    // Entry Count
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 6,
                        
                        gtk4::Image {
                            set_icon_name: Some("view-list-symbolic"),
                            add_css_class: "dim-label",
                        },
                        gtk4::Label {
                            #[watch]
                            set_label: &format!("{} entries", model.entry_count),
                            add_css_class: "dim-label",
                        },
                    },
                    
                    // Database Size
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 6,
                        
                        gtk4::Image {
                            set_icon_name: Some("drive-harddisk-symbolic"),
                            add_css_class: "dim-label",
                        },
                        gtk4::Label {
                            #[watch]
                            set_label: &model.db_size,
                            add_css_class: "dim-label",
                        },
                    },
                },

                // Right: Unsaved Indicator + Last saved status
                #[wrap(Some)]
                set_end_widget = &gtk4::Box {
                    set_orientation: gtk4::Orientation::Horizontal,
                    set_spacing: 12,

                    // Unsaved changes indicator (only show dot when there are unsaved changes)
                    gtk4::Label {
                        #[watch]
                        set_label: if model.unsaved_changes { "●" } else { "" },
                        add_css_class: "dim-label",
                    },

                    // Last Save Status - always visible
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 6,
                        
                        // Last saved text first (left side)
                        gtk4::Label {
                            #[watch]
                            set_label: &if model.is_saving {
                                "Saving".to_string()
                            } else if model.last_save_time.is_empty() {
                                "No changes".to_string()
                            } else {
                                format!("Last save: {}", model.last_save_time)
                            },
                            add_css_class: "dim-label",
                        },
                        
                        // Status icon on right: spinner (saving), checkmark (saved), dash (not saved yet)
                        gtk4::Spinner {
                            set_size_request: (16, 16),
                            set_spinning: true,
                            #[watch]
                            set_visible: model.is_saving,
                        },
                        gtk4::Image {
                            #[watch]
                            set_icon_name: Some(if model.last_save_time.is_empty() {
                                "content-loading-symbolic"
                            } else {
                                "object-select-symbolic"
                            }),
                            add_css_class: "dim-label",
                            #[watch]
                            set_visible: !model.is_saving,
                        },
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = InfoBar {
            db_filename: None,
            db_full_path: String::new(),
            entry_count: 0,
            db_size: String::new(),
            unsaved_changes: false,
            is_saving: false,
            last_save_time: String::new(),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            InfoBarInput::SetFilename(filename) => {
                self.db_filename = filename;
            }
            InfoBarInput::SetFullPath(path) => {
                self.db_full_path = path;
            }
            InfoBarInput::SetEntryCount(count) => {
                self.entry_count = count;
            }
            InfoBarInput::SetDbSize(size) => {
                self.db_size = size;
            }
            InfoBarInput::SetUnsavedChanges(unsaved) => {
                self.unsaved_changes = unsaved;
            }
            InfoBarInput::SetSaving(saving) => {
                self.is_saving = saving;
            }
            InfoBarInput::SetLastSaveTime(time) => {
                self.last_save_time = time;
            }
        }
    }
}
