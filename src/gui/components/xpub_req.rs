use std::str::FromStr;
use std::sync::Mutex;
use bdk::bitcoin::PrivateKey;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, RichText, Ui};
use itertools::Either;
use rocket::serde::Serialize;
use serde::Deserialize;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_keys::xpub_wrapper::XpubWrapper;
use tracing::error;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::{EasyJson, error_info, RgResult};
use redgold_schema::local_stored_state::{NamedXpub, StoredMnemonic, StoredPrivateKey, XPubRequestType};
use crate::core::internal_message::Channel;
use crate::gui::app_loop::LocalState;
use crate::gui::common::{bounded_text_area_size, copy_to_clipboard, editable_text_input_copy, valid_label};
use crate::gui::components::derivation_path_sel::DerivationPathInputState;
use crate::gui::components::key_source_sel::key_source;
use crate::gui::tabs::transact::cold_wallet::hardware_connected;
use crate::gui::tabs::transact::wallet_tab;
use crate::gui::tabs::transact::wallet_tab::{StateUpdate};
use crate::hardware::trezor;
use crate::observability::logging::Loggable;

pub fn request_xpub_hardware(ls: &mut LocalState, ui: &mut Ui) {
    if ui.button("Request Xpub").clicked() {
        ls.wallet_state.public_key = None;
        ls.wallet_state.public_key_msg = Some("Awaiting input on device...".to_string());
        // This blocks the entire UI... ah jeez
        match trezor::get_public_node(ls.wallet_state.xpub_derivation_path.clone()).map(|x| x.xpub) {
            Ok(xpub) => {
                ls.wallet_state.show_save_xpub_window = true;
                ls.wallet_state.active_xpub = xpub.clone();
                let pk = XpubWrapper::new(xpub).public_at(0, 0).expect("xpub failure");
                ls.wallet_state.public_key = Some(pk.clone());
                ls.wallet_state.public_key_msg = Some("Got public key".to_string());
                wallet_tab::get_address_info(
                    &ls.node_config,
                    pk,
                    ls.wallet_state.updates.sender.clone(),
                );
            }
            Err(e) => {
                ls.wallet_state.public_key_msg = Some("Error getting public key".to_string());
                error!("Error getting public key: {}", e.json_or());
            }
        }
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RequestXpubState {
    save_name: String,
    result: Option<RgResult<String>>,
    xpub_type: XPubRequestType,
    message: String,
    derivation_path: DerivationPathInputState,
    show_window: bool
}

impl Default for RequestXpubState {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestXpubState {
    pub fn new() -> Self {
        Self {
            save_name: "".to_string(),
            result: None,
            xpub_type: XPubRequestType::Cold,
            message: "".to_string(),
            derivation_path: Default::default(),
            show_window: false,
        }
    }

    pub fn clear(&mut self) {
        self.save_name = "".to_string();
        self.result = None;
        self.message = "".to_string();
        // self.derivation_path = Default::default();
    }

    pub fn button(&mut self, ui: &mut Ui) {
        if ui.button("Request XPub").clicked() {
            self.clear();
            self.show_window = true;
        }
    }

    pub fn view(&mut self, ui: &mut Ui, ctx: &egui::Context, updates: &Channel<StateUpdate>, device_list: Option<String>) {
        self.button(ui);

        egui::Window::new("Request XPub")
            .open(&mut self.show_window)
            .resizable(false)
            .collapsible(false)
            .min_width(500.0)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let request_type = self.xpub_type.clone();
                    ComboBox::from_label("XPub Request Type")
                        .width(125.0)
                        .selected_text(format!("{:?}", request_type))
                        .show_ui(ui, |ui| {
                            for style in XPubRequestType::iter()
                            {
                                if style != XPubRequestType::Hot {
                                    ui.selectable_value(&mut self.xpub_type,
                                                        style.clone(),
                                                        format!("{:?}", style)
                                    );
                                }
                            }
                        });

                    self.derivation_path.view(ui);

                    if request_type == XPubRequestType::Cold {
                        LocalState::send_update(&updates, |lss| {
                            lss.wallet_state.update_hardware();
                        });
                        ui.horizontal(|ui| {
                            ui.label("Hardware Wallet: ");
                            let connected = device_list.is_some();
                            if connected {
                                ui.label(RichText::new("Connected").color(Color32::GREEN));
                            } else {
                                ui.label(RichText::new("Not Connected").color(Color32::RED));
                            }
                        });
                        // ui.spacing();
                        ui.label(device_list.unwrap_or("".to_string()));
                    }

                    if request_type == XPubRequestType::QR {
                        ui.label("QR code not yet supported");
                    }

                    if ui.button("Request XPub").clicked() {

                        let xpub = match self.xpub_type {
                            XPubRequestType::Cold => {
                                self.message = "Awaiting input on device...".to_string();
                                get_cold_xpub(self.derivation_path.derivation_path.clone())
                            }
                            XPubRequestType::QR => {
                                Err(error_info("QR code not yet supported"))
                            }
                            _ => {Err(error_info("Not yet supported"))}
                        };

                        self.result = Some(xpub.clone());

                        if let Some(xp) = xpub.log_error().ok() {
                            self.message = "Success".to_string();

                        } else {
                            self.message = "Error".to_string();
                        }

                    }

                    ui.label(self.message.clone());

                    if let Some(r) = self.result.clone() {
                        if let Ok(xpub) = r {
                            ui.label("XPub:");
                            let mut string = xpub.clone();
                            let mut string2 = string.clone();
                            bounded_text_area_size(ui, &mut string, 300.0, 4);
                            copy_to_clipboard(ui, string2.clone());
                            editable_text_input_copy(ui, "Save Name", &mut self.save_name, 150.0);
                            if ui.button("Save").clicked() {
                                let named = NamedXpub{
                                    name: self.save_name.clone(),
                                    derivation_path: self.derivation_path.derivation_path.clone(),
                                    xpub: string2.clone(),
                                    hot_offset: None,
                                    key_name_source: None,
                                    device_id: None,
                                    key_reference_source: None,
                                    key_nickname_source: None,
                                    request_type: Some(request_type.clone()),
                                    skip_persist: None,
                                };
                                LocalState::send_update(&updates, move |lss| {
                                    let named2 = named.clone();
                                    lss.add_named_xpubs(true, vec![named2], false).log_error().ok();
                                    lss.persist_local_state_store();
                                });

                            };
                        }
                    }

                });
            });
    }
}


pub fn get_cold_xpub(dp: String) -> RgResult<String> {
    let node = trezor::get_public_node(dp)?;
    let w = XpubWrapper::new(node.xpub);
    w.public_at(0, 0)?;
    Ok(w.xpub)
}

