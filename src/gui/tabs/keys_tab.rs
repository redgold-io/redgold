use bdk::bitcoin::bech32::ToBase32;
use eframe::egui;
use eframe::egui::{ComboBox, Context, ScrollArea, TextEdit, Ui};
use log::info;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use tracing::Instrument;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::local_stored_state::{NamedXpub, XPubRequestType};
use crate::gui::app_loop::LocalState;
use crate::gui::common::{bounded_text_area_size, copy_to_clipboard, data_item, editable_text_input_copy, medium_data_item, medium_data_item_vertical};
use crate::gui::components::derivation_path_sel::DerivationPathInputState;
use crate::gui::components::key_info::{extract_gui_key, GuiKey, KeyInfo, update_keys_key_info, update_xpub_key_info};
use crate::gui::components::key_source_sel::{add_new_key_button, key_source};
use crate::gui::components::save_key_window;
use crate::gui::components::xpub_req::{RequestXpubState};
use crate::gui::tabs::keygen_subtab;
use crate::gui::tabs::keygen_subtab::keys_screen_scroll;
use crate::gui::wallet_tab::{derivation_path_section, hot_passphrase_section, window_xpub_loader};


#[derive(Debug, EnumIter, Clone, Serialize, Deserialize, EnumString)]
#[repr(i32)]
pub enum KeygenSubTab {
    Manage,
    Generate,
}

#[derive(Debug, EnumIter, Clone, Serialize, Deserialize, EnumString)]
#[repr(i32)]
pub enum KeygenSubSubTab {
    Keys,
    XPubs
}

pub struct KeyTabState {
    pub keygen_subtab: KeygenSubTab,
    pub subsubtab: KeygenSubSubTab,
    pub show_private_key_window: bool,
    pub show_xpub: bool,
    pub dp_key_viewer: DerivationPathInputState,
    pub dp_xpub_viewer: DerivationPathInputState,
    pub request_xpub: RequestXpubState,
    pub keys_key_info: KeyInfo,
    pub xpub_key_info: KeyInfo,
    pub save_xpub_account_name: String
}

impl Default for KeyTabState {
    fn default() -> Self {
        KeyTabState {
            keygen_subtab: KeygenSubTab::Manage,
            subsubtab: KeygenSubSubTab::XPubs,
            show_private_key_window: false,
            show_xpub: false,
            dp_key_viewer: Default::default(),
            dp_xpub_viewer: Default::default(),
            request_xpub: Default::default(),
            keys_key_info: Default::default(),
            xpub_key_info: Default::default(),
            save_xpub_account_name: "".to_string(),
        }
    }
}


pub fn manage_view(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState, first_init: bool) {
    ui.add_space(10.0);
    ui.heading("Add");
    ui.separator();

    // Add New Stuff buttons
    ui.horizontal(|ui| {
        add_new_key_button(ls, ui);
        add_xpub_csv_button(ls, ui, ctx);
        ls.keytab_state.request_xpub.view(ui, ctx, &ls.updates, ls.wallet_state.device_list_status.device_output.clone());
    });

    save_key_window::save_key_window(ui, ls, ctx);

    if ls.wallet_state.public_key.is_none() {
        ls.wallet_state.update_hot_mnemonic_or_key_info();
    }

    keygen_subtab::mnemonic_window(ctx, ls);
    show_private_key_window(ctx, ls);

    // ui.label("".to_string());
    ui.add_space(10.0);
    ui.heading("View");
    ui.separator();
    ui.spacing();
    ui.spacing();

    ui.horizontal(|ui| {

    for subsubtab in KeygenSubSubTab::iter() {
        if ui.button(format!("View {:?}", subsubtab)).clicked() {
            ls.keytab_state.subsubtab = subsubtab;
        }
    }
    });
    ui.separator();
    ui.spacing();
    ui.spacing();

    match ls.keytab_state.subsubtab {
        KeygenSubSubTab::Keys => {
            ui.label("Internal Stored Keys");
            ui.spacing();
            internal_stored_keys(ui, ls, first_init);

        }
        KeygenSubSubTab::XPubs => {
            ui.label("Internal Stored XPubs");
            ui.spacing();
            internal_stored_xpubs(ls, ui, ctx, first_init);
        }
    }
    // TODO: Sub-subtabs for these two



}

fn internal_stored_keys(ui: &mut Ui, ls: &mut LocalState, first_init: bool) {
    let mut need_keys_update = false;
    ui.horizontal(|ui| {
        let has_changed_key = key_source(ui, ls);
        need_keys_update = has_changed_key;
        medium_data_item(ui,"Checksum: ", &ls.wallet_state.mnemonic_or_key_checksum);
        if ui.button("Show Key").clicked() {
            if ls.wallet_state.active_hot_private_key_hex.is_none() {
                ls.keygen_state.mnemonic_window_state.set_words(
                    ls.wallet_state.hot_mnemonic().words,
                    ls.wallet_state.selected_key_name.clone(),
                );
            } else {
                ls.keytab_state.show_private_key_window = true;
            }
        }
    });

    let dp_has_changed_key = ls.keytab_state.dp_key_viewer.view(ui);
    // TODO: Hot passphrase should ONLY apply to mnemonics as it doesn't work for private keys
    if ls.wallet_state.active_hot_private_key_hex.is_none() {
        let update_clicked = hot_passphrase_section(ui, ls);
        if update_clicked {
            need_keys_update = true;
        }
    }
    if need_keys_update || first_init || dp_has_changed_key {
        info!("Updating keys key info {} {}", need_keys_update, first_init);
        update_keys_key_info(ls);
    }
    // Show seed checksum (if mnemonic)
    if ls.wallet_state.active_hot_private_key_hex.is_none() {
        medium_data_item(ui,"Seed Checksum: ", ls.wallet_state.seed_checksum.as_ref().unwrap_or(&"".to_string()));
    }

    ls.keytab_state.keys_key_info.view(ui);

    if ls.wallet_state.active_hot_private_key_hex.is_none() {
        ui.horizontal(|ui| {
            editable_text_input_copy(ui, "Save XPub Account Name:", &mut ls.keytab_state.save_xpub_account_name, 150.0);
            if ui.button("Save").clicked() {
                let derivation_path = ls.keytab_state.dp_key_viewer.derivation_path.as_account_path();
                if let Some(derivation_account_path) = derivation_path {
                    let m = ls.wallet_state.hot_mnemonic();
                    if let Ok(xpub) = m.xpub_str(&derivation_account_path) {
                        let dp2 = ls.keytab_state.dp_key_viewer.derivation_path.clone();
                        let check = m.checksum().unwrap_or("".to_string());
                        let words_public = m.public_at(&dp2).expect("Public at failed").hex_or();
                        let xpub_w = XpubWrapper::new(xpub.clone());
                        let xpub_public = xpub_w.public_at_dp(&dp2).expect("Public at DP failed").hex_or();
                        let equal = words_public == xpub_public;
                        info!("Adding xpub to local state from keys tab with words pass \
                        checksum: {check} equal {equal} words public: {words_public} xpub public: {xpub_public}");
                        let ho = Some(ls.wallet_state.hot_offset.clone()).filter(|x| !x.is_empty());
                        ls.add_named_xpubs(true,  vec![NamedXpub {
                            name: ls.keytab_state.save_xpub_account_name.clone(),
                            xpub,
                            derivation_path: derivation_account_path,
                            hot_offset: ho,
                            key_name_source: Some(ls.wallet_state.selected_key_name.clone()),
                            device_id: None,
                            key_reference_source: None,
                            key_nickname_source: None,
                            request_type: Some(XPubRequestType::Hot),
                        }]).ok();
                    }
                }
            }
        });
    }
}


pub fn keys_tab(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState, first_init: bool) {
    ui.heading("Keys");
    ui.separator();

    ui.horizontal(|ui| {
    KeygenSubTab::iter().for_each(|subtab| {
        if ui.button(format!("{:?}", subtab)).clicked() {
            local_state.keytab_state.keygen_subtab = subtab;
        }
    })
    });
    match local_state.keytab_state.keygen_subtab {
        KeygenSubTab::Manage => {
            manage_view(ui, ctx, local_state, first_init);
        }
        KeygenSubTab::Generate => {
            keygen_subtab::keys_screen(ui, ctx, local_state);
        }
    }
}


pub(crate) fn show_private_key_window(
    ctx: &Context, ls: &mut LocalState
) {

    egui::Window::new("Private Key")
        .open(&mut ls.keytab_state.show_private_key_window)
        .resizable(false)
        .collapsible(false)
        .min_width(300.0)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(ls.wallet_state.selected_key_name.clone());
                ui.horizontal(|ui| {
                    let mut kp = ls.wallet_state.active_hot_private_key_hex.clone().unwrap_or("".to_string());
                    TextEdit::multiline(&mut kp)
                        .desired_width(400f32).show(ui);
                    copy_to_clipboard(ui, kp.clone());
                });
            });
        });
}


pub(crate) fn show_xpub_window(
    ctx: &Context, ls: &mut LocalState, xpub: NamedXpub
) {

    egui::Window::new("XPub")
        .open(&mut ls.keytab_state.show_xpub)
        .resizable(false)
        .collapsible(false)
        .min_width(300.0)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                medium_data_item(ui, "Name", xpub.name.clone());
                medium_data_item(ui, "Derivation Path", xpub.derivation_path.clone());
                if let Some(ho) = xpub.hot_offset {
                    medium_data_item(ui, "Hot Offset", ho);
                }
                let mut string = xpub.xpub.clone();
                bounded_text_area_size(ui, &mut string, 300.0, 4);
                copy_to_clipboard(ui, xpub.xpub.clone());

            });
        });
}


pub fn internal_stored_xpubs(ls: &mut LocalState, ui: &mut Ui, ctx: &egui::Context, first_init: bool) {
    let xpub =
        ls.local_stored_state.xpubs.iter().find(|x| x.name == ls.wallet_state.selected_xpub_name)
            .cloned();

    let xpub2 = xpub.clone();

    let mut update = false;


    ui.horizontal(|ui| {
        ui.label("Select XPub");
        ComboBox::from_label("".to_string())
            .width(125.0)
            .selected_text(ls.wallet_state.selected_xpub_name.clone())
            .show_ui(ui, |ui| {
                for style in ls.local_stored_state.xpubs.iter().map(|x| x.name.clone()) {
                    ui.selectable_value(&mut ls.wallet_state.selected_xpub_name, style.clone(), style.to_string());
                }
                ui.selectable_value(&mut ls.wallet_state.selected_xpub_name,
                                    "Select Xpub".to_string(), "Select Xpub".to_string());
            });
        if let Some(xp) = xpub {
            let i = xp.xpub.len();
            if let Some(slice) = xp.xpub.get((i -8)..i) {
                medium_data_item(ui, "Last 8:", slice);
            }
            if ui.button("Show XPub").clicked() {
                ls.keytab_state.show_xpub = true;
            }
        }

    });


    if let Some(xp) = xpub2.as_ref() {
        show_xpub_window(ctx, ls, xp.clone());

        ui.horizontal(|ui| {
        if let Some(ap) = xp.derivation_path.as_account_path() {
            medium_data_item(ui, "Account:", ap);
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
        if ls.keytab_state.dp_xpub_viewer.view(ui) {
            update = true;
        }
        if ui.button("Update").clicked() {
            update = true;
        }
    });

    if ls.wallet_state.last_selected_xpub_name != ls.wallet_state.selected_xpub_name {
        ls.wallet_state.last_selected_xpub_name = ls.wallet_state.selected_xpub_name.clone();
        update = true;
    }

    if update || first_init {
        update_xpub_key_info(ls);
    }



    ls.keytab_state.xpub_key_info.view(ui);


}
pub fn add_xpub_csv_button(ls: &mut LocalState, ui: &mut Ui, ctx: &egui::Context) {
    window_xpub_loader(ui, ls, ctx);
    if ui.button("Add XPubs From CSV").clicked() {
        ls.wallet_state.show_xpub_loader_window = true;
    }
}



