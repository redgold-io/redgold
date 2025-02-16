use eframe::egui;
use eframe::egui::Ui;
use redgold_common::external_resources::ExternalNetworkResources;
use crate::common::{editable_text_input_copy, valid_label};
use redgold_schema::conf::local_stored_state::{StoredMnemonic, StoredPrivateKey};
use redgold_schema::keys::words_pass::WordsPass;
use std::str::FromStr;
use crate::dependencies::gui_depends::GuiDepends;
use crate::state::local_state::{LocalState, LocalStateAddons};

pub fn save_key_window<E, G>(
    _ui: &mut Ui,
    ls: &mut LocalState<E>,
    ctx: &egui::Context,
    g: &G
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone, G: GuiDepends {
    let mut add_new_key_window = ls.wallet.add_new_key_window;
    egui::Window::new("Add Mnemonic/Private Key")
        .open(&mut add_new_key_window)
        .resizable(false)
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                editable_text_input_copy(ui, "Name", &mut ls.wallet.mnemonic_save_name, 150.0);
                editable_text_input_copy(ui, "Mnemonic / Key", &mut ls.wallet.mnemonic_save_data, 150.0);
                ui.checkbox(&mut ls.wallet.mnemonic_save_persist, "Persist to Disk");
                valid_label(ui, ls.wallet.is_mnemonic_or_kp.is_some(), );

                if ui.button("Save Internal").clicked() {
                    let name = ls.wallet.mnemonic_save_name.clone();
                    let data = ls.wallet.mnemonic_save_data.clone();
                    let mut is_mnemonic: Option<bool> = None;
                    let wp = WordsPass::new(data.clone(), None);
                    let ok = G::validate_mnemonic(wp.clone());

                    let as_hex = data.as_str();
                    let pk_parsed = g.private_hex_to_public_key(as_hex);
                    if ok.is_ok() {
                        is_mnemonic = Some(true);
                    } else if pk_parsed.is_ok() {
                        is_mnemonic = Some(false);
                    }
                    ls.wallet.is_mnemonic_or_kp = is_mnemonic.clone();

                    if let Some(is_m) = is_mnemonic {
                        // ls.updates.sender.send(StateUpdate {
                        //     update: Box::new(
                        //         move |lss: &mut LocalState| {
                                    if is_m {
                                        ls.upsert_mnemonic(StoredMnemonic {
                                            name: name.clone(),
                                            mnemonic: data.clone(),
                                            passphrase: None,
                                            persist_disk: None,
                                        });
                                    } else {
                                        ls.upsert_private_key(StoredPrivateKey {
                                            name: name.clone(),
                                            key_hex: data.clone(),
                                        })
                                    }
                                // })
                        // }).unwrap();
                        ls.wallet.mnemonic_save_name = "".to_string();
                        ls.wallet.mnemonic_save_data = "".to_string();
                        // send_update(&ls.updates, |lss| {
                            ls.wallet.add_new_key_window = false;
                        // })
                    }
                }
            });
        });

    ls.wallet.add_new_key_window = add_new_key_window;

}
