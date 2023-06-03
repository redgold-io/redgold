use std::future::Future;
use std::time::Instant;
use chrono::Local;
use eframe::egui;
use eframe::egui::{Button, Color32, Direction, Layout, RichText, ScrollArea, TextEdit, TextStyle, Ui, Widget};
use flume::Sender;
use crate::gui::app_loop::LocalState;

use strum::IntoEnumIterator; // 0.17.1
use strum_macros::{EnumIter, EnumString};
use surf::http::headers::ToHeaderValues;
use tokio::task::spawn_blocking;
use tracing::{error, info};
use redgold_schema::{SafeOption, structs, WithMetadataHashable};
use redgold_schema::structs::{Address, AddressInfo, ErrorInfo, NetworkEnvironment, Proof, PublicKey, SubmitTransactionResponse, Transaction, TransactionAmount};
use crate::hardware::trezor;
use crate::hardware::trezor::trezor_list_devices;
use redgold_schema::EasyJson;
use redgold_schema::transaction::rounded_balance_i64;
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::core::internal_message::{Channel, map_fut, new_channel, SendErrorInfo};
use crate::node_config::NodeConfig;
use crate::util::lang_util::JsonCombineResult;
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
    balance_f64: Option<f64>,
    address_info: Option<AddressInfo>,
    prepared_transaction: Option<Result<Transaction, ErrorInfo>>,
    signed_transaction: Option<Result<Transaction, ErrorInfo>>,
    signing_status: Option<String>,
    signing_flow_transaction_box_msg: Option<String>,
    broadcast_transaction_response: Option<Result<SubmitTransactionResponse, ErrorInfo>>
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
            address_info: None,
            prepared_transaction: None,
            signed_transaction: None,
            signing_status: None,
            signing_flow_transaction_box_msg: None,
            broadcast_transaction_response: None,
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

pub fn medium_data_item(ui: &mut Ui, label: String, text: String) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.spacing();
        ui.label(text.clone());
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
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
        Err(e) => { error!("Error receiving updates: {}", e.json_or()) }
    }
    local_state.wallet_state.update_hardware();
    ui.style_mut().spacing.item_spacing.y = 2f32;
    ui.heading("Wallet");
    ScrollArea::vertical().show(ui, |ui| wallet_screen_scrolled(ui, ctx, local_state));
}

pub fn big_button<S: Into<String>>(mut ui: Ui, lb: S) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Heading);
        ui.button(lb.into())
    });
}

pub fn wallet_screen_scrolled(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {

    // local_state.wallet_state.updates.receiver.tr
    let state = &mut local_state.wallet_state;
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
                            get_address_info(local_state.node_config.clone(), pk.address().expect("").clone(),
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
                        .color(Color32::LIGHT_GREEN));

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
                            get_address_info(local_state.node_config.clone(), address,
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

                            if ui.button("Prepare Transaction").clicked() {
                                match &state.address_info {
                                    None => {

                                    }
                                    Some(ai) => {
                                        let result = prepare_transaction(
                                            ai,
                                            &state.amount_input,
                                            &state.destination_address
                                        );
                                        state.prepared_transaction = Some(result.clone());
                                        state.signing_flow_transaction_box_msg = Some(
                                            result.clone().json_or_combine()
                                        );
                                        let status = result.map(|x| "Transaction Prepared".to_string())
                                            .unwrap_or("Preparation Failed".to_string());
                                        state.signing_status = Some(status);
                                    }
                                }
                            }
                            if let Some(p) = &state.signing_flow_transaction_box_msg {
                                // ui.with_layout(
                                //     Layout::centered_and_justified(Direction::TopDown)
                                //     ,|ui|
                                ui.label("Rendered Transaction Information"); //);
                                ui.spacing();
                                let string1 = &mut p.clone();
                                ui.horizontal(|ui| {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        egui::TextEdit::multiline(string1)
                                            .desired_width(600.0)
                                            .desired_rows(2)
                                            .clip_text(true)
                                            .ui(ui);
                                    });
                                });
                            }
                            if let Some(res) = &state.prepared_transaction {
                                if let Some(t) = res.as_ref().ok() {
                                    ui.allocate_ui(egui::Vec2::new(500.0, 0.0), |ui| {
                                        ui.centered_and_justified(|ui| {
                                            medium_data_item(ui, "Transaction Hash:".to_string(), t.hash_hex_or_missing());
                                        });
                                    });
                                    if ui.button("Sign Transaction").clicked() {
                                        initiate_hardware_signing(
                                            t.clone(),
                                            state.updates.sender.clone(),
                                            pk.clone()
                                        );
                                        state.signing_status = Some("Awaiting hardware response...".to_string());
                                    }
                                }
                            }
                            if let Some(m) = &state.signing_status {
                                ui.label(m);
                            }
                            if let Some(t) = &state.signed_transaction {
                                if let Some(t) = t.as_ref().ok() {
                                    if ui.button("Broadcast Transaction").clicked() {
                                        broadcast_transaction(
                                            local_state.node_config.clone(),
                                            t.clone(),
                                            NetworkEnvironment::Dev,
                                            state.updates.sender.clone()
                                        );
                                        state.signing_status = Some("Awaiting broadcast response...".to_string());
                                    }
                                }
                            }
                        }
                        SendReceiveTabs::Receive => {

                        }
                    }
                }

            }
            ui.spacing();


            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));
            ui.label(format!("X:{} Y:{}", ui.available_size().x.to_string(), ui.available_size().y.to_string()));

        }
        WalletTab::Software => {

        }
    }
}

//
//
// fn spawn_update(fun: impl Future<Output = Box<dyn FnMut(&mut LocalState) + Send>> + std::marker::Send, update_channel: Sender<StateUpdate>) {
//     tokio::spawn(async move {
//         let res = fun.await;
//         let up = StateUpdate {
//             update: Box::new(res),
//         };
//         update_channel.send_err(up).log_error().ok();
//     });
// }

// TODO: Abstract over spawn/send updates
fn broadcast_transaction(nc: NodeConfig, tx: Transaction, ne: NetworkEnvironment, send: Sender<StateUpdate>) {
    tokio::spawn(async move {
        let mut nc = nc.clone();
        nc.network = ne;
        let res = nc.clone().lb_client().send_transaction(&tx.clone(), true).await;

        let st = Some(res.clone());
        let st_msg = Some(res.clone().json_or_combine());
        let ss = Some(res
            .map(|x| "Transaction Accepted".to_string())
            .unwrap_or("Rejected Transaction".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet_state.broadcast_transaction_response = st.clone();
            ls.wallet_state.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet_state.signing_status = ss.clone();
        };
        let up = StateUpdate {
            update: Box::new(fun),
        };
        send.send_err(up).log_error().ok();
    });
}

fn initiate_hardware_signing(t: Transaction, send: Sender<StateUpdate>, public: PublicKey) {
    tokio::spawn(async move {
        let t = &mut t.clone();
        let res = trezor::sign_transaction(
            t, public, trezor::default_pubkey_path())
            .await
            .log_error()
            .map(|x| x.clone())
            .map_err(|e| e.clone());

        let st = Some(res.clone());
        let st_msg = Some(res.clone().json_or_combine());
        let ss = Some(res
            .map(|x| "Signed Successfully".to_string())
            .unwrap_or("Signing error".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet_state.signed_transaction = st.clone();
            ls.wallet_state.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet_state.signing_status = ss.clone();
        };
        let up = StateUpdate {
            update: Box::new(fun),
        };
        send.send_err(up).log_error().ok();
    });
}

pub fn prepare_transaction(ai: &AddressInfo, amount: &String, destination: &String)
    -> Result<Transaction, ErrorInfo> {
    let destination = Address::parse(destination.clone())?;
    let amount = TransactionAmount::from_float_string(amount)?;
    let mut tb = TransactionBuilder::new();
    tb.with_address_info(ai.clone());
    tb.with_output(&destination, &amount);
    let res = tb.build();
    res
}


pub fn get_address_info(
    node_config: NodeConfig, address: Address, network: NetworkEnvironment,
    update_channel: flume::Sender<StateUpdate>
) {
    let mut node_config = node_config;
    node_config.network = network;
    let _ = tokio::spawn(async move {
        let client = node_config.lb_client();
        let response = client
            .address_info(address).await;
        let fun: Box<dyn FnMut(&mut LocalState) + Send> = match response {
            Ok(ai) => {
                info!("balance success: {}", ai.json_or());
                Box::new(move |ls: &mut LocalState| {
                    info!("Applied update function inside closure for balance thing");
                    let o = rounded_balance_i64(ai.balance.clone());
                    ls.wallet_state.balance = o.to_string();
                    ls.wallet_state.balance_f64 = Some(o.clone());
                    ls.wallet_state.address_info = Some(ai.clone());
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
