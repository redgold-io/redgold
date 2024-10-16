use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use eframe::egui::{TextStyle, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use log::info;
use strum::IntoEnumIterator;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::conf::local_stored_state::XPubRequestType;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ExternalTransactionId, PartySigningValidation, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::tx_builder::{TransactionBuilder};
use crate::common;
use crate::common::{big_button, data_item};
use crate::components::combo_box::combo_box;
use crate::components::currency_input::currency_combo_box;
use crate::dependencies::gui_depends::{GuiDepends, QrMessage, TransactionSignInfo};



// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString, Eq, Hash)]
// pub enum SigningMethod {
//     Hot,
//     Hardware,
//     QR
// }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionProgressFlow {
    pub stage: TransactionStage,
    pub unsigned_info_box: String,
    pub signed_info_box: String,
    pub unsigned_hash_txid: String,
    pub signed_hash_txid: String,
    pub broadcast_info: String,
    pub use_single_box: bool,
    pub prepared_tx: Option<PreparedTransaction>,
    pub stage_err: Option<String>,
    pub heading_details: HashMap<TransactionStage, String>,
    pub proceed_button_text: HashMap<TransactionStage, String>,
    pub changing_stages: bool,
    pub rdg_broadcast_response: Arc<Mutex<Option<RgResult<String>>>>,
    pub signing_method: XPubRequestType,
    // todo: change this to a transaction stage
    pub awaiting_broadcast: bool
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString, Eq, Hash)]
pub enum TransactionStage {
    NotCreated,
    Created,
    Signed,
    Broadcast
}

impl Default for TransactionProgressFlow {

    fn default() -> Self {
        TransactionProgressFlow {
            stage: TransactionStage::NotCreated,
            unsigned_info_box: "".to_string(),
            signed_info_box: "".to_string(),
            unsigned_hash_txid: "".to_string(),
            signed_hash_txid: "".to_string(),
            broadcast_info: "".to_string(),
            use_single_box: true,
            prepared_tx: None,
            stage_err: None,
            heading_details: Default::default(),
            proceed_button_text: Default::default(),
            changing_stages: false,
            rdg_broadcast_response: Arc::new(Mutex::new(None)),
            signing_method: XPubRequestType::Hot,
            awaiting_broadcast: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PreparedTransaction {
    pub currency: SupportedCurrency,
    pub from: PublicKey,
    pub to: Address,
    pub amount: CurrencyAmount,
    pub address_info: Option<AddressInfo>,
    pub tx: Option<Transaction>,
    pub btc_payloads: Option<(Vec<(Vec<u8>, String)>, PartySigningValidation)>,
    pub party_address: Option<Address>,
    pub party_fee: Option<CurrencyAmount>,
    pub eth_payload: Option<(Vec<u8>, PartySigningValidation, String)>,
    pub txid: Option<ExternalTransactionId>,
    pub internal_unsigned_hash: Option<String>,
    pub ser_tx: Option<String>,
    pub unsigned_hash: String,
    pub signed_hash: String,
    pub broadcast_response: String,
    pub tsi: TransactionSignInfo,
    pub signing_in_progress: bool,
    pub signing_method: XPubRequestType,
    pub qr_message: Option<QrMessage>
}


impl TransactionProgressFlow {


    pub fn rdg_only_prepared_tx(tx: Transaction) -> PreparedTransaction {
        let mut def = PreparedTransaction::default();
        def.tx = Some(tx.clone());
        def.ser_tx = Some(tx.json_or());
        def.internal_unsigned_hash = Some(tx.hash_hex());
        def
    }

    pub fn locked(&self) -> bool {
        self.stage != TransactionStage::NotCreated
    }
    pub async fn make_transaction<T: ExternalNetworkResources>(
        nc: &NodeConfig,
        mut external_resources: &mut T,
        currency: &SupportedCurrency,
        from: &PublicKey,
        to_address: &Address,
        amount: &CurrencyAmount,
        address_info: Option<&AddressInfo>,
        party_address: Option<&Address>,
        party_fee: Option<&CurrencyAmount>,
        from_eth_addr: Option<Address>,
        transaction_sign_info: &TransactionSignInfo
    ) -> RgResult<PreparedTransaction> {
        let mut prepped = PreparedTransaction::default();
        prepped.currency = currency.clone();
        prepped.from = from.clone();
        prepped.to = to_address.clone();
        prepped.amount = amount.clone();
        prepped.address_info = address_info.cloned();
        prepped.tsi = transaction_sign_info.clone();

        match currency {
            SupportedCurrency::Redgold => {
                let is_swap = party_address.is_some();
                let mut builder = TransactionBuilder::new(&nc);
                let mut tx_b = builder.with_utxos(&address_info.unwrap().utxo_entries)?;
                if is_swap {
                    let default = CurrencyAmount::from_rdg(100_000);
                    let fee = party_fee.unwrap_or(&default);
                    prepped.party_fee = Some(fee.clone());
                    let p_address = party_address.unwrap();
                    prepped.party_address = Some(p_address.clone());
                    tx_b = tx_b.with_swap(
                        to_address,
                        fee,
                        p_address
                    )?;
                } else {
                    tx_b = tx_b.with_output(&to_address, &amount);
                }
                let tx = tx_b.build()?;
                prepped.internal_unsigned_hash = Some(tx.signable_hash().hex());
                prepped.ser_tx = Some(tx.json_or());
                prepped.tx = Some(tx);
            }
            SupportedCurrency::Bitcoin => {
                let secret = transaction_sign_info.secret().ok_msg("No secret")?;
                let out = to_address.render_string()?;
                let payloads = external_resources.btc_payloads(vec![(out, amount.amount as u64)], from).await?;
                prepped.btc_payloads = Some(payloads);
                let (txid, tx_ser) = external_resources.send(
                    to_address, amount, false, Some(from.clone()), Some(secret)
                ).await?;
                prepped.txid = Some(txid);
                prepped.ser_tx = Some(tx_ser);
            }
            SupportedCurrency::Ethereum => {
                let secret = transaction_sign_info.secret().ok_msg("No secret")?;
                let f = from_eth_addr.ok_msg("Ethereum address required")?;
                let res = external_resources.eth_tx_payload(&f, to_address, amount).await?;
                prepped.eth_payload = Some(res);
                let (txid, tx_ser) = external_resources.send(to_address, amount, false,
                                                             Some(from.clone()), Some(secret)).await?;
                prepped.txid = Some(txid);
                prepped.ser_tx = Some(tx_ser);
            }
            _ => {}
        }
        let unsigned_hash = prepped.internal_unsigned_hash.clone().or(
            prepped.txid.clone().map(|x| x.identifier.clone())
        ).ok_msg("No txid")?;
        prepped.unsigned_hash = unsigned_hash;
        Ok(prepped)
    }

    pub fn created(&mut self, prepared_transaction: Option<PreparedTransaction>, err: Option<String>) {
        self.stage = TransactionStage::Created;
        if let Some(tx) = prepared_transaction {
            self.prepared_tx = Some(tx.clone());
            if let Some(tx_ser) = tx.ser_tx.clone() {
                self.unsigned_info_box = tx_ser;
            }
            self.unsigned_hash_txid = tx.internal_unsigned_hash.clone().or(
                tx.txid.clone().map(|x| x.identifier.clone()))
                .unwrap_or("".to_string());
        }
        self.stage_err = err;
    }

    pub fn signed(&mut self, prepared_transaction: Option<PreparedTransaction>, err: Option<String>) {
        self.stage = TransactionStage::Signed;
        if let Some(tx) = prepared_transaction {
            self.prepared_tx = Some(tx.clone());
            if let Some(tx_ser) = tx.ser_tx.clone() {
                self.signed_info_box = tx_ser;
            }
            self.signed_hash_txid = tx.signed_hash.clone()
        }
        self.stage_err = err;
    }

    pub fn broadcast(&mut self, prepared_transaction: Option<PreparedTransaction>, err: Option<String>) {
        self.stage = TransactionStage::Broadcast;
        if let Some(tx) = prepared_transaction {
            self.prepared_tx = Some(tx.clone());
            self.broadcast_info = tx.broadcast_response.clone();
        }
        self.stage_err = err;
    }

    pub fn reset(&mut self) {
        self.stage = TransactionStage::NotCreated;
        self.unsigned_info_box = "".to_string();
        self.signed_info_box = "".to_string();
        self.unsigned_hash_txid = "".to_string();
        self.signed_hash_txid = "".to_string();
        self.broadcast_info = "".to_string();
        self.prepared_tx = None;
        self.stage_err = None;
    }

    pub fn info_box_view(&mut self, ui: &mut Ui) {
        if self.stage != TransactionStage::NotCreated {
            let mut box_label = "";
            let mut box_text = "";
            let mut extra_label = "";
            let mut txid = "";
            match self.stage {
                TransactionStage::NotCreated => {}
                TransactionStage::Created => {
                    box_label = "Unsigned Transaction Details";
                    box_text = &self.unsigned_info_box;
                    extra_label = "Unsigned Transaction Hash";
                    txid = &self.unsigned_hash_txid;
                }
                TransactionStage::Signed => {
                    box_label = "Signed Transaction Details";
                    box_text = &self.signed_info_box;
                    extra_label = "Signed Transaction Hash";
                    txid = &self.signed_hash_txid;
                }
                TransactionStage::Broadcast => {
                    box_label = "Broadcast Transaction Details";
                    box_text = &self.broadcast_info;
                    extra_label = "Broadcast Transaction Hash";
                    txid = &self.signed_hash_txid;
                }
            }
            if self.use_single_box {
                ui.heading(box_label);
                let mut string1 = box_text.clone().to_string();
                if let Some(e) = self.stage_err.as_ref() {
                    string1 = e.clone();
                }
                common::bounded_text_area_size_id(ui, &mut string1, 800.0, 5, "tx_progress");
                ui.spacing();
            }
            ui.spacing();
            ui.separator();
            ui.horizontal(|ui| {
                if self.stage == TransactionStage::Created {
                    ui.label("Signing Method:");
                    combo_box(ui, &mut self.signing_method, "", XPubRequestType::iter().collect(), false, 100.0, Some("signing_method_box".to_string()));
                }
                // ui.label(extra_label);
                data_item(ui, "TXID:", txid.clone());
            });
        }
    }

    pub fn back(&mut self) {
        match self.stage {
            TransactionStage::Created => {
                self.stage = TransactionStage::NotCreated;
            }
            TransactionStage::Signed => {
                self.stage = TransactionStage::Created;
            }
            TransactionStage::Broadcast => {
                self.stage = TransactionStage::Signed;
            }
            _ => {}
        }
    }

    pub fn next_stage<G>(&mut self, g: &G, sign_info: &TransactionSignInfo) where G:  GuiDepends + Sized + Clone + Send + 'static {
        match self.stage {
            TransactionStage::NotCreated => {
                self.stage = TransactionStage::Created;
                self.awaiting_broadcast = false;
            }
            TransactionStage::Created => {
                let option = self.prepared_tx.as_mut().unwrap();
                option.tsi = sign_info.clone();
                option.signing_method = self.signing_method.clone();
                let (s, r) = flume::unbounded();
                let res = g.clone().sign_prepared_transaction(option, s);
                let res = r.recv().unwrap();
                if let Ok(res) = res {
                    let res = res.tx.unwrap();
                    let mut prepped = self.prepared_tx.clone().unwrap();
                    prepped.tx = Some(res.clone());
                    prepped.ser_tx = Some(res.json_or());
                    prepped.signed_hash = res.hash_hex();
                    self.signed(Some(prepped.clone()), None);
                } else if let Err(e) = res {
                    self.signed(None, Some(e.json_or()));
                }
                self.stage = TransactionStage::Signed;
            }
            TransactionStage::Signed => {

                let arc = self.rdg_broadcast_response.clone();
                let option = self.prepared_tx.as_mut().unwrap();
                let (s, r) = flume::unbounded();
                let res = g.clone().broadcast_prepared_transaction(option, s);
                self.awaiting_broadcast = true;

                let res = async move {
                    let s = r.recv().unwrap();
                    let mut guard = arc.lock().unwrap();
                    *guard = Some(s.map(|x| x.broadcast_response));
                };
                g.spawn(res);
            }
            _ => {}
        }
    }

    pub fn stage_proceed_next_text(&self) -> String {
        let ret = self.proceed_button_text.get(&self.stage)
            .map(|x| x.clone());
        let default = match self.stage {
            TransactionStage::NotCreated => { "Create Transaction" }
            TransactionStage::Created => { "Sign Transaction" }
            TransactionStage::Signed => { "Broadcast Transaction" }
            TransactionStage::Broadcast => { "Complete" }
        };
        ret.unwrap_or(default.to_string())
    }

    pub fn progress_buttons<G>(&mut self, ui: &mut Ui, g: &G, ksi: &TransactionSignInfo) -> TxProgressEvent
     where G:  GuiDepends + Sized + Clone + Send + 'static {

        let mut event = TxProgressEvent {
            reset: false,
            next_stage: false
        };
        if let Ok(mut r) = self.rdg_broadcast_response.lock() {
            if r.is_some() && self.stage == TransactionStage::Signed {
                let res = r.as_ref().unwrap();
                self.broadcast_info = res.json_or();
                self.stage = TransactionStage::Broadcast;
                *r = None;
            }
        }

        let header = self.heading_details.get(&self.stage)
            .map(|h| h.clone()).unwrap_or(format!("{:?}", self.stage));
        //
        // ui.heading(header);

        ui.horizontal(|ui| {
            if big_button(ui, "Reset") {
                event.reset = true;
                self.reset();
            };
            if self.stage != TransactionStage::NotCreated {
                if self.stage != TransactionStage::Broadcast {
                    let back = big_button(ui, "Back");
                    if back {
                        self.back();
                    }
                }
            }

            if self.stage != TransactionStage::Broadcast && self.stage_err.is_none() {
                let changed = big_button(ui, self.stage_proceed_next_text());
                if changed {
                    event.next_stage = true;
                    self.next_stage(g, ksi);
                }
            }
        });
        event
    }
}


#[derive(Clone, Debug)]
pub struct TxProgressEvent {
    pub reset: bool,
    pub next_stage: bool
}

