use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use redgold_keys::xpub_wrapper::ValidateDerivationPath;
use crate::gui::common::{editable_text_input_copy, valid_label};


const DEFAULT_DP: &str = "m/44'/16180'/0'/0/0";
const COLD_DEFAULT_DP: &str = "m/44'/0'/50'/0/0";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DerivationPathInputState {
    pub derivation_path: String,
    pub last_derivation_path: String,
    pub valid: bool
}

impl Default for DerivationPathInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl DerivationPathInputState {
    pub fn new() -> Self {
        DerivationPathInputState {
            derivation_path: DEFAULT_DP.to_string(),
            last_derivation_path: DEFAULT_DP.to_string(),
            valid: true
        }
    }
    pub fn set_cold_default(&mut self) {
        self.derivation_path = COLD_DEFAULT_DP.to_string();
        self.last_derivation_path = COLD_DEFAULT_DP.to_string();
        self.valid = true;
    }
    pub fn validation(&mut self) -> bool {
        let has_changed = self.derivation_path != self.last_derivation_path;
        if has_changed {
            self.last_derivation_path = self.derivation_path.clone();
            self.valid = self.derivation_path.valid_derivation_path();
        }
        has_changed && self.valid
    }

    pub fn view(&mut self, ui: &mut Ui) -> bool {
        ui.horizontal(|ui| {
            editable_text_input_copy(ui, "Derivation Path", &mut self.derivation_path, 150.0);
            valid_label(ui, self.valid);
        });
        self.validation()
    }
}
