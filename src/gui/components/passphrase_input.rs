use eframe::egui;
use eframe::egui::{RichText, Ui};
use serde::{Deserialize, Serialize};
use redgold_keys::xpub_wrapper::ValidateDerivationPath;
use crate::gui::common::{editable_text_input_copy, valid_label};



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PassphraseInput {
    pub passphrase: String,
    pub last_passphrase: String,
    pub show_password: bool,
    pub err_msg: Option<String>,
}

impl Default for PassphraseInput {
    fn default() -> Self {
        Self {
            passphrase: "".to_string(),
            last_passphrase: "".to_string(),
            show_password: false,
            err_msg: None,
        }
    }
}

impl PassphraseInput {
    pub fn view(&mut self, ui: &mut Ui) -> bool {
        ui.horizontal(|ui| {
            ui.label("Passphrase:");
            egui::TextEdit::singleline(&mut self.passphrase)
                .desired_width(150f32)
                .password(!self.show_password).show(ui);
            ui.checkbox(&mut self.show_password, "Show");
            if let Some(err) = &self.err_msg {
                let rt = RichText::new(err).color(egui::Color32::RED);
                ui.label(rt);
            }
        });
        if self.last_passphrase != self.passphrase {
            self.last_passphrase = self.passphrase.clone();
            true
        } else {
            false
        }
    }
}
