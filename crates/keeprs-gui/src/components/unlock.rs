//! Password unlock dialog component.

use gtk4::prelude::*;
use relm4::prelude::*;

/// Messages for the unlock dialog.
#[derive(Debug)]
pub enum UnlockInput {
    /// Password text changed.
    PasswordChanged(String),
    /// Attempt to unlock.
    Unlock,
    /// Show error message.
    ShowError(String),
}

/// Output messages from the unlock dialog.
#[derive(Debug)]
pub enum UnlockOutput {
    /// User submitted password.
    Unlocked(String),
}

/// Unlock dialog model.
pub struct UnlockDialog {
    password: String,
    error: Option<String>,
    unlocking: bool,
}

#[relm4::component(pub)]
impl Component for UnlockDialog {
    type Init = ();
    type Input = UnlockInput;
    type Output = UnlockOutput;
    type CommandOutput = ();

    view! {
        gtk4::Box {
            set_orientation: gtk4::Orientation::Vertical,
            set_spacing: 20,
            set_margin_all: 40,
            set_halign: gtk4::Align::Center,
            set_valign: gtk4::Align::Center,
            set_width_request: 400,

            gtk4::Label {
                set_markup: "<span size='xx-large' weight='bold'>🔐 Keeprs</span>",
                set_margin_bottom: 10,
            },

            gtk4::Label {
                set_text: "Enter your master password to unlock the database",
                set_wrap: true,
                add_css_class: "dim-label",
            },

            gtk4::PasswordEntry {
                set_placeholder_text: Some("Master Password"),
                set_show_peek_icon: true,
                set_hexpand: true,

                connect_changed[sender] => move |entry| {
                    sender.input(UnlockInput::PasswordChanged(entry.text().to_string()));
                },

                connect_activate[sender] => move |_| {
                    sender.input(UnlockInput::Unlock);
                },
            },

            #[name = "error_label"]
            gtk4::Label {
                #[watch]
                set_visible: model.error.is_some(),
                #[watch]
                set_text: model.error.as_deref().unwrap_or(""),
                add_css_class: "error",
            },

            gtk4::Button {
                set_label: "Unlock",
                add_css_class: "suggested-action",
                add_css_class: "pill",
                set_height_request: 40,

                #[watch]
                set_sensitive: !model.unlocking && !model.password.is_empty(),

                connect_clicked => UnlockInput::Unlock,
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = UnlockDialog {
            password: String::new(),
            error: None,
            unlocking: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            UnlockInput::PasswordChanged(password) => {
                self.password = password;
                self.error = None;
            }
            UnlockInput::Unlock => {
                if !self.password.is_empty() {
                    self.unlocking = true;
                    let _ = sender.output(UnlockOutput::Unlocked(self.password.clone()));
                }
            }
            UnlockInput::ShowError(error) => {
                self.error = Some(error);
                self.unlocking = false;
            }
        }
    }
}
