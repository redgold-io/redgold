use std::future::Future;
use std::time::Instant;
use eframe::egui;
use eframe::egui::{Color32, RichText, ScrollArea, TextStyle, Ui, Widget};
use flume::Sender;
use crate::gui::app_loop::LocalState;

use strum::IntoEnumIterator; // 0.17.1
use strum_macros::{EnumIter, EnumString};
use tracing::{error, info};
use redgold_keys::TestConstants;
use redgold_keys::transaction_support::{TransactionBuilderSupport, TransactionSupport};
use redgold_schema::{RgResult, WithMetadataHashable};
use redgold_schema::structs::{Address, AddressInfo, ErrorInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, Transaction, CurrencyAmount};
use crate::hardware::trezor;
use crate::hardware::trezor::trezor_list_devices;
use redgold_schema::EasyJson;
use redgold_schema::transaction::rounded_balance_i64;
use redgold_schema::transaction_builder::TransactionBuilder;
use redgold_keys::util::mnemonic_support::WordsPass;
use crate::core::internal_message::{Channel, new_channel, SendErrorInfo};
use crate::gui::{cold_wallet, common, hot_wallet};
use crate::gui::common::{data_item, editable_text_input_copy, medium_data_item, valid_label};
use crate::node_config::NodeConfig;
use redgold_schema::util::lang_util::JsonCombineResult;
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
    pub(crate) device_list_status: DeviceListStatus,
    pub(crate) public_key: Option<PublicKey>,
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
    unsigned_transaction_hash: Option<String>,
    signed_transaction: Option<Result<Transaction, ErrorInfo>>,
    signed_transaction_hash: Option<String>,
    signing_flow_status: Option<String>,
    signing_flow_transaction_box_msg: Option<String>,
    broadcast_transaction_response: Option<Result<SubmitTransactionResponse, ErrorInfo>>,
    pub hot_mnemonic: String,
    pub derivation_path: String,
    pub derivation_path_valid: bool,
    pub derivation_path_last_check: String,
    pub mnemonic_checksum: String,
}


impl WalletState {

    pub fn clear_data(&mut self) {
        self.update_unsigned_tx(None);
        self.update_signed_tx(None);
        self.signing_flow_status = None;
        self.broadcast_transaction_response = None;
        self.signing_flow_transaction_box_msg = None;
        self.faucet_success = "".to_string();
        self.balance = "".to_string();
        self.balance_f64 = None;
        self.destination_address = "".to_string();
        self.address_info = None;
        self.public_key = None;
        self.send_receive = None;
    }

    pub fn update_signed_tx(&mut self, tx_o: Option<RgResult<Transaction>>) {
        if let Some(tx) = tx_o.as_ref().and_then(|tx| tx.as_ref().ok()) {
            self.signed_transaction_hash = Some(tx.hash_hex_or_missing());
            self.signed_transaction = tx_o.clone();
        } else {
            self.signed_transaction_hash = None;
            self.signed_transaction = None;
        }
    }

    pub fn update_unsigned_tx(&mut self, tx_o: Option<RgResult<Transaction>>) {
        if let Some(tx) = tx_o.as_ref().and_then(|tx| tx.as_ref().ok()) {
            self.unsigned_transaction_hash = Some(tx.hash_hex_or_missing());
            self.prepared_transaction = tx_o.clone()
        } else {
            self.signed_transaction_hash = None;
            self.prepared_transaction = None;
        }
    }

    pub fn hot_mnemonic(&self) -> WordsPass {
        WordsPass::words(self.hot_mnemonic.clone())
    }

    pub fn new(hot_mnemonic: String) -> Self {
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
            unsigned_transaction_hash: None,
            signed_transaction: None,
            signed_transaction_hash: None,
            signing_flow_status: None,
            signing_flow_transaction_box_msg: None,
            broadcast_transaction_response: None,
            hot_mnemonic,
            derivation_path: trezor::default_pubkey_path(),
            derivation_path_valid: true,
            derivation_path_last_check: "".to_string(),
            mnemonic_checksum: "".to_string(),
        }
    }
    pub fn update_hardware(&mut self) {
        if self.device_list_status.last_polled.elapsed().as_secs() > 5 {
            self.device_list_status = DeviceListStatus::poll();
        }
    }
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


pub trait ValidateDerivationPath {
    fn valid_derivation_path(&self) -> bool;
}

impl ValidateDerivationPath for String {
    fn valid_derivation_path(&self) -> bool {
        WordsPass::words(TestConstants::new().words).public_at(self.clone()).is_ok()
    }
}



pub fn wallet_screen_scrolled(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState) {

    // local_state.wallet_state.updates.receiver.tr
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Cold Hardware").clicked() {
            ls.wallet_state.clear_data();
            ls.wallet_state.derivation_path = trezor::default_pubkey_path();
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
            cold_wallet::cold_header(ui, &mut ls.wallet_state);
        }
        WalletTab::Software => {
            hot_wallet::hot_header(&mut ls.wallet_state, ui, ctx);
        }
    }

    derivation_path_section(ui, ls);

    if let Some(pk) = ls.wallet_state.public_key.clone() {
        proceed_from_pk(ui, ls, &pk);
    }
    ui.spacing();


}

fn proceed_from_pk(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {
    data_item(ui, "Public Key:", pk.hex_or());
    let address_str = pk.address()
        .and_then(|a| a.render_string())
        .unwrap_or("Address failure".to_string());
    data_item(ui, "Address", address_str);

    // TODO: Include bitcoin address / ETH address for path 0 here for verification.
    ui.separator();

    send_receive_bar(ui, ls, pk);

    ui.separator();
    ui.spacing();

    if let Some(srt) = &ls.wallet_state.send_receive.clone() {
        match srt {
            SendReceiveTabs::Send => {
                send_view(ui, ls, pk);
            }
            SendReceiveTabs::Receive => {}
        }
    }
}

fn send_view(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {
    ui.horizontal(|ui| {
        ui.label("Destination Address");
        let string = &mut ls.wallet_state.destination_address;
        ui.add(egui::TextEdit::singleline(string).desired_width(460.0));
        common::copy_to_clipboard(ui, string.clone());
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
        let string = &mut ls.wallet_state.amount_input;
        ui.add(egui::TextEdit::singleline(string).desired_width(200.0));
    });

    if ui.button("Prepare Transaction").clicked() {
        match &ls.wallet_state.address_info {
            None => {}
            Some(ai) => {
                let result = prepare_transaction(
                    ai,
                    &ls.wallet_state.amount_input,
                    &ls.wallet_state.destination_address
                );
                ls.wallet_state.update_unsigned_tx(Some(result.clone()));
                ls.wallet_state.signing_flow_transaction_box_msg = Some(
                    result.clone().json_or_combine()
                );
                let status = result.map(|_x| "Transaction Prepared".to_string())
                    .unwrap_or("Preparation Failed".to_string());
                ls.wallet_state.signing_flow_status = Some(status);
            }
        }
    }
    if let Some(p) = &ls.wallet_state.signing_flow_transaction_box_msg {
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
    if let Some(res) = &ls.wallet_state.prepared_transaction {
        if let Some(t) = res.as_ref().ok() {
            ui.allocate_ui(egui::Vec2::new(500.0, 0.0), |ui| {
                ui.centered_and_justified(|ui| {
                    medium_data_item(ui, "Unsigned Transaction Hash:".to_string(), t.hash_hex_or_missing());
                });
            });
            if ui.button("Sign Transaction").clicked() {
                match ls.wallet_state.tab {
                    WalletTab::Hardware => {
                        initiate_hardware_signing(
                            t.clone(),
                            ls.wallet_state.updates.sender.clone(),
                            pk.clone().clone()
                        );
                        ls.wallet_state.signing_flow_status = Some("Awaiting hardware response...".to_string());
                    }
                    WalletTab::Software => {
                        let kp = ls.wallet_state.hot_mnemonic().keypair_at(ls.wallet_state.derivation_path.clone()).expect("kp");
                        let mut t2 = t.clone();
                        let signed = t2.sign(&kp);
                        ls.wallet_state.update_signed_tx(Some(signed));
                    }
                }
            }
        }
    }
    if let Some(m) = &ls.wallet_state.signing_flow_status {
        ui.label(m);
    }
    if let Some(t) = &ls.wallet_state.signed_transaction {
        if let Some(t) = t.as_ref().ok() {
            medium_data_item(ui,"Signed TX Hash:", ls.wallet_state.signed_transaction_hash.clone().unwrap_or("error".to_string()));
            if ui.button("Broadcast Transaction").clicked() {
                broadcast_transaction(
                    ls.node_config.clone(),
                    t.clone(),
                    NetworkEnvironment::Dev,
                    ls.wallet_state.updates.sender.clone()
                );
                ls.wallet_state.signing_flow_status = Some("Awaiting broadcast response...".to_string());
            }
        }
    }
}

fn send_receive_bar(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Heading);
        if ui.button("Send").clicked() {
            let some = Some(SendReceiveTabs::Send);
            if ls.wallet_state.send_receive == some.clone() {
                ls.wallet_state.send_receive = None;
            } else {
                ls.wallet_state.send_receive = some;
            }
        }
        if ui.button("Receive").clicked() {
            let some = Some(SendReceiveTabs::Receive);
            if ls.wallet_state.send_receive == some.clone() {
                ls.wallet_state.send_receive = None;
            } else {
                ls.wallet_state.send_receive = some;
            }
        }

        ui.heading(RichText::new(format!("Balance: {}", ls.wallet_state.balance.clone()))
            .color(Color32::LIGHT_GREEN));

        let layout = egui::Layout::right_to_left(egui::Align::RIGHT);
        ui.with_layout(layout, |ui| {
            if ui.button("Debug Faucet").clicked() {
                let address = pk.address().expect("a");
                handle_faucet(ls.node_config.clone(), address,
                              NetworkEnvironment::Dev,
                              ls.wallet_state.updates.sender.clone()
                );
            };
            ui.label(ls.wallet_state.faucet_success.clone());
            if ui.button("Refresh Balance").clicked() {
                let address = pk.address().expect("a");
                get_address_info(ls.node_config.clone(), address,
                                 NetworkEnvironment::Dev,
                                 ls.wallet_state.updates.sender.clone()
                );
            };
        });
    });
}

pub fn derivation_path_section(ui: &mut Ui, ls: &mut LocalState) {
    ui.horizontal(|ui| {
        ui.horizontal(|ui| {

            editable_text_input_copy(ui, "Derivation Path", &mut ls.wallet_state.derivation_path, 150.0);
            if ls.wallet_state.derivation_path != ls.wallet_state.derivation_path_last_check {
                ls.wallet_state.derivation_path_last_check = ls.wallet_state.derivation_path.clone();
                ls.wallet_state.derivation_path_valid = ls.wallet_state.derivation_path.valid_derivation_path();
            }
            valid_label(ui, ls.wallet_state.derivation_path_valid);
        });
        ui.spacing();
        match ls.wallet_state.tab {
            WalletTab::Hardware => {
                if ui.button("Refresh Public Key").clicked() {
                    ls.wallet_state.public_key = None;
                    ls.wallet_state.public_key_msg = Some("Awaiting input on device...".to_string());
                    // This blocks the entire UI... ah jeez
                    match trezor::default_pubkey() {
                        Ok(pk) => {
                            ls.wallet_state.public_key = Some(pk.clone());
                            ls.wallet_state.public_key_msg = Some("Got public key".to_string());
                            get_address_info(ls.node_config.clone(), pk.address().expect("").clone(),
                                             NetworkEnvironment::Dev,
                                             ls.wallet_state.updates.sender.clone()
                            );
                        }
                        Err(e) => {
                            ls.wallet_state.public_key_msg = Some("Error getting public key".to_string());
                            error!("Error getting public key: {}", e.json_or());
                        }
                    }
                }
                ui.label(ls.wallet_state.public_key_msg.clone().unwrap_or("Refresh to get public".to_string()));
            }
            _ => {}
        }
    });
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
            .map(|_x| "Transaction Accepted".to_string())
            .unwrap_or("Rejected Transaction".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet_state.broadcast_transaction_response = st.clone();
            ls.wallet_state.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet_state.signing_flow_status = ss.clone();
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
            .map(|_x| "Signed Successfully".to_string())
            .unwrap_or("Signing error".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet_state.update_signed_tx(st.clone());
            ls.wallet_state.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet_state.signing_flow_status = ss.clone();
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
    let amount = CurrencyAmount::from_float_string(amount)?;
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
