use eframe::egui::{ComboBox, Ui};
use itertools::Either;
use crate::gui::app_loop::LocalState;


pub fn key_source(ui: &mut Ui, ls: &mut LocalState) {

    // Combo box to choose mnemonic
    ui.horizontal(|ui| {

        ui.label("Key Source");
        let string = ls.wallet_state.selected_key_name.clone();
        ComboBox::from_label("")
            .selected_text(string.clone())
            .show_ui(ui, |ui| {
                for style in ls.local_stored_state.key_names() {
                    ui.selectable_value(&mut ls.wallet_state.selected_key_name, style.clone(), style.to_string());
                }
            });
        if ls.wallet_state.selected_key_name != ls.wallet_state.last_selected_key_name {
            ls.wallet_state.last_selected_key_name = string.clone();
            ls.wallet_state.active_hot_mnemonic = None;
            ls.wallet_state.active_hot_kp = None;
            let opt = ls.local_stored_state.by_key(&string).map(|key| {
                match key {
                    Either::Left(mnemonic) => {
                        ls.wallet_state.active_hot_mnemonic = Some(mnemonic.mnemonic.clone());
                    }
                    Either::Right(private_key) => {
                        ls.wallet_state.active_hot_kp = Some(private_key.key_hex);
                    }
                }
            });
            if opt.is_none() {
                ls.wallet_state.active_hot_mnemonic = Some(ls.wallet_state.hot_mnemonic_default.clone());
            }
            ls.wallet_state.update_hot_mnemonic_info();
        }
        // add_new_key_button(ls, ui);
    });
}

pub fn add_new_key_button(ls: &mut LocalState, ui: &mut Ui) {
    if ui.button("Add Mnemonic / Private Key").clicked() {
        ls.wallet_state.add_new_key_window = true;
    }
}