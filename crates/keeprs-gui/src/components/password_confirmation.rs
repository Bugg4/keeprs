//! Password confirmation dialog component.
//!
//! Modal dialog to confirm actions with master password.

use gtk4::prelude::*;
use relm4::prelude::*;

/// Messages for the password confirmation dialog.
#[derive(Debug)]
pub enum PasswordConfirmationInput {
    /// Show the dialog with a message.
    Show { message: String, action_id: String },
    /// Password text changed.
    PasswordChanged(String),
    /// Confirm action.
    Confirm,
    /// Cancel action.
    Cancel,
    /// Show error.
    ShowError(String),
}

/// Output messages.
#[derive(Debug, Clone)]
pub enum PasswordConfirmationOutput {
    /// User confirmed with password.
    Confirmed { password: String, action_id: String },
    /// Dialog was cancelled.
    Cancelled,
}

/// Component model.
pub struct PasswordConfirmation {
    password: String,
    message: String,
    action_id: String,
    error: Option<String>,
    visible: bool,
    processing: bool,
}

#[relm4::component(pub)]
impl Component for PasswordConfirmation {
    type Init = ();
    type Input = PasswordConfirmationInput;
    type Output = PasswordConfirmationOutput;
    type CommandOutput = ();

    view! {
        #[name = "dialog"]
        gtk4::Window {
            set_modal: true,
            set_default_width: 400,
            set_default_height: 250,
            set_title: Some("Confirm Action"),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(PasswordConfirmationInput::Cancel);
                gtk4::glib::Propagation::Stop
            },

            gtk4::Box {
                set_orientation: gtk4::Orientation::Vertical,
                set_spacing: 0,

                // Header bar
                gtk4::HeaderBar {
                    set_show_title_buttons: true,

                    #[wrap(Some)]
                    set_title_widget = &gtk4::Label {
                        set_text: "Confirm Action",
                        add_css_class: "title",
                    },
                },

                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_spacing: 16,
                    set_margin_all: 24,

                    // Message
                    gtk4::Label {
                        #[watch]
                        set_text: &model.message,
                        set_wrap: true,
                        set_halign: gtk4::Align::Start,
                    },

                    // Password field
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_spacing: 4,

                        gtk4::Label {
                            set_text: "Master Password",
                            set_halign: gtk4::Align::Start,
                            add_css_class: "dim-label",
                        },

                        #[name = "_password_entry"]
                        gtk4::PasswordEntry {
                            set_placeholder_text: Some("Enter master password"),
                            set_show_peek_icon: true,
                            
                            connect_changed[sender] => move |entry| {
                                sender.input(PasswordConfirmationInput::PasswordChanged(entry.text().to_string()));
                            },
                            
                            connect_activate[sender] => move |_| {
                                sender.input(PasswordConfirmationInput::Confirm);
                            }
                        },
                    },

                    // Error label
                    #[name = "error_label"]
                    gtk4::Label {
                        #[watch]
                        set_visible: model.error.is_some(),
                        #[watch]
                        set_text: model.error.as_deref().unwrap_or(""),
                        add_css_class: "error",
                        set_halign: gtk4::Align::Start,
                    },

                    // Buttons
                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Horizontal,
                        set_spacing: 10,
                        set_halign: gtk4::Align::End,
                        set_margin_top: 10,

                        gtk4::Button {
                            set_label: "Cancel",
                            connect_clicked => PasswordConfirmationInput::Cancel,
                        },

                        gtk4::Button {
                            set_label: "Confirm",
                            add_css_class: "destructive-action", // Usually these represent delete
                            #[watch]
                            set_sensitive: !model.processing && !model.password.is_empty(),
                            connect_clicked => PasswordConfirmationInput::Confirm,
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
        let model = PasswordConfirmation {
            password: String::new(),
            message: String::new(),
            action_id: String::new(),
            error: None,
            visible: false,
            processing: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PasswordConfirmationInput::Show { message, action_id } => {
                self.message = message;
                self.action_id = action_id;
                self.password.clear();
                self.error = None;
                self.visible = true;
                self.processing = false;
            }
            PasswordConfirmationInput::PasswordChanged(pwd) => {
                self.password = pwd;
                self.error = None;
            }
            PasswordConfirmationInput::Confirm => {
                if !self.password.is_empty() {
                    self.processing = true;
                    // Send output
                    let _ = sender.output(PasswordConfirmationOutput::Confirmed {
                        password: self.password.clone(),
                        action_id: self.action_id.clone(),
                    });
                }
            }
            PasswordConfirmationInput::Cancel => {
                self.visible = false;
                self.password.clear();
                let _ = sender.output(PasswordConfirmationOutput::Cancelled);
            }
            PasswordConfirmationInput::ShowError(err) => {
                self.error = Some(err);
                self.processing = false;
                self.password.clear(); // Security: clear password on error? Or keep it? kept usually for checking typos.
                // But if clear, user has to retype. "Keeprs" unlock dialog keeps it usually?
                // Let's keep it but user might have typed wrong.
            }
        }
    }
}
