use eframe::egui;
use eframe::egui::{ComboBox, Context, TextEdit, Ui};
use itertools::{Either, Itertools};
use redgold_keys::util::mnemonic_support::WordsPass;
use crate::gui::app_loop::LocalState;
use redgold_gui::common::{copy_to_clipboard, editable_text_input_copy, medium_data_item};
use crate::gui::tables::text_table;


pub fn key_source(ui: &mut Ui, ls: &mut LocalState) -> bool {

    let mut has_changed = false;
    // Combo box to choose mnemonic
    ui.horizontal(|ui| {

        ui.label("Key Source");
        let string = ls.wallet.selected_key_name.clone();
        ComboBox::from_label("")
            .selected_text(string.clone())
            .show_ui(ui, |ui| {
                for style in ls.local_stored_state.key_names() {
                    ui.selectable_value(&mut ls.wallet.selected_key_name, style.clone(), style.to_string());
                }
            });
        if ls.wallet.selected_key_name != ls.wallet.last_selected_key_name {
            has_changed = true;
            ls.wallet.last_selected_key_name = string.clone();
            ls.wallet.active_hot_mnemonic = None;
            ls.wallet.active_hot_private_key_hex = None;
            ls.wallet.mnemonic_or_key_checksum = "".to_string();
            // TODO: Really this could be refactored in an Enum that has multiple direct value
            // structs, but for now we'll just use the Either type
            // also the state storage should account for that as well.
            let opt = ls.local_stored_state.by_key(&string).map(|key| {
                match key {
                    Either::Left(mnemonic) => {
                        ls.wallet.active_hot_mnemonic = Some(mnemonic.mnemonic.clone());
                    }
                    Either::Right(private_key) => {
                        ls.wallet.active_hot_private_key_hex = Some(private_key.key_hex);
                    }
                }
            });
            if opt.is_none() {
                ls.wallet.active_hot_mnemonic = Some(ls.wallet.hot_mnemonic_default.clone());
            }
            ls.wallet.update_hot_mnemonic_or_key_info();
        }
        // add_new_key_button(ls, ui);
    });
    has_changed
}

pub fn add_new_key_button(ls: &mut LocalState, ui: &mut Ui) {
    if ui.button("Add Hot Mnemonic / Private Key").clicked() {
        ls.wallet.add_new_key_window = true;
    }
}
