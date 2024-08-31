use std::future::Future;
use std::time::Instant;
use bdk::bitcoin::bech32::ToBase32;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, Context, RichText, ScrollArea, TextStyle, Ui, Widget};
use flume::Sender;
use itertools::{Either, Itertools};
use serde::Deserialize;
use crate::gui::app_loop::LocalState;

use strum::IntoEnumIterator;
// 0.17.1
use strum_macros::{EnumIter, EnumString};
use tracing::{error, info};
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::{KeyPair, TestConstants};
use redgold_keys::address_support::AddressSupport;
use redgold_keys::eth::historical_client::EthHistoricalClient;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, Hash, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use crate::hardware::trezor;
use crate::hardware::trezor::trezor_list_devices;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::transaction::rounded_balance_i64;
use crate::core::transact::tx_builder_supports::TransactionBuilder;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::core::internal_message::{Channel, new_channel, SendErrorInfo};
use crate::gui::common;
use crate::gui::common::{bounded_text_area, data_item, data_item_multiline_fixed, editable_text_input_copy, medium_data_item, valid_label};
use crate::node_config::NodeConfig;
use redgold_schema::util::lang_util::JsonCombineResult;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::local_stored_state::{NamedXpub, StoredMnemonic, StoredPrivateKey, XPubRequestType};
use redgold_schema::proto_serde::ProtoSerde;
use crate::core::transact::tx_builder_supports::TransactionBuilderSupport;
use crate::gui::components::passphrase_input::PassphraseInput;
use crate::gui::components::xpub_req;
use crate::gui::tabs::keys::keys_tab::internal_stored_xpubs;
use crate::gui::tabs::transact::{address_query, broadcast_tx, cold_wallet, hardware_signing, hot_wallet, prepare_tx, prepared_tx_view};


#[derive(Debug, EnumIter, EnumString, PartialEq)]
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
    pub(crate) update: Box<dyn FnMut(&mut LocalState) + Send>,
}

#[derive(Clone, PartialEq)]
pub enum SendReceiveTabs {
    Send,
    // Receive,
    CustomTx,
    // Swap
}

#[derive(Clone, PartialEq, EnumString)]
pub enum CustomTransactionType {
    Swap,
    Stake
}

// #[derive(Clone)]
pub struct WalletState {
    pub tab: WalletTab,
    pub(crate) device_list_status: DeviceListStatus,
    pub(crate) public_key: Option<PublicKey>,
    pub(crate) public_key_msg: Option<String>,
    // futs: Vec<impl Future>
    pub(crate) updates: Channel<StateUpdate>,
    pub send_receive: Option<SendReceiveTabs>,
    pub destination_address: String,
    pub amount_input: String,
    faucet_success: String,
    pub balance: String,
    pub balance_f64: Option<f64>,
    pub balance_btc: Option<String>,
    pub balance_btc_f64: Option<f64>,
    pub balance_eth: Option<String>,
    pub balance_eth_f64: Option<f64>,
    pub address_info: Option<AddressInfo>,
    pub prepared_transaction: Option<Result<Transaction, ErrorInfo>>,
    pub unsigned_transaction_hash: Option<String>,
    pub signed_transaction: Option<Result<Transaction, ErrorInfo>>,
    pub signed_transaction_hash: Option<String>,
    pub signing_flow_status: Option<String>,
    pub transaction_prepared_success: bool,
    pub transaction_sign_success: bool,
    pub signing_flow_transaction_box_msg: Option<String>,
    pub broadcast_transaction_response: Option<Result<SubmitTransactionResponse, ErrorInfo>>,
    pub show_btc_info: bool,
    pub hot_mnemonic_default: String,
    pub send_currency_type: SupportedCurrency,
    pub active_hot_mnemonic: Option<String>,
    pub active_hot_private_key_hex: Option<String>,
    pub derivation_path: String,
    pub xpub_derivation_path: String,
    pub derivation_path_valid: bool,
    pub xpub_derivation_path_valid: bool,
    pub derivation_path_last_check: String,
    pub xpub_derivation_path_last_check: String,
    pub mnemonic_or_key_checksum: String,
    pub seed_checksum: Option<String>,
    pub active_xpub: String,
    pub active_derivation_path: String,
    pub xpub_save_name: String,
    pub mnemonic_save_name: String,
    pub mnemonic_save_data: String,
    pub is_mnemonic_or_kp: Option<bool>,
    pub valid_save_mnemonic: String,
    pub show_xpub_loader_window: bool,
    pub last_selected_xpub_name: String,
    pub selected_xpub_name: String,
    pub selected_key_name: String,
    pub last_selected_key_name: String,
    pub add_new_key_window: bool,
    pub show_save_xpub_window: bool,
    pub purge_existing_xpubs_on_save: bool,
    pub allow_xpub_name_overwrite: bool,
    pub xpub_loader_rows: String,
    pub xpub_loader_error_message: String,
    pub hot_passphrase: String,
    pub hot_passphrase_last: String,
    pub hot_offset: String,
    pub hot_offset_last: String,
    pub custom_tx_json: String,
    pub mnemonic_save_persist: bool,
    pub mark_output_as_stake: bool,
    pub mark_output_as_swap: bool,
    pub passphrase_input: PassphraseInput,
}

impl WalletState {
    pub(crate) fn update_hot_mnemonic_or_key_info(&mut self) {
        self.mnemonic_or_key_checksum = self.checksum_key();
        if let Some((pkhex, key_pair)) = self.active_hot_private_key_hex.as_ref()
            .and_then(|kp| KeyPair::from_private_hex(kp.clone()).ok().map(|kp2| (kp.clone(), kp2))) {
            self.public_key = Some(key_pair.public_key());
            let hex = hex::decode(pkhex).unwrap_or(vec![]);
            let check = Hash::new_checksum(&hex);
            self.mnemonic_or_key_checksum = check;
        } else {
            let m = self.hot_mnemonic();
            let check = m.checksum_words().unwrap_or("".to_string());
            let pk = m.public_at(self.derivation_path.clone());
            self.public_key = pk.ok();
            self.mnemonic_or_key_checksum = check;
            self.seed_checksum = m.checksum().ok().clone();
        }
    }
}


impl WalletState {

    pub fn checksum_key(&self) -> String {
        if let Some(kp) = self.active_hot_private_key_hex.as_ref() {
            if let Some(b) = hex::decode(kp).ok() {
                return Hash::new_checksum(&b);
            }
        }
        return self.hot_mnemonic().checksum_words().unwrap_or("".to_string());
    }
    pub fn clear_data(&mut self) {
        self.update_unsigned_tx(None);
        self.update_signed_tx(None);
        self.signing_flow_status = None;
        self.broadcast_transaction_response = None;
        self.signing_flow_transaction_box_msg = None;
        self.faucet_success = "".to_string();
        self.balance_btc = None;
        self.balance = "".to_string();
        self.balance_f64 = None;
        self.balance_btc_f64 = None;
        self.destination_address = "".to_string();
        self.address_info = None;
        self.public_key = None;
        self.send_receive = None;
    }

    pub fn update_signed_tx(&mut self, tx_o: Option<RgResult<Transaction>>) {
        if let Some(tx) = tx_o.as_ref().and_then(|tx| tx.as_ref().ok()) {
            self.signed_transaction_hash = Some(tx.hash_hex());
            self.signed_transaction = tx_o.clone();
        } else {
            self.signed_transaction_hash = None;
            self.signed_transaction = None;
        }
    }

    pub fn update_unsigned_tx(&mut self, tx_o: Option<RgResult<Transaction>>) {
        if let Some(tx) = tx_o.as_ref().and_then(|tx| tx.as_ref().ok()) {
            self.unsigned_transaction_hash = Some(tx.hash_hex());
            self.prepared_transaction = tx_o.clone()
        } else {
            self.signed_transaction_hash = None;
            self.prepared_transaction = None;
        }
    }

    pub fn hot_mnemonic(&self) -> WordsPass {
        let pass = if self.hot_passphrase.is_empty() {
            None
        } else {
            Some(self.hot_passphrase.clone())
        };
        let m = self.active_hot_mnemonic.as_ref().unwrap_or(&self.hot_mnemonic_default);
        let mut w = WordsPass::new(m, pass.clone());
        if !self.hot_offset.is_empty() {
            w = w.hash_derive_words(self.hot_offset.clone()).expect("err");
            w.passphrase = pass;
        }
        w
    }

    pub fn new(hot_mnemonic: String, option: Option<&NamedXpub>) -> Self {
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
            balance_btc: None,
            balance_btc_f64: None,
            balance_eth: None,
            balance_eth_f64: None,
            address_info: None,
            prepared_transaction: None,
            unsigned_transaction_hash: None,
            signed_transaction: None,
            signed_transaction_hash: None,
            signing_flow_status: None,
            transaction_prepared_success: false,
            transaction_sign_success: false,
            signing_flow_transaction_box_msg: None,
            broadcast_transaction_response: None,
            show_btc_info: false,
            hot_mnemonic_default: hot_mnemonic,
            send_currency_type: SupportedCurrency::Redgold,
            active_hot_mnemonic: None,
            active_hot_private_key_hex: None,
            derivation_path: trezor::default_pubkey_path(),
            xpub_derivation_path: "m/44'/0'/0'".to_string(),
            derivation_path_valid: true,
            xpub_derivation_path_valid: false,
            derivation_path_last_check: "".to_string(),
            xpub_derivation_path_last_check: "".to_string(),
            mnemonic_or_key_checksum: "".to_string(),
            seed_checksum: None,
            active_xpub: "".to_string(),
            active_derivation_path: "".to_string(),
            xpub_save_name: "".to_string(),
            mnemonic_save_name: "".to_string(),
            mnemonic_save_data: "".to_string(),
            is_mnemonic_or_kp: None,
            show_save_xpub_window: false,
            purge_existing_xpubs_on_save: false,
            allow_xpub_name_overwrite: true,
            selected_xpub_name: option.map(|o| o.name.clone()).unwrap_or("Select Xpub".to_string()),
            selected_key_name: "default".to_string(),
            last_selected_key_name: "default".to_string(),
            show_xpub_loader_window: false,
            xpub_loader_rows: "".to_string(),
            xpub_loader_error_message: "".to_string(),
            hot_passphrase: "".to_string(),
            hot_passphrase_last: "".to_string(),
            hot_offset: "".to_string(),
            hot_offset_last: "".to_string(),
            custom_tx_json: "".to_string(),
            valid_save_mnemonic: "".to_string(),
            add_new_key_window: false,
            mnemonic_save_persist: true,
            mark_output_as_stake: false,
            mark_output_as_swap: false,
            last_selected_xpub_name: "".to_string(),
            passphrase_input: Default::default(),
        }
    }
    pub fn update_hardware(&mut self) {
        if self.device_list_status.last_polled.elapsed().as_secs() > 5 {
            self.device_list_status = DeviceListStatus::poll();
        }
    }
}

pub fn wallet_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState, has_changed_tab: bool) {
    match local_state.wallet_state.updates.recv_while() {
        Ok(updates) => {
            for mut update in updates {
                info!("Received item of update, applying");
                (update.update)(local_state);
                // info!("New wallet state faucet message: {}", local_state.wallet_state.faucet_success.clone());
            }
        }
        Err(e) => { error!("Error receiving updates: {}", e.json_or()) }
    }
    local_state.wallet_state.update_hardware();
    ui.style_mut().spacing.item_spacing.y = 2f32;
    ui.heading("Transact");
    ui.separator();
    ScrollArea::vertical().show(ui, |ui| wallet_screen_scrolled(ui, ctx, local_state, has_changed_tab));
}


pub fn wallet_screen_scrolled(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState, has_changed_tab: bool) {
    let (mut update, xpub) = internal_stored_xpubs(ls, ui, ctx, has_changed_tab);
    let mut is_hot = false;
    if has_changed_tab {
        update = true;
    }

    if let Some(x) = xpub {

        let mut passphrase_changed = false;

        if x.request_type != Some(XPubRequestType::Hot) {
            cold_wallet::hardware_connected(ui, &mut ls.wallet_state);
        } else {
            is_hot = true;
            passphrase_changed = ls.wallet_state.passphrase_input.view(ui);
        }

        if update || passphrase_changed {
            // Are either of these even used?
            // ls.wallet_state.active_xpub = x.xpub.clone();
            // ls.wallet_state.active_derivation_path = ls.keytab_state.derivation_path_xpub_input_account.derivation_path();
            if let Ok(pk) = ls.keytab_state.xpub_key_info.public_key() {
                ls.wallet_state.public_key = Some(pk.clone());
                if is_hot {
                    if check_assign_hot_key(ls, &x, &pk).is_err() {
                        // Err, refuse to proceed. Show error message to user
                        ls.wallet_state.public_key = None;
                    } else {

                    }
                }
            } else {
                ls.wallet_state.public_key = None;
            }
        }
    }

    if let Some(pk) = ls.wallet_state.public_key.clone() {
        ls.wallet_state.passphrase_input.err_msg = None;
        if update || (ls.wallet_state.address_info.is_none() && has_changed_tab) {
            ls.wallet_state.address_info = None;
            refresh_balance(ls);
        }
        if update {
            // let kp = ls.wallet_state.hot_mnemonic().keypair_at(ls.wallet_state.derivation_path.clone()).expect("kp");


        }
        proceed_from_pk(ui, ls, &pk, is_hot);
    }

}

fn check_assign_hot_key(ls: &mut LocalState, x: &NamedXpub, pk: &PublicKey) -> RgResult<()> {
    let key_name = x.key_name_source.as_ref().ok_msg("key_name")?;
    let k = ls.local_stored_state.by_key(&key_name).ok_msg("key")?;

    match k {
        Either::Left(m) => {
            let pass = ls.wallet_state.passphrase_input.passphrase.clone();
            let words = m.mnemonic.clone();
            let w = WordsPass::new(
                &words,
                Some(pass.clone())
            );
            w.validate()?;
            let dp = ls.keytab_state.derivation_path_xpub_input_account.derivation_path();
            let dp_xpub =  ls.keytab_state.xpub_key_info.derivation_path.clone();
            let pk2 = w.public_at(
                &dp
            )?;
            if &pk2 != pk {
                info!("Setting public key mismatch error for keyname {} checksum {} {} {} {} {} ",
                    key_name.clone(),
                    w.checksum().expect(""), dp.clone(), dp_xpub.clone(),
                    pk2.hex(), pk.hex()
                );
                ls.wallet_state.passphrase_input.err_msg = Some("Public key mismatch".to_string());
                return Err(error_info("Public key mismatch".to_string()));
            }
            ls.wallet_state.active_hot_mnemonic = Some(words);
            ls.wallet_state.hot_passphrase = pass;
            ls.wallet_state.derivation_path = dp.clone();
            ls.wallet_state.derivation_path_valid = true;
            ls.wallet_state.active_derivation_path = dp.clone();
            ls.wallet_state.xpub_derivation_path = dp.clone();
        }
        Either::Right(kpo) => {
            // Eek, need to get the xpub diff combo box for private keys? Or include it somehow
            // this is more complex here.
            return Err(error_info("Not yet implemented".to_string()));
            // let kp = KeyPair::from_private_hex(kpo.key_hex.clone())?;
            // let pk2 = kp.public_key();
            // if &pk2 != pk {
            //     return Err(error_info("Public key mismatch".to_string()));
            // }
            // ls.wallet_state.active_hot_private_key_hex = Some(kpo.key_hex.clone());
        }
    };
    Ok(())
}

pub fn hot_passphrase_section(ui: &mut Ui, ls: &mut LocalState) -> bool {

    let mut update_clicked = false;

    if &ls.wallet_state.hot_passphrase_last != &ls.wallet_state.hot_passphrase.clone() {
        ls.wallet_state.hot_passphrase_last = ls.wallet_state.hot_passphrase.clone();
        update_clicked = true;
    }
    if &ls.wallet_state.hot_offset_last != &ls.wallet_state.hot_offset.clone() {
        ls.wallet_state.hot_offset_last = ls.wallet_state.hot_offset.clone();
        update_clicked = true;
    }

        ui.horizontal(|ui| {
            ui.label("Passphrase:");
            egui::TextEdit::singleline(&mut ls.wallet_state.hot_passphrase)
                .desired_width(150f32)
                .password(true).show(ui);
            ui.label("Offset:");
            egui::TextEdit::singleline(&mut ls.wallet_state.hot_offset)
                .desired_width(150f32)
                .show(ui);
            if ui.button("Update").clicked() {
                update_clicked = true;
            };
        });
    if update_clicked {
        ls.wallet_state.update_hot_mnemonic_or_key_info();
    };
    update_clicked
}

fn proceed_from_pk(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey, is_hot: bool) {
    // data_item(ui, "Public Key:", pk.hex_or());
    // let address_str = pk.address()
    //     .and_then(|a| a.render_string())
    //     .unwrap_or("Address failure".to_string());
    // data_item(ui, "Address", address_str);

    // TODO: Include bitcoin address / ETH address for path 0 here for verification.
    ui.separator();

    let mut balance_str = format!("Balance RDG: {} ", ls.wallet_state.balance.clone());
    if let Some(b) = ls.wallet_state.balance_btc.as_ref() {
        balance_str = format!("{} BTC: {}", balance_str, b);
    };
    if let Some(b) = ls.wallet_state.balance_eth.as_ref() {
        balance_str = format!("{} ETH: {}", balance_str, b);
    };

    if ls.wallet_state.address_info.is_none() {
        balance_str = "loading address info".to_string();
    }

    ui.heading(RichText::new(balance_str).color(Color32::LIGHT_GREEN));

    // ui.checkbox(&mut ls.wallet_state.show_btc_info, "Show BTC Info / Enable BTC");
    // if ls.wallet_state.show_btc_info {
    //     data_item(ui, "BTC Address", pk.to_bitcoin_address(&ls.node_config.network).unwrap_or("".to_string()));
    // }

    send_receive_bar(ui, ls, pk);

    ui.separator();
    ui.spacing();

    if let Some(srt) = &ls.wallet_state.send_receive.clone() {
        let show_prepared = true;
        match srt {
            SendReceiveTabs::Send => {
                send_view(ui, ls, pk);
            }
            // SendReceiveTabs::Receive => {
            //     show_prepared = false;
            // }
            SendReceiveTabs::CustomTx => {
                ui.label("Enter custom transaction JSON:");
                ui.horizontal(|ui| bounded_text_area(ui, &mut ls.wallet_state.custom_tx_json));
            }
            // SendReceiveTabs::Swap => {
            //     // show_prepared = false;
            //     // swap_view(ui, ls, pk);
            // }
        }
        if show_prepared {
            prepared_tx_view::prepared_view(ui, ls, pk, is_hot);
        }
    }
}

fn send_view(ui: &mut Ui, ls: &mut LocalState, _pk: &PublicKey) {

    ComboBox::from_label("Currency")
        .selected_text(format!("{:?}", ls.wallet_state.send_currency_type))
        .show_ui(ui, |ui| {
            let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold];
            for style in styles {
                ui.selectable_value(&mut ls.wallet_state.send_currency_type, style.clone(), format!("{:?}", style));
            }
        });
    ui.horizontal(|ui| {
        ui.label("To:");
        let string = &mut ls.wallet_state.destination_address;
        ui.add(egui::TextEdit::singleline(string).desired_width(460.0));
        common::copy_to_clipboard(ui, string.clone());
        let valid_addr = string.parse_address().is_ok();
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
        // ui.checkbox(&mut ls.wallet_state.mark_output_as_stake, "Mark as Stake");
        // ui.checkbox(&mut ls.wallet_state.mark_output_as_swap, "Mark as Swap");
        // if ls.wallet_state.mark_output_as_stake {
        //     ui.checkbox(&mut ls.wallet_state.mark_output_as_stake, "Mark as Stake");
        // }
    });

}

fn swap_view(_ui: &mut Ui, _ls: &mut LocalState, _pk: &PublicKey) {
    //
    // ComboBox::from_label("Currency")
    //     .selected_text(format!("{:?}", ls.wallet_state.send_currency_type))
    //     .show_ui(ui, |ui| {
    //         let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold];
    //         for style in styles {
    //             ui.selectable_value(&mut ls.wallet_state.send_currency_type, style.clone(), format!("{:?}", style));
    //         }
    //     });
    // ui.horizontal(|ui| {
    //     ui.label("Destination Address");
    //     let string = &mut ls.wallet_state.destination_address;
    //     ui.add(egui::TextEdit::singleline(string).desired_width(460.0));
    //     common::copy_to_clipboard(ui, string.clone());
    //     let valid_addr = Address::parse(string.clone()).is_ok();
    //     if valid_addr {
    //         ui.label(RichText::new("Valid").color(Color32::GREEN));
    //     } else {
    //         ui.label(RichText::new("Invalid").color(Color32::RED));
    //     }
    // });
    // // TODO: Amount USD and conversions etc.
    // ui.horizontal(|ui| {
    //     ui.label("Amount");
    //     let string = &mut ls.wallet_state.amount_input;
    //     ui.add(egui::TextEdit::singleline(string).desired_width(200.0));
    // });
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
        // if ui.button("Receive").clicked() {
        //     let some = Some(SendReceiveTabs::Receive);
        //     if ls.wallet_state.send_receive == some.clone() {
        //         ls.wallet_state.send_receive = None;
        //     } else {
        //         ls.wallet_state.send_receive = some;
        //     }
        // }
        if ui.button("Custom Tx").clicked() {
            let some = Some(SendReceiveTabs::CustomTx);
            if ls.wallet_state.send_receive == some.clone() {
                ls.wallet_state.send_receive = None;
            } else {
                ls.wallet_state.send_receive = some;
            }
        }
        // if ui.button("Swap").clicked() {
        //     let some = Some(SendReceiveTabs::Swap);
        //     if ls.wallet_state.send_receive == some.clone() {
        //         ls.wallet_state.send_receive = None;
        //     } else {
        //         ls.wallet_state.send_receive = some;
        //     }
        // }

        let layout = egui::Layout::right_to_left(egui::Align::RIGHT);

        ui.with_layout(layout, |ui| {

            let url_env = if ls.node_config.network.is_main() {
                "".to_string()
            } else {
                format!("{}.",ls.node_config.network.to_std_string())
            };
            // TODO: Format the address of some xpub.
            let env_formatted_faucet = format!("https://{}explorer.redgold.io/faucet", url_env, );
            ui.hyperlink_to("Faucet", env_formatted_faucet);
            ui.label(ls.wallet_state.faucet_success.clone());
            if ui.button("Refresh Balance").clicked() {
                refresh_balance(ls);
            };
        });
    });
}

fn refresh_balance(ls: &mut LocalState) {
    address_query::get_address_info(&ls.node_config.clone(),
                                    ls.wallet_state.public_key.clone().expect("pk"),
                                    ls.wallet_state.updates.sender.clone(),
    );
}
