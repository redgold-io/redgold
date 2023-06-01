use std::time::Instant;
use chrono::Local;
use eframe::egui;
use eframe::egui::{Button, Color32, RichText, TextStyle, Ui};
use crate::gui::app_loop::LocalState;

use strum::IntoEnumIterator; // 0.17.1
use strum_macros::{EnumIter, EnumString};
use tracing::{error, info};
use redgold_schema::{SafeOption, structs};
use redgold_schema::structs::{Address, ErrorInfo, NetworkEnvironment, PublicKey};
use crate::hardware::trezor;
use crate::hardware::trezor::trezor_list_devices;
use redgold_schema::EasyJson;
use redgold_schema::transaction::rounded_balance_i64;
use crate::core::internal_message::{Channel, new_channel, SendErrorInfo};
use crate::node_config::NodeConfig;
use crate::util::logging::Loggable;

#[derive(Debug, EnumIter, EnumString)]
#[repr(i32)]
pub enum WalletTab {
    Hardware,
    Software,
}

pub struct DeviceListStatus {
    pub device_output: Option<String>,
    last_polled: Instant,
}

impl DeviceListStatus {
    pub fn poll() -> Self {
        let result = trezor_list_devices().ok().flatten();
        Self {
            device_output: result,
            last_polled: Instant::now(),
        }
    }
}

// #[derive(Clone)]
pub struct StateUpdate {
    update: Box<dyn FnMut(&mut LocalState) + Send>
}

#[derive(Clone, PartialEq)]
enum SendReceiveTabs {
    Send,
    Receive,
}

pub struct WalletState {
    tab: WalletTab,
    device_list_status: DeviceListStatus,
    public_key: Option<PublicKey>,
    public_key_msg: Option<String>,
    // futs: Vec<impl Future>
    updates: Channel<StateUpdate>,
    send_receive: Option<SendReceiveTabs>,
    destination_address: String,
    amount_input: String,
    faucet_success: String,
    balance: String,
    balance_f64: Option<f64>
}


impl WalletState {
    pub fn new() -> Self {
        Self {
            tab: WalletTab::Hardware,
            device_list_status: DeviceListStatus::poll(),
            public_key: None,
            public_key_msg: None,
            updates: new_channel(),
            send_receive: None,
            destination_address: "".to_string(),
            amount_input: "".to_string(),
            faucet_success: "".to_string(),
            balance: "loading".to_string(),
            balance_f64: None,
        }
    }
    pub fn update_hardware(&mut self) {
        if self.device_list_status.last_polled.elapsed().as_secs() > 5 {
            self.device_list_status = DeviceListStatus::poll();
        }
    }
}

pub fn copy_to_clipboard(ui: &mut Ui, text: String) {
    let style = ui.style_mut();
    style.override_text_style = Some(TextStyle::Small);
    if ui.small_button("Copy").clicked() {
        ui.ctx().output_mut(|o| o.copied_text = text.clone());
    }
}

pub fn data_item(ui: &mut Ui, label: String, text: String) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
        ui.label(label);
        let text_line = &mut text.clone();
        ui.add(egui::TextEdit::singleline(text_line).clip_text(false));
        copy_to_clipboard(ui, text.clone());
    });
}

pub fn wallet_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    match local_state.wallet_state.updates.recv_while() {
        Ok(updates) => {
            for mut update in updates {
                info!("Received item of update, applying");
                (update.update)(local_state);
                info!("New wallet state faucet message: {}", local_state.wallet_state.faucet_success.clone());
            }
        }
        Err(e) => {error!("Error receiving updates: {}", e.json_or())}
    }
    // local_state.wallet_state.updates.receiver.tr
    let state = &mut local_state.wallet_state;
    state.update_hardware();
    ui.style_mut().spacing.item_spacing.y = 2f32;

    ui.heading("Wallet");
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Cold Hardware").clicked() {
            state.tab = WalletTab::Hardware;
            // device_list_status = None;
        }
        if ui.button("Hot Software").clicked() {
            state.tab = WalletTab::Software;
        }
    });

    ui.separator();

    // ui.spacing();
    // ui.label(format!("{:?}", state.tab));

    match state.tab {
        WalletTab::Hardware => {
            state.update_hardware();
            ui.horizontal(|ui| {
                ui.label("Hardware Wallet: ");
                let connected = state.device_list_status.device_output.is_some();
                if connected {
                    ui.label(RichText::new("Connected").color(Color32::GREEN));
                } else {
                    ui.label(RichText::new("Not Connected").color(Color32::RED));
                }

            });
            // ui.spacing();
            ui.label(state.device_list_status.device_output.clone().unwrap_or("".to_string()));
            // ui.separator();
            ui.horizontal(|ui| {

                ui.horizontal(|ui| {
                    ui.label("Default Path: ");
                    let string = &mut trezor::default_pubkey_path();
                    ui.add(egui::TextEdit::singleline(string).desired_width(150.0));
                    copy_to_clipboard(ui, string.clone());
                });
                ui.spacing();
                if ui.button("Refresh Public Key").clicked() {
                    state.public_key = None;
                    state.public_key_msg = Some("Awaiting input on device...".to_string());
                    // This blocks the entire UI... ah jeez
                    match trezor::default_pubkey() {
                        Ok(pk) => {
                            state.public_key = Some(pk.clone());
                            state.public_key_msg = Some("Got public key".to_string());
                            get_balance(local_state.node_config.clone(), pk.address().expect("").clone(),
                                          NetworkEnvironment::Dev,
                                          state.updates.sender.clone()
                            );
                        }
                        Err(e) => {
                            state.public_key_msg = Some("Error getting public key".to_string());
                            error!("Error getting public key: {}", e.json_or());
                        }
                    }
                }
                ui.label(state.public_key_msg.clone().unwrap_or("Refresh to get public".to_string()));
            });
            // ui.spacing();
            // ui.spacing();
            // ui.spacing();
            // ui.spacing();

            if let Some(pk) = &state.public_key {
                let hex = &mut pk.hex().unwrap_or("Hex failure".to_string());
                data_item(ui, "Public".to_string(), hex.clone());
                let address_str = pk.address()
                    .and_then(|a| a.render_string())
                    .unwrap_or("Address failure".to_string());
                data_item(ui, "Address".to_string(), address_str);

                // TODO: Include bitcoin address / ETH address for path 0 here for verification.
                ui.separator();

                ui.horizontal(|ui| {
                    let style = ui.style_mut();
                    style.override_text_style = Some(TextStyle::Heading);
                    if ui.button("Send").clicked() {
                        let some = Some(SendReceiveTabs::Send);
                        if state.send_receive == some.clone() {
                            state.send_receive = None;
                        } else {
                            state.send_receive = some;
                        }
                    }
                    if ui.button("Receive").clicked() {
                        let some = Some(SendReceiveTabs::Receive);
                        if state.send_receive == some.clone() {
                            state.send_receive = None;
                        } else {
                            state.send_receive = some;
                        }
                    }

                    ui.heading(RichText::new(format!("Balance: {}", state.balance.clone()))
                        .color(Color32::GREEN));

                    let layout = egui::Layout::right_to_left(egui::Align::RIGHT);
                    ui.with_layout(layout, |ui| {
                        if ui.button("Debug Faucet").clicked() {
                            let address = pk.address().expect("a");
                            handle_faucet(local_state.node_config.clone(), address,
                                          NetworkEnvironment::Dev,
                                state.updates.sender.clone()
                            );
                        };
                        ui.label(state.faucet_success.clone());
                        if ui.button("Refresh Balance").clicked() {
                            let address = pk.address().expect("a");
                            get_balance(local_state.node_config.clone(), address,
                                          NetworkEnvironment::Dev,
                                          state.updates.sender.clone()
                            );
                        };
                    });

                });

                ui.separator();
                ui.spacing();

                if let Some(srt) = &state.send_receive.clone() {
                    match srt {
                        SendReceiveTabs::Send => {
                            ui.horizontal(|ui| {
                                ui.label("Destination Address");
                                let string = &mut state.destination_address;
                                ui.add(egui::TextEdit::singleline(string).desired_width(460.0));
                                copy_to_clipboard(ui, string.clone());
                                let valid_addr = Address::parse(string.clone()).is_ok();
                                if valid_addr {
                                    ui.label(RichText::new("Valid").color(Color32::GREEN));
                                } else {
                                    ui.label(RichText::new("Invalid").color(Color32::RED));
                                }
                            });
                            // TODO: Amount USD and conversions etc.
                            ui.horizontal(|ui| {
                                ui.label("Amount");
                                let string = &mut state.amount_input;
                                ui.add(egui::TextEdit::singleline(string).desired_width(200.0));
                            });
                        }
                        SendReceiveTabs::Receive => {

                        }
                    }
                }

            }
            ui.spacing();


            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));

        }
        WalletTab::Software => {

        }
    }
}


pub fn get_balance(
    node_config: NodeConfig, address: Address, network: NetworkEnvironment,
    update_channel: flume::Sender<StateUpdate>
) {
    let mut node_config = node_config;
    node_config.network = network;
    let _ = tokio::spawn(async move {
        let client = node_config.lb_client();
        let response = client
            .balance(address).await;
        let fun: Box<dyn FnMut(&mut LocalState) + Send> = match response {
            Ok(o) => {
                info!("balance success: {}", o.json_or());
                Box::new(move |ls: &mut LocalState| {
                    info!("Applied update function inside closure for balance thing");
                    ls.wallet_state.balance = o.to_string();
                    ls.wallet_state.balance_f64 = Some(o)
                })
            }
            Err(e) => {
                error!("balance error: {}", e.json_or());
                Box::new(move |ls: &mut LocalState| {
                    ls.wallet_state.balance = "error".to_string();
                })
            }
        };
        let up = StateUpdate {
            update: fun,
        };
        update_channel.send_err(up).log_error().ok();
    });

}


fn handle_faucet(
    node_config: NodeConfig,
    address: Address,
    network: NetworkEnvironment,
    update_channel: flume::Sender<StateUpdate>
) {
    let mut node_config = node_config;
    node_config.network = network;
    let _ = tokio::spawn(async move {
        let client = node_config.lb_client();
        let fun = match client.faucet(&address).await {
            Ok(o) => {
                info!("Faucet success: {}", o.json_or());
                |ls: &mut LocalState| {
                    info!("Applied update function inside closure for faucet thing");
                    ls.wallet_state.faucet_success = "Faucet success".to_string();
                }
            }
            Err(e) => {
                error!("Faucet error: {}", e.json_or());
                |ls: &mut LocalState| {
                    ls.wallet_state.faucet_success = "Faucet error".to_string();
                }
            }
        };
        let up = StateUpdate {
            update: Box::new(fun),
        };
        update_channel.send_err(up).log_error().ok();
    });

}
