
use eframe::egui;
use eframe::egui::{ComboBox, Context, ScrollArea, TextEdit, Ui};
use log::info;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::conf::local_stored_state::{AccountKeySource, XPubLikeRequestType};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::PublicKey;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use crate::common::{copy_to_clipboard, editable_text_input_copy, medium_data_item};
use crate::components::account_deriv_sel::AccountDerivationPathInputState;
use crate::components::derivation_path_sel::DerivationPathInputState;
use crate::dependencies::gui_depends::GuiDepends;
use crate::state::local_state::{LocalState, LocalStateAddons};
use crate::tab::keys::key_info::{update_keys_key_info, update_xpub_key_info, KeyInfo};
use crate::tab::keys::keygen::KeygenSubTab;
use crate::tab::keys::{keygen_subtab, save_key_window};
use crate::tab::keys::key_source_sel::key_source;
use crate::tab::keys::show_xpub_window::show_xpub_window;
use crate::tab::keys::xpub_csv_loader::window_xpub_loader;
use crate::tab::keys::xpub_req::RequestXpubState;


pub fn add_new_key_button<E>(ls: &mut LocalState<E>, ui: &mut Ui) where E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    if ui.button("Add Hot Mnemonic / Private Key").clicked() {
        ls.wallet.add_new_key_window = true;
    }
}

pub fn manage_view<G, E>(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState<E>, first_init: bool, g: &G)
where G: GuiDepends + Clone + Send + 'static + Sync,
      E: ExternalNetworkResources + Send + Sync + 'static + Clone {
    ui.add_space(10.0);

    // Add New Stuff buttons
    ui.horizontal(|ui| {
        ui.heading("Add");
        add_new_key_button(ls, ui);
        add_xpub_csv_button(ls, ui, ctx);
        let res = ls.keytab_state.request_xpub
            .view(ui, ctx, ls.local_messages.sender.clone(), ls.wallet.device_list_status.device_output.clone(), g);
        if !res.is_empty() {
            ls.add_named_xpubs(true, res, false).log_error().ok();
            ls.persist_local_state_store();
        }

    });

    save_key_window::save_key_window(ui, ls, ctx, g);

    if ls.wallet.public_key.is_none() {
        ls.wallet.update_hot_mnemonic_or_key_info(g);
    }

    keygen_subtab::mnemonic_window(ctx, ls, g);
    show_private_key_window(ctx, ls);
    ui.separator();
    ui.spacing();
    internal_stored_keys(ui, ls, first_init, g);
    ui.separator();
    internal_stored_xpubs(ls, ui, ctx, first_init, g, Some("Internal Stored XPubs".to_string()), None, false);

}


pub fn hot_passphrase_section<E, G>(ui: &mut Ui, ls: &mut LocalState<E>, g: &G) -> bool
where E: ExternalNetworkResources + Clone + Send + 'static + Sync, G: GuiDepends + Clone + Send + 'static + Sync {

    let mut update_clicked = false;

    if &ls.wallet.hot_passphrase_last != &ls.wallet.hot_passphrase.clone() {
        ls.wallet.hot_passphrase_last = ls.wallet.hot_passphrase.clone();
        update_clicked = true;
    }
    if &ls.wallet.hot_offset_last != &ls.wallet.hot_offset.clone() {
        ls.wallet.hot_offset_last = ls.wallet.hot_offset.clone();
        update_clicked = true;
    }

    ui.horizontal(|ui| {
        ui.label("Passphrase:");
        egui::TextEdit::singleline(&mut ls.wallet.hot_passphrase)
            .desired_width(150f32)
            .password(true).show(ui);
        ui.label("Offset:");
        egui::TextEdit::singleline(&mut ls.wallet.hot_offset)
            .desired_width(150f32)
            .show(ui);
        if ui.button("Update").clicked() {
            update_clicked = true;
        };
    });
    if update_clicked {
        ls.wallet.update_hot_mnemonic_or_key_info(g);
    };
    update_clicked
}


fn internal_stored_keys<G, E>(ui: &mut Ui, ls: &mut LocalState<E>, first_init: bool, g: &G)
where G: GuiDepends + Clone + Send + 'static + Sync,
      E : ExternalNetworkResources + Send + Sync + 'static + Clone {
    let mut need_keys_update = false;
    ui.horizontal(|ui| {
        ui.heading("Internal Stored Keys");
        let has_changed_key = key_source(ui, ls, g);
        need_keys_update = has_changed_key;
        medium_data_item(ui,"Checksum: ", &ls.wallet.mnemonic_or_key_checksum);
        if ui.button("Show Key").clicked() {
            if ls.wallet.active_hot_private_key_hex.is_none() {
                ls.keygen_state.mnemonic_window_state.set_words(
                    ls.wallet.hot_mnemonic(g).words,
                    ls.wallet.selected_key_name.clone(),
                    g
                );
            } else {
                ls.keytab_state.show_private_key_window = true;
            }
        }
    });

    let dp_has_changed_key = ls.keytab_state.key_derivation_path_input.view(ui, g);
    // TODO: Hot passphrase should ONLY apply to mnemonics as it doesn't work for private keys
    if ls.wallet.active_hot_private_key_hex.is_none() {
        let update_clicked = hot_passphrase_section(ui, ls, g);
        if update_clicked {
            need_keys_update = true;
        }
    }
    if need_keys_update || first_init || dp_has_changed_key {
        // info!("Updating keys key info {} {}", need_keys_update, first_init);
        update_keys_key_info(ls, g);
    }
    // Show seed checksum (if mnemonic)
    if ls.wallet.active_hot_private_key_hex.is_none() {
        medium_data_item(ui,"Seed Checksum: ", ls.wallet.seed_checksum.as_ref().unwrap_or(&"".to_string()));
    }

    ls.keytab_state.keys_key_info.view(ui, None, ls.node_config.network.clone(), g);

    if ls.wallet.active_hot_private_key_hex.is_none() {
        ui.horizontal(|ui| {
            editable_text_input_copy(ui, "Save XPub Account Name:", &mut ls.keytab_state.save_xpub_account_name, 150.0);
            if ui.button("Save").clicked() {
                let derivation_path = G::as_account_path(ls.keytab_state.key_derivation_path_input.derivation_path.clone());
                if let Some(derivation_account_path) = derivation_path {
                    let m = ls.wallet.hot_mnemonic(g);
                    if let Ok(xpub) = G::get_xpub_string_path(m.clone(), &derivation_account_path) {
                        let dp2 = ls.keytab_state.key_derivation_path_input.derivation_path.clone();
                        let check = G::checksum_words(m.clone()).unwrap_or("".to_string());
                        // let check = m.checksum().unwrap_or("".to_string());
                        let pk = G::public_at(m.clone(), &dp2).expect("Public at failed");
                        // let pk = m.public_at(&dp2).expect("Public at failed");
                        let all = g.to_all_address(&pk);
                        let words_public = pk.hex();
                        let xpub_public = g.xpub_public(xpub.clone(), dp2.clone()).log_error().ok().unwrap().hex();
                        // let xpub_w = XpubWrapper::new(xpub.clone());
                        // let xpub_public = xpub_w.public_at_dp(&dp2).expect("Public at DP failed").hex();
                        let equal = words_public == xpub_public;
                        info!("Adding xpub to local state from keys tab with words pass \
                        checksum: {check} equal {equal} words public: {words_public} xpub public: {xpub_public}");
                        let ho = Some(ls.wallet.hot_offset.clone()).filter(|x| !x.is_empty());
                        ls.add_named_xpubs(true, vec![AccountKeySource {
                            name: ls.keytab_state.save_xpub_account_name.clone(),
                            xpub,
                            derivation_path: derivation_account_path,
                            hot_offset: ho,
                            key_name_source: Some(ls.wallet.selected_key_name.clone()),
                            device_id: None,
                            key_reference_source: None,
                            key_nickname_source: None,
                            request_type: Some(XPubLikeRequestType::Hot),
                            skip_persist: None,
                            preferred_address: None,
                            all_address: Some(all),
                            public_key: Some(pk),
                        }], false).ok();
                    }
                }
            }
        });
    }
}


pub fn keys_tab<G, E>(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState<E>, first_init: bool, g: &G)
where G: GuiDepends + Clone + Send + 'static + Sync,
      E : ExternalNetworkResources + Send + Sync + 'static + Clone{

    ui.horizontal(|ui| {
        ui.heading("Keys");
        KeygenSubTab::iter().for_each(|subtab| {
            if ui.button(format!("{:?}", subtab)).clicked() {
                local_state.keytab_state.keygen_subtab = subtab;
            }
        })
    });
    ui.separator();

    match local_state.keytab_state.keygen_subtab {
        KeygenSubTab::Manage => {
            manage_view(ui, ctx, local_state, first_init, g);
        }
        KeygenSubTab::Generate => {
            keygen_subtab::keys_screen(ui, ctx, local_state, g);
        }
    }
}


pub(crate) fn show_private_key_window<E>(
    ctx: &Context, ls: &mut LocalState<E>
) where E: ExternalNetworkResources + Clone + Send + Sync + 'static {

    egui::Window::new("Private Key")
        .open(&mut ls.keytab_state.show_private_key_window)
        .resizable(false)
        .collapsible(false)
        .min_width(300.0)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(ls.wallet.selected_key_name.clone());
                ui.horizontal(|ui| {
                    let mut kp = ls.wallet.active_hot_private_key_hex.clone().unwrap_or("".to_string());
                    TextEdit::multiline(&mut kp)
                        .desired_width(400f32).show(ui);
                    copy_to_clipboard(ui, kp.clone());
                });
            });
        });
}



pub fn internal_stored_xpubs<G, E>(
    ls: &mut LocalState<E>,
    ui: &mut Ui,
    ctx: &egui::Context,
    first_init: bool, g: &G,
    heading_override: Option<String>,
    option: Option<PublicKey>,
    show_balance_checkbox: bool,
) -> (bool, Option<AccountKeySource>) where G: GuiDepends + Clone + Send + 'static, E: ExternalNetworkResources + Send + Sync + 'static + Clone {


    let mut xpub : Option<AccountKeySource> = None;

    let mut update = false;


    ui.horizontal(|ui| {
        ui.heading(heading_override.unwrap_or("Transact".to_string()));
        ui.label("Select Account");
        ComboBox::from_label("".to_string())
            .width(125.0)
            .selected_text(ls.wallet.selected_xpub_name.clone())
            .show_ui(ui, |ui| {
                let option = ls.local_stored_state.keys.clone().unwrap_or(vec![]);
                for style in option.iter().map(|x| x.name.clone()) {
                    ui.selectable_value(&mut ls.wallet.selected_xpub_name, style.clone(), style.to_string());
                }
                ui.selectable_value(&mut ls.wallet.selected_xpub_name,
                                    "Select Account".to_string(), "Select Account".to_string());
            });
        xpub = ls.local_stored_state.keys.as_ref().and_then(|x| x.iter().find(|x| x.name == ls.wallet.selected_xpub_name)
            .cloned());
        if let Some(xp) = &xpub {
            let i = xp.xpub.len();
            if let Some(slice) = xp.xpub.get((i -8)..i) {
                medium_data_item(ui, "Last 8:", slice);
            }
            if ui.button("Show Source").clicked() {
                ls.keytab_state.show_xpub = true;
            }
            ui.checkbox(&mut ls.wallet.view_additional_xpub_details, "Show Key Details");
            if show_balance_checkbox {
                ui.checkbox(&mut ls.wallet.show_xpub_balance_info, "Show Balance Info");
            }
        }

    });
    ui.separator();

    if ls.wallet.view_additional_xpub_details {
        if let Some(xp) = xpub.as_ref() {
            show_xpub_window(ctx, ls, xp.clone(), g);

            ui.horizontal(|ui| {
                if let Some(ap) = G::as_account_path(xp.derivation_path.clone()) {
                    ls.keytab_state.derivation_path_xpub_input_account.account_derivation_path = ap;
                    // medium_data_item(ui, "Account:", ap);
                }
                if let Some(rt) = &xp.request_type {
                    medium_data_item(ui, "Type:", format!("{:?}", rt));
                }
                if let Some(ks) = &xp.key_name_source {
                    medium_data_item(ui, "Key Name:", ks);
                }
                if let Some(ho) = &xp.hot_offset {
                    medium_data_item(ui, "Hot Offset:", ho);
                }
            });
        }

        ui.horizontal(|ui| {
            if ls.keytab_state.derivation_path_xpub_input_account.view(ui, g.clone()) {
                update = true;
            }
            if ui.button("Update").clicked() {
                update = true;
            }
        });

        if ls.wallet.last_selected_xpub_name != ls.wallet.selected_xpub_name {
            ls.wallet.last_selected_xpub_name = ls.wallet.selected_xpub_name.clone();
            // info!("Selected xpub changed to {} returning {}", ls.wallet.selected_xpub_name.clone(), xpub.json_or());
            update = true;
        }

        if update || first_init {
            update_xpub_key_info(ls, g);
        }

        ls.keytab_state.xpub_key_info.view(ui, option, ls.node_config.network.clone(), g);
    }

    (update, xpub)
}
pub fn add_xpub_csv_button<E>(ls: &mut LocalState<E>, ui: &mut Ui, ctx: &egui::Context) where E: ExternalNetworkResources + Send + Sync + 'static + Clone {
    window_xpub_loader(ui, ls, ctx);
    if ui.button("Add XPubs From CSV").clicked() {
        ls.wallet.show_xpub_loader_window = true;
    }
}



