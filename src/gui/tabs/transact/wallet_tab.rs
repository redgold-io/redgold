use std::future::Future;
use std::time::Instant;
use bdk::bitcoin::bech32::ToBase32;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, Context, RichText, ScrollArea, TextStyle, Ui, Widget};
use flume::Sender;
use itertools::{Either, Itertools};
use rocket::form::validate::Contains;
use serde::Deserialize;
use crate::gui::app_loop::{LocalState, LocalStateAddons};

use strum::IntoEnumIterator;
// 0.17.1
use strum_macros::{EnumIter, EnumString};
use tracing::{error, info};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::{new_channel, Channel, RecvAsyncErrorInfo, SendErrorInfo};
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
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_gui::common;
use redgold_gui::common::{bounded_text_area, data_item, data_item_multiline_fixed, editable_text_input_copy, medium_data_item, valid_label};
use redgold_gui::components::address_input_box::AddressInputBox;
use redgold_gui::components::balance_table::balance_table;
use redgold_gui::components::currency_input::CurrencyInputBox;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::util::lang_util::JsonCombineResult;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::conf::local_stored_state::{AccountKeySource, StoredMnemonic, StoredPrivateKey, XPubLikeRequestType};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_gui::components::passphrase_input::PassphraseInput;
use redgold_gui::components::transaction_table::TransactionTable;
use redgold_gui::components::tx_progress::{PreparedTransaction, TransactionProgressFlow, TransactionStage};
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::state::lss_addon::LssAddon;
use redgold_gui::tab::custom_tx::CustomTxState;
use redgold_gui::tab::receive::ReceiveData;
use redgold_gui::tab::stake::StakeState;
use redgold_gui::tab::transact::portfolio_transact::{PortfolioState, PortfolioTransactSubTab};
use redgold_gui::tab::transact::states::{DeviceListStatus, SendReceiveTabs, WalletTab};
use crate::gui::components::explorer_links::rdg_explorer;
use crate::gui::components::swap::SwapState;
use crate::gui::components::xpub_req;
use crate::gui::ls_ext::create_swap_tx;
use crate::gui::tabs::keys::keys_tab::internal_stored_xpubs;
use crate::gui::tabs::transact::{address_query, broadcast_tx, cold_wallet, hardware_signing, hot_wallet, portfolio_transact, prepare_tx, prepared_tx_view};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;


pub trait DeviceListTrezorNative {
    fn poll() -> Self;
}

impl DeviceListTrezorNative for DeviceListStatus {
    fn poll() -> Self {
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


pub struct WalletState {
    pub last_query: Option<i64>,
    pub tab: WalletTab,
    pub(crate) device_list_status: DeviceListStatus,
    pub(crate) public_key: Option<PublicKey>,
    pub(crate) public_key_msg: Option<String>,
    // futs: Vec<impl Future>
    // pub(crate) updates: Channel<StateUpdate>,
    pub send_receive: SendReceiveTabs,
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
    pub hot_secret_key: Option<String>,
    pub port: PortfolioState,
    pub view_additional_xpub_details: bool,
    pub show_xpub_balance_info: bool,
    pub send_currency_input: CurrencyInputBox,
    pub send_address_input_box: AddressInputBox,
    pub send_tx_progress: TransactionProgressFlow,
    pub address_labels: Vec<(String, Address)>,
    pub receive_data: Option<ReceiveData>,
    pub stake: StakeState,
    pub custom_tx: CustomTxState,
}

impl WalletState {


    pub fn send_view_top<G, E>(
        &mut self,
        ui: &mut Ui,
        pk: &PublicKey,
        g: &G,
        d: &DataQueryInfo<E>,
        labels: Vec<(String, Address)>,
        tsi: &TransactionSignInfo,
        nc: &NodeConfig,
        csi: &TransactionSignInfo,
        allowed: &Vec<XPubLikeRequestType>
    )
    where G: GuiDepends + Clone + Send + 'static,
          E: ExternalNetworkResources + Clone + Send + 'static {


        let labels = labels.into_iter().filter(|(l, v)|
            l.contains(self.send_currency_input.input_currency.abbreviated().as_str())
        ).collect_vec();
        //         .selected_text(format!("{:?}", ls.wallet.send_currency_type))
        //             let string = &mut ls.wallet.destination_address;
        self.send_currency_input.view(ui, &d.price_map_usd_pair_incl_rdg);
        self.send_currency_type = self.send_currency_input.input_currency.clone();
        self.send_address_input_box.view(ui, labels, g);
        ui.separator();

        if self.send_address_input_box.valid {
            let event = self.send_tx_progress.view(ui, g, tsi, csi, allowed);

            if event.reset {
                self.send_tx_progress.reset();
                self.send_address_input_box.reset();
                self.send_currency_input.reset();
            }
            if event.next_stage {
                match self.send_tx_progress.stage {
                    TransactionStage::Created => {
                        let mut e = d.external.clone();
                        let address = self.send_address_input_box.address.clone();
                        let currency = self.send_currency_input.input_currency.clone();
                        let amount = self.send_currency_input.input_currency_amount(&d.price_map_usd_pair_incl_rdg);
                        let option = {
                            let guard = d.address_infos.lock().unwrap();
                            let ai = guard.get(pk).cloned();
                            ai
                        };
                        let pk_inner = pk.clone();
                        let option1 = g.form_eth_address(pk).ok();
                        let tsii = tsi.clone();
                        let ncc = nc.clone();
                        let c = Channel::new();
                        let sender = c.clone();
                        let tx = async move {
                            sender.send(TransactionProgressFlow::make_transaction(
                                &ncc,
                                &mut e,
                                &currency,
                                &pk_inner,
                                &address,
                                &amount,
                                option.as_ref(),
                                None,
                                None,
                                option1,
                                &tsii
                            ).await).await.ok();
                        };
                        let res = g.spawn(tx);
                        let res = c.receiver.recv_err().unwrap();
                        self.send_tx_progress.created(res.clone().ok(), res.err().map(|e| e.json_or()));
                    }
                    _ => {}
                }
            }
        }

        // currency_selection_box(ui, labels);
        // ui.horizontal(|ui| {
        //     ui.label("To:");
        //     let string = &mut ls.wallet.destination_address;
        //     ui.add(egui::TextEdit::singleline(string).desired_width(460.0));
        //     common::copy_to_clipboard(ui, string.clone());
        //     let valid_addr = string.parse_address().is_ok();
        //     if valid_addr {
        //         ui.label(RichText::new("Valid").color(Color32::GREEN));
        //     } else {
        //         ui.label(RichText::new("Invalid").color(Color32::RED));
        //     }
        // });
        // // TODO: Amount USD and conversions etc.
        // ui.horizontal(|ui| {
        //     ui.label("Amount");
        //     let string = &mut ls.wallet.amount_input;
        //     ui.add(egui::TextEdit::singleline(string).desired_width(200.0));
        //     // ui.checkbox(&mut ls.wallet_state.mark_output_as_stake, "Mark as Stake");
        //     // ui.checkbox(&mut ls.wallet_state.mark_output_as_swap, "Mark as Swap");
        //     // if ls.wallet_state.mark_output_as_stake {
        //     //     ui.checkbox(&mut ls.wallet_state.mark_output_as_stake, "Mark as Stake");
        //     // }
        // });

    }
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
            self.hot_secret_key = m.private_at(self.derivation_path.clone()).ok();
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
        self.send_receive = Default::default();
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

    pub fn new(hot_mnemonic: String, option: Option<&AccountKeySource>) -> Self {
        Self {
            last_query: None,
            tab: WalletTab::Hardware,
            device_list_status: DeviceListStatus::poll(),
            public_key: None,
            public_key_msg: None,
            // updates: new_channel(),
            send_receive: Default::default(),
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
            hot_secret_key: None,
            port: Default::default(),
            view_additional_xpub_details: true,
            show_xpub_balance_info: true,
            send_currency_input: Default::default(),
            send_address_input_box: AddressInputBox::default(),
            send_tx_progress: Default::default(),
            address_labels: vec![],
            receive_data: None,
            stake: Default::default(),
            custom_tx: Default::default(),
        }
    }
    pub fn update_hardware(&mut self) {
        if self.device_list_status.last_polled.elapsed().as_secs() > 5 {
            self.device_list_status = DeviceListStatus::poll();
        }
    }
}

pub fn wallet_screen<G>(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState, has_changed_tab: bool, depends: &G, d: &DataQueryInfo<ExternalNetworkResourcesImpl>)
    where G: GuiDepends + Clone + Send + 'static {
    // match local_state.wallet.updates.recv_while() {
    //     Ok(updates) => {
    //         for mut update in updates {
    //             // info!("Received item of update, applying");
    //             (update.update)(local_state);
    //             // info!("New wallet state faucet message: {}", local_state.wallet_state.faucet_success.clone());
    //         }
    //     }
    //     Err(e) => { error!("Error receiving updates: {}", e.json_or()) }
    // }
    local_state.wallet.update_hardware();
    ui.style_mut().spacing.item_spacing.y = 2f32;

    ScrollArea::vertical().show(ui, |ui| wallet_screen_scrolled(ui, ctx, local_state, has_changed_tab, depends, d));
}


pub fn wallet_screen_scrolled<G>(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState, has_changed_tab: bool, g: &G, d: &DataQueryInfo<ExternalNetworkResourcesImpl>)
    where G: GuiDepends + Clone + Send + 'static {

    let (mut update, xpub) =
        internal_stored_xpubs(
            ls, ui, ctx, has_changed_tab, g, None, ls.wallet.public_key.clone(), true
        );
    let mut is_hot = false;
    if has_changed_tab {
        update = true;
    }

    if let Some(x) = xpub {

        let mut passphrase_changed = false;

        if x.request_type != Some(XPubLikeRequestType::Hot) {
            cold_wallet::hardware_connected(ui, &mut ls.wallet);
        } else {
            is_hot = true;
            if ls.wallet.view_additional_xpub_details {
                passphrase_changed = ls.wallet.passphrase_input.view(ui);
            }
        }

        if update || passphrase_changed {
            // Are either of these even used?
            // ls.wallet_state.active_xpub = x.xpub.clone();
            // ls.wallet_state.active_derivation_path = ls.keytab_state.derivation_path_xpub_input_account.derivation_path();
            if let Ok(pk) = ls.keytab_state.xpub_key_info.public_key() {
                ls.wallet.public_key = Some(pk.clone());
                if is_hot {
                    if check_assign_hot_key(ls, &x, &pk).is_err() {
                        // Err, refuse to proceed. Show error message to user
                        ls.wallet.public_key = None;
                    } else {

                    }
                }
            } else {
                ls.wallet.public_key = None;
            }
        }
    }

    if let Some(pk) = ls.wallet.public_key.clone() {
        ls.wallet.passphrase_input.err_msg = None;
        if update || (ls.wallet.address_info.is_none() && has_changed_tab) {
            ls.wallet.address_info = None;
            refresh_balance(ls, g);
            ls.wallet.address_labels = ls.local_stored_state.address_labels(g);
            ls.wallet.receive_data = Some(ReceiveData::from_public_key(&pk, g));
        }
        if update {
            // let kp = ls.wallet_state.hot_mnemonic().keypair_at(ls.wallet_state.derivation_path.clone()).expect("kp");


        }
        let allowed = if !is_hot {
            vec![XPubLikeRequestType::Cold, XPubLikeRequestType::File, XPubLikeRequestType::QR]
        } else {
            vec![XPubLikeRequestType::Hot]
        };

        let mut hot_tsi = ls.hot_transaction_sign_info(g);
        proceed_from_pk(ui, ls, &pk, is_hot, g, d, &allowed, &hot_tsi, &ls.cold_transaction_sign_info(g));
    }

}

fn check_assign_hot_key(ls: &mut LocalState, x: &AccountKeySource, pk: &PublicKey) -> RgResult<()> {
    let key_name = x.key_name_source.as_ref().ok_msg("key_name")?;
    let k = ls.local_stored_state.by_key(&key_name).ok_msg("key")?;

    match k {
        Either::Left(m) => {
            let pass = ls.wallet.passphrase_input.passphrase.clone();
            let words = m.mnemonic.clone();
            let w = WordsPass::new(
                &words,
                Some(pass.clone())
            );
            w.validate()?;
            if !ls.keytab_state.derivation_path_xpub_input_account.valid {
                return Ok(())
            }
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
                ls.wallet.passphrase_input.err_msg = Some("Public key mismatch".to_string());
                return Err(error_info("Public key mismatch".to_string()));
            }
            ls.wallet.active_hot_mnemonic = Some(words);
            ls.wallet.hot_passphrase = pass;
            ls.wallet.derivation_path = dp.clone();
            ls.wallet.derivation_path_valid = true;
            ls.wallet.active_derivation_path = dp.clone();
            ls.wallet.xpub_derivation_path = dp.clone();
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
        ls.wallet.update_hot_mnemonic_or_key_info();
    };
    update_clicked
}

fn proceed_from_pk<G, E>(
    ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey, is_hot: bool, g: &G, d: &DataQueryInfo<E>,
    allowed: &Vec<XPubLikeRequestType>, hot_tsi: &TransactionSignInfo, csi: &TransactionSignInfo
)
    where G: GuiDepends + Clone + Send + 'static,
    E: ExternalNetworkResources + Clone + Send + 'static {

    // TODO: Include bitcoin address / ETH address for path 0 here for verification.
    ui.separator();

    if ls.wallet.show_xpub_balance_info {
        balance_table(ui, &ls.data, &ls.node_config, None, Some(pk), None, Some("wallet_balance".to_string()));
    }


    send_receive_bar(ui, ls, pk, g);

    ui.separator();
    ui.spacing();

    let mut show_prepared = false;
    match ls.wallet.send_receive {
        SendReceiveTabs::Send => {
            show_prepared = true;
            ls.wallet.send_view_top(
                ui, pk, g, d,
                ls.wallet.address_labels.clone(),
                hot_tsi,
                &ls.node_config,
                csi,
                allowed
            );
        }
        SendReceiveTabs::Receive => {
            if let Some(rd) = ls.wallet.receive_data.as_ref() {
                rd.view(ui);
            }
        }
        SendReceiveTabs::Custom => {
            ls.wallet.custom_tx.view::<E, G>(ui, g, hot_tsi, csi, allowed);
        }
        SendReceiveTabs::Swap => {
            if ls.swap_state.view(ui, g, pk, allowed, csi, hot_tsi, d) {
                // TODO: refactor this out
                create_swap_tx(ls);
            }
        }
        SendReceiveTabs::Home => {
            let rows = d.recent_tx(Some(pk), None, false, None);
            let mut tx_table = TransactionTable::default();
            tx_table.rows = rows;
            tx_table.full_view::<E>(ui, &g.get_network(), d, Some(pk));
            ui.separator();
        }
        SendReceiveTabs::Portfolio => {
            ls.wallet.port.view(ui, pk, g, hot_tsi, &ls.node_config, d, csi, allowed);
        }
        SendReceiveTabs::Stake => {
            ls.wallet.stake.view(ui, d, g, pk, hot_tsi, &ls.node_config, allowed, csi);
        }

    }
    if show_prepared {
        // prepared_tx_view::prepared_view(ui, ls, pk, is_hot);
    }

}


fn currency_selection_box(ui: &mut Ui, ls: &mut LocalState) {
    ComboBox::from_label("Currency")
        .selected_text(format!("{:?}", ls.wallet.send_currency_type))
        .show_ui(ui, |ui| {
            let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold];
            for style in styles {
                ui.selectable_value(&mut ls.wallet.send_currency_type, style.clone(), format!("{:?}", style));
            }
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

fn send_receive_bar<G>(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey, g: &G) where G: GuiDepends + Clone + Send + 'static  {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Heading);

        for t in SendReceiveTabs::iter() {
            if ui.button(format!("{:?}", t)).clicked() {
                ls.wallet.send_receive = t.clone();
            }
        }
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
            ui.label(ls.wallet.faucet_success.clone());
            if ui.button("Refresh Balance").clicked() {
                refresh_balance(ls, g);
            };
        });
    });
}

fn refresh_balance<G>(ls: &mut LocalState, g: &G) where G: GuiDepends + Clone + Send + 'static {
    let pk = ls.wallet.public_key.clone().expect("pk");
    ls.data.refresh_all_pk(&pk, g);
    address_query::get_address_info(
        &ls.node_config.clone(), pk, ls.local_messages.sender.clone(), g
    );
}
