use crate::components::address_input_box::AddressInputBox;
use crate::components::currency_input::CurrencyInputBox;
use crate::components::passphrase_input::PassphraseInput;
use crate::components::tx_progress::{TransactionProgressFlow, TransactionStage};
use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use crate::tab::custom_tx::CustomTxState;
use crate::tab::receive::ReceiveData;
use crate::tab::stake::StakeState;
use crate::tab::transact::portfolio_transact::PortfolioState;
use crate::tab::transact::states::{DeviceListStatus, SendReceiveTabs, WalletTab};
use eframe::egui::Ui;
use itertools::Itertools;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::{Channel, RecvAsyncErrorInfo};
use redgold_schema::conf::local_stored_state::{AccountKeySource, XPubLikeRequestType};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::structs::{Address, AddressInfo, ErrorInfo, Hash, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::RgResult;

pub struct WalletState {
    pub last_query: Option<i64>,
    pub tab: WalletTab,
    pub device_list_status: DeviceListStatus,
    pub public_key: Option<PublicKey>,
    pub public_key_msg: Option<String>,
    // futs: Vec<impl Future>
    // pub(crate) updates: Channel<StateUpdate>,
    pub send_receive: SendReceiveTabs,
    pub destination_address: String,
    pub amount_input: String,
    pub faucet_success: String,
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
    where G: GuiDepends + Clone + Send + 'static + Sync,
          E: ExternalNetworkResources + Clone + Send + 'static + Sync{


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
                        let g2 = g.clone();
                        let tx = async move {
                            let g2 = g2.clone();
                            let result = TransactionProgressFlow::make_transaction(
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
                                &tsii,
                                &g2
                            ).await;
                            sender.send(result).await.ok();
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
    pub fn update_hot_mnemonic_or_key_info<G>(&mut self, g: &G) where G: GuiDepends + Clone + Send + 'static + Sync {
        self.mnemonic_or_key_checksum = self.checksum_key(g);
        if let Some((pkhex, public_key)) = self.active_hot_private_key_hex.as_ref()
            .and_then(|kp| g.private_hex_to_public_key(kp.clone()).ok().map(|kp2| (kp.clone(), kp2))) {
            self.public_key = Some(public_key);
            let hex = hex::decode(pkhex).unwrap_or(vec![]);
            let check = Hash::new_checksum(&hex);
            self.mnemonic_or_key_checksum = check;
        } else {
            let m = self.hot_mnemonic(g);
            let check = G::checksum_words(m.clone()).unwrap_or("".to_string());
            let pk = G::public_at(m.clone(), self.derivation_path.clone());
            self.hot_secret_key = G::private_at(m.clone(), self.derivation_path.clone()).ok();
            self.public_key = pk.ok();
            self.mnemonic_or_key_checksum = check;
            self.seed_checksum = G::seed_checksum(m).ok().clone();
        }
    }
}

impl WalletState {

    pub fn checksum_key<G>(&self, g: &G) -> String where G: GuiDepends + Clone + Send + 'static + Sync {
        if let Some(kp) = self.active_hot_private_key_hex.as_ref() {
            if let Some(b) = hex::decode(kp).ok() {
                return Hash::new_checksum(&b);
            }
        }

        G::checksum_words(self.hot_mnemonic(g)).unwrap_or("".to_string())
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

    pub fn hot_mnemonic<G>(&self, g: &G) -> WordsPass where G: GuiDepends + Clone + Send + 'static + Sync {
        let pass = if self.hot_passphrase.is_empty() {
            None
        } else {
            Some(self.hot_passphrase.clone())
        };
        let m = self.active_hot_mnemonic.as_ref().unwrap_or(&self.hot_mnemonic_default);
        let mut w = WordsPass::new(m, pass.clone());
        if !self.hot_offset.is_empty() {
            w = G::hash_derive_words(w, self.hot_offset.clone()).expect("err");
            w.passphrase = pass;
        }
        w
    }

    pub fn new(hot_mnemonic: String, option: Option<&AccountKeySource>) -> Self {
        Self {
            last_query: None,
            tab: WalletTab::Hardware,
            device_list_status: Default::default(),
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
            derivation_path: default_pubkey_path(),
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
            selected_xpub_name: option.map(|o| o.name.clone()).unwrap_or("Select Account".to_string()),
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
    pub fn update_hardware<G>(&mut self, g: &G) where G: GuiDepends + Clone + Send + 'static + Sync {
        if self.device_list_status.last_polled.elapsed().as_secs() > 5 {
            self.device_list_status = g.get_device_list_status();
        }
    }
}


pub const DEFAULT_ACCOUNT_NUM: u32 = 50;

pub fn default_pubkey_path() -> String {
    format!("m/44'/0'/{}'/0/0", DEFAULT_ACCOUNT_NUM)
}