use eframe::egui;
use eframe::egui::{ComboBox, Context, Ui};
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use crate::gui::app_loop::LocalState;
use crate::gui::common::{editable_text_input_copy, medium_data_item, valid_label};
use crate::gui::components::xpub_req;
use crate::gui::tabs::transact::{cold_wallet, hot_wallet, wallet_tab};
use crate::gui::tabs::transact::wallet_tab::WalletTab;
use crate::hardware::trezor;

pub fn wallet_screen_scrolled_deprecated(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState) {

    // local_state.wallet_state.updates.receiver.tr
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Cold Hardware").clicked() {
            ls.wallet_state.clear_data();
            ls.wallet_state.derivation_path = trezor::default_pubkey_path();
            ls.wallet_state.xpub_derivation_path = trezor::default_xpub_path();
            ls.wallet_state.public_key = None;
            ls.wallet_state.tab = WalletTab::Hardware;
            // device_list_status = None;
        }
        if ui.button("Hot Software").clicked() {
            ls.wallet_state.clear_data();
            hot_wallet::init_state(&mut ls.wallet_state);
            ls.wallet_state.tab = WalletTab::Software;
        }
    });

    ui.separator();

    match ls.wallet_state.tab {
        WalletTab::Hardware => {
            cold_wallet::hardware_connected(ui, &mut ls.wallet_state);
        }
        WalletTab::Software => {
            hot_wallet::hot_header(ls, ui, ctx);
        }
    }

    // wallet_tab::derivation_path_section(ui, ls);
    if ls.wallet_state.tab == WalletTab::Software {
        wallet_tab::hot_passphrase_section(ui, ls);
    }
    xpub_path_section(ui, ls, ctx);

    if let Some(pk) = ls.wallet_state.public_key.clone() {
        // proceed_from_pk(ui, ls, &pk, );
    }
    ui.spacing();
}


pub fn xpub_path_section(ui: &mut Ui, ls: &mut LocalState, ctx: &Context) {
    // wallet_tab::window_xpub(ui, ls, ctx);
    // xpub_csv_loader::window_xpub_loader(ui, ls, ctx);

    ui.horizontal(|ui| {
        ui.horizontal(|ui| {
            editable_text_input_copy(
                ui, "Xpub Derivation Path",
                &mut ls.wallet_state.xpub_derivation_path, 150.0,
            );
            if ls.wallet_state.xpub_derivation_path != ls.wallet_state.xpub_derivation_path_last_check {
                ls.wallet_state.xpub_derivation_path_last_check = ls.wallet_state.xpub_derivation_path.clone();
                ls.wallet_state.xpub_derivation_path_valid = ls.wallet_state.xpub_derivation_path.valid_derivation_path();
            }
            valid_label(ui, ls.wallet_state.xpub_derivation_path_valid);


            if ls.wallet_state.tab == WalletTab::Hardware {
                ui.spacing();

                xpub_req::request_xpub_hardware(ls, ui);
            }
        });
    });


    ui.horizontal(|ui| {
        ComboBox::from_label("Set Xpub Source")
            .selected_text(ls.wallet_state.selected_xpub_name.clone())
            .show_ui(ui, |ui| {
                for style in ls.local_stored_state.xpubs.iter().map(|x| x.name.clone()) {
                    ui.selectable_value(&mut ls.wallet_state.selected_xpub_name, style.clone(), style.to_string());
                }
                ui.selectable_value(&mut ls.wallet_state.selected_xpub_name,
                                    "Select Xpub".to_string(), "Select Xpub".to_string());
            });
        if ui.button("Load Xpub").clicked() {
            let xpub = ls.local_stored_state.xpubs.iter().find(|x|
                x.name == ls.wallet_state.selected_xpub_name);
            if let Some(named_xpub) = xpub {
                let xpub = named_xpub.clone().xpub.clone();
                ls.wallet_state.active_xpub = xpub.clone();
                let pk = XpubWrapper::new(xpub).public_at(0, 0).expect("xpub failure");
                ls.wallet_state.public_key = Some(pk.clone());
                let dp = format!("{}/0/0", named_xpub.derivation_path.clone());
                if ls.wallet_state.tab == WalletTab::Hardware {
                    ls.wallet_state.active_derivation_path = dp;
                } else {
                    ls.wallet_state.derivation_path = dp;
                    ls.wallet_state.xpub_derivation_path = named_xpub.derivation_path.clone();
                    ls.wallet_state.active_derivation_path = named_xpub.derivation_path.clone();
                }
            }
        }
    });
    medium_data_item(ui, "Active Derivation Path:", ls.wallet_state.active_derivation_path.clone());

    if ls.wallet_state.tab == WalletTab::Software {
        if ui.button("Save Xpub").clicked() {
            let xpub = ls.wallet_state.hot_mnemonic().xpub(ls.wallet_state.xpub_derivation_path.clone()).expect("xpub failure");
            ls.wallet_state.active_xpub = xpub.to_string();
            ls.wallet_state.show_save_xpub_window = true;
        }
    }

    if ui.button("Load Xpubs from CSV").clicked() {
        ls.wallet_state.show_xpub_loader_window = true;
    }
}
