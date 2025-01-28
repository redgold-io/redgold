use crate::common::{editable_text_input_copy, valid_label};
use crate::dependencies::gui_depends::GuiDepends;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

const DEFAULT_DP: &str = "m/44'/16180'/0'/0/0";
const COLD_DEFAULT_DP: &str = "m/44'/0'/50'/0/0";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DerivationPathInputState {
    pub derivation_path: String,
    pub last_derivation_path: String,
    pub valid: bool,
    pub changed: bool
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
            valid: true,
            changed: false,
        }
    }
    pub fn set_cold_default(&mut self) {
        self.derivation_path = COLD_DEFAULT_DP.to_string();
        self.last_derivation_path = COLD_DEFAULT_DP.to_string();
        self.valid = true;
    }
    pub fn validation<G>(&mut self, g: &G) -> bool where G: GuiDepends {
        let has_changed = self.derivation_path != self.last_derivation_path;
        if has_changed {
            self.last_derivation_path = self.derivation_path.clone();
            self.valid = g.validate_derivation_path(&self.derivation_path);
        }
        has_changed && self.valid
    }

    pub fn view<G>(&mut self, ui: &mut Ui, g: &G) -> bool where G: GuiDepends {
        ui.horizontal(|ui| {
            self.changed = editable_text_input_copy(ui, "Derivation Path", &mut self.derivation_path, 150.0);
            valid_label(ui, self.valid);
        });
        self.validation(g)
    }
}
