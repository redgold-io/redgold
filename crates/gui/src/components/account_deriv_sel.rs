use crate::common::{editable_text_input_copy, medium_data_item, valid_label};
use crate::dependencies::gui_depends::GuiDepends;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

const DEFAULT_ACCOUNT_DP: &str = "m/44'/0'/50'";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AccountDerivationPathInputState {
    pub account_derivation_path: String,
    pub last_account_derivation_path: String,
    pub index: String,
    pub last_index: String,
    pub change: String,
    pub last_change: String,
    pub valid: bool
}

impl Default for AccountDerivationPathInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountDerivationPathInputState {
    pub fn new() -> Self {
        AccountDerivationPathInputState {
            account_derivation_path: DEFAULT_ACCOUNT_DP.to_string(),
            last_account_derivation_path: "".to_string(),
            index: "0".to_string(),
            last_index: "".to_string(),
            change: "0".to_string(),
            valid: true,
            last_change: "".to_string(),
        }
    }
    pub fn derivation_path(&self) -> String {
        format!("{}/{}/{}", self.account_derivation_path, self.change, self.index)
    }

    pub fn derivation_path_zero(&self) -> String {
        format!("{}/0/0", self.account_derivation_path)
    }

    pub fn derivation_path_valid_fallback(&self) -> String {
        if self.valid {
            self.derivation_path()
        } else {
            self.derivation_path_zero()
        }
    }


    pub fn validation(&mut self, g: impl GuiDepends + Sized) -> bool {
        let has_changed = self.account_derivation_path != self.last_account_derivation_path ||
            self.index != self.last_index || self.change != self.last_change;
        if has_changed {
            self.last_account_derivation_path = self.account_derivation_path.clone();
            self.last_index = self.index.clone();
            self.last_change = self.change.clone();
            self.valid = g.validate_derivation_path(self.derivation_path());
        }
        has_changed && self.valid
    }

    pub fn view(&mut self, ui: &mut Ui, g: impl GuiDepends) -> bool {
        ui.horizontal(|ui| {
            medium_data_item(ui, "Account", self.account_derivation_path.clone());
            editable_text_input_copy(ui, "Index", &mut self.index, 50.0);
            editable_text_input_copy(ui, "Change", &mut self.change, 50.0);
            valid_label(ui, self.valid, );
        });
        self.validation(g)
    }
}
