use std::str::FromStr;
use bitcoin::PrivateKey;
use eframe::egui;
use eframe::egui::{ComboBox, Ui};
use itertools::{Either, Itertools};
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::util::mnemonic_words::MnemonicWords;
use redgold_schema::local_stored_state::{StoredMnemonic, StoredPrivateKey};
use crate::gui::app_loop::LocalState;
use crate::gui::common::{data_item, editable_text_input_copy, valid_label};
use crate::gui::wallet_tab::{StateUpdate, WalletState};


fn save_key_window(
    ui: &mut Ui,
    ls: &mut LocalState,
    ctx: &egui::Context,
) {
    egui::Window::new("Add New Key")
        .open(&mut ls.wallet_state.add_new_key_window)
        .resizable(false)
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                editable_text_input_copy(ui, "Name", &mut ls.wallet_state.mnemonic_save_name, 150.0);
                editable_text_input_copy(ui, "Mnemonic / Key", &mut ls.wallet_state.mnemonic_save_data, 150.0);
                ui.checkbox(&mut ls.wallet_state.mnemonic_save_persist, "Persist to Disk");
                valid_label(ui, ls.wallet_state.is_mnemonic_or_kp.is_some());

                if ui.button("Save Internal").clicked() {
                    let name = ls.wallet_state.mnemonic_save_name.clone();
                    let data = ls.wallet_state.mnemonic_save_data.clone();
                    let mut is_mnemonic: Option<bool> = None;
                    if WordsPass::new(data.clone(), None).mnemonic().is_ok() {
                        is_mnemonic = Some(true);
                    } else if PrivateKey::from_str(data.as_str()).is_ok() {
                        is_mnemonic = Some(false);
                    }
                    ls.wallet_state.is_mnemonic_or_kp = is_mnemonic.clone();

                    if let Some(is_m) = is_mnemonic {
                        ls.updates.sender.send(StateUpdate {
                            update: Box::new(
                                move |lss: &mut LocalState| {
                                    if is_m {
                                        lss.upsert_mnemonic(StoredMnemonic {
                                            name: name.clone(),
                                            mnemonic: data.clone(),
                                            persist_disk: None,
                                        });
                                    } else {
                                        lss.upsert_private_key(StoredPrivateKey {
                                            name: name.clone(),
                                            key_hex: data.clone(),
                                        })
                                    }
                                })
                        }).unwrap();
                        ls.wallet_state.mnemonic_save_name = "".to_string();
                        ls.wallet_state.mnemonic_save_data = "".to_string();
                        LocalState::send_update(&ls.updates, |lss| {
                            lss.wallet_state.add_new_key_window = false;
                        })
                    }
                }
            });
        });
}



pub fn hot_header(ls: &mut LocalState, ui: &mut Ui, _ctx: &egui::Context) {

    save_key_window(ui, ls, _ctx);


    // Combo box to choose mnemonic
    ui.horizontal(|ui| {

        let string = ls.wallet_state.selected_key_name.clone();
        ComboBox::from_label("Key Source")
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
        if ui.button("Add New Key").clicked() {
            ls.wallet_state.add_new_key_window = true;
        }
    });

    let state = &mut ls.wallet_state;

    let check = state.mnemonic_checksum.clone();
    ui.label(format!("Hot Wallet Checksum: {check}"));

    if state.public_key.is_none() {
        state.update_hot_mnemonic_info();
    }

}


pub(crate) fn init_state(state: &mut WalletState) {
    // TODO: From constant or function for account zero.
    state.derivation_path = "m/44'/16180'/0'/0/0".to_string();
    state.xpub_derivation_path = "m/44'/16180'/0'".to_string();
    state.public_key = None;
}