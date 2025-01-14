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
use redgold_schema::airgap::{AirgapMessage, AirgapResponse};
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, ExternalTransactionId, PartySigningValidation, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::tx_builder::{TransactionBuilder};
use redgold_schema::util::lang_util::{JsonCombineResult, SameResult};
use crate::airgap::signer_window::{AirgapSignerWindow, AirgapWindowMode};
use crate::common;
use crate::common::{big_button, data_item, editable_text_input_copy};
use crate::components::combo_box::combo_box;
use crate::components::currency_input::currency_combo_box;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};



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
    pub signing_method: XPubLikeRequestType,
    // todo: change this to a transaction stage
    pub awaiting_broadcast: bool,
    pub file_input: String,
    #[serde(skip)]
    pub receiver: Option<flume::Receiver<RgResult<PreparedTransaction>>>
}

impl TransactionProgressFlow {
    pub fn with_config(&mut self, node_config: &NodeConfig) -> &mut Self {
        self.file_input = node_config.usb_paths_exist().get(0).cloned().unwrap_or("".to_string());
        self
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString, Eq, Hash)]
pub enum TransactionStage {
    NotCreated,
    Created,
    AwaitingSignatures,
    Signed,
    AwaitingBroadcastResponse,
    BroadcastComplete
}

impl TransactionStage {
    pub fn box_label(&self) -> String {
        let res = match self {
            TransactionStage::NotCreated => "",
            TransactionStage::Created => "Unsigned Transaction Details",
            TransactionStage::AwaitingSignatures => "Awaiting Signatures",
            TransactionStage::Signed => "Signed Transaction Details",
            TransactionStage::AwaitingBroadcastResponse => "Awaiting Broadcast Response",
            TransactionStage::BroadcastComplete => "Broadcast Transaction Response",
        };
        res.to_string()
    }

    pub fn allows_next_button(&self) -> bool {
        match self {
            TransactionStage::NotCreated => true,
            TransactionStage::Created => true,
            TransactionStage::AwaitingSignatures => false,
            TransactionStage::Signed => true,
            TransactionStage::AwaitingBroadcastResponse => false,
            TransactionStage::BroadcastComplete => false,
        }
    }
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
            signing_method: XPubLikeRequestType::Hot,
            awaiting_broadcast: false,
            file_input: "".to_string(),
            receiver: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
    pub signing_method: XPubLikeRequestType,
    pub error: Option<ErrorInfo>,
    pub file_input: String,
    pub airgap_signer_window: AirgapSignerWindow,
}

impl PreparedTransaction {
    pub fn apply_airgap_response(&mut self, msg: AirgapResponse) {
        if let Some(t) = self.tx.as_mut() {
            if let Some(s) = msg.sign_internal.as_ref() {
                if let Some(zero) = s.signed_txs.get(0) {
                    for sig in zero.signatures.iter() {
                        if let Some(i) = t.inputs.get_mut(sig.index as usize) {
                            i.proof = sig.proof.clone();
                        }
                    }
                }
            }
        }
    }
}


impl TransactionProgressFlow {


    pub fn rdg_only_prepared_tx(tx: Transaction) -> PreparedTransaction {
        let mut def = PreparedTransaction::default();
        def.tx = Some(tx.clone());
        def.ser_tx = Some(tx.json_or());
        def.internal_unsigned_hash = Some(tx.hash_hex());
        def
    }

    pub fn with_built_rdg_tx(&mut self, tx: RgResult<Transaction>) {
        match tx {
            Ok(tx) => {
                self.created(Some(Self::rdg_only_prepared_tx(tx)), None);
            },
            Err(e) => {
                self.created(None, Some(e.json_or()));
            }
        };
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
                info!("Builder fee addrs: {}", builder.fee_addrs.json_or());
                let mut tx_b = builder.with_utxos(&address_info.unwrap().utxo_entries)?;
                if is_swap {
                    let default = CurrencyAmount::from_rdg(100_000);
                    let fee = party_fee.unwrap_or(&default);
                    prepped.party_fee = Some(fee.clone());
                    let p_address = party_address.unwrap();
                    prepped.party_address = Some(p_address.clone());
                    // This is always a RDG -> pair swap, so the fee isn't really used here
                    // altho the main function needs to be updated to allow for a fee
                    // its more useful in the stake / external stuff.
                    tx_b = tx_b.with_swap(
                        to_address,
                        amount,
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
                let res = external_resources.eth_tx_payload(&f, to_address, amount, None).await?;
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

    pub fn broadcast_done(&mut self, prepared_transaction: Option<PreparedTransaction>, err: Option<String>) {
        self.stage = TransactionStage::BroadcastComplete;
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

    pub fn info_box_view_inner(&mut self, ui: &mut Ui, allowed_signing_methods: &Vec<XPubLikeRequestType>) {

        if self.stage != TransactionStage::NotCreated {
            let mut box_label = self.stage.box_label();
            let mut box_text = "";
            let mut txid = "";
            match self.stage {
                TransactionStage::NotCreated => {}
                TransactionStage::Created => {
                    box_text = &self.unsigned_info_box;
                    txid = &self.unsigned_hash_txid;
                }
                TransactionStage::Signed => {
                    box_text = &self.signed_info_box;
                    txid = &self.signed_hash_txid;
                }
                TransactionStage::BroadcastComplete => {
                    box_text = &self.broadcast_info;
                    txid = &self.signed_hash_txid;
                }
                TransactionStage::AwaitingSignatures => {
                    box_text = &self.unsigned_info_box;
                    txid = &self.unsigned_hash_txid;
                }
                TransactionStage::AwaitingBroadcastResponse => {
                    box_text = &self.signed_info_box;
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
                    combo_box(ui, &mut self.signing_method, "", allowed_signing_methods.clone(), false, 100.0, Some("signing_method_box".to_string()));
                    if self.signing_method == XPubLikeRequestType::File {
                        ui.label("File:");
                        editable_text_input_copy(ui, "", &mut self.file_input, 200.0);
                    };
                };
                if self.stage == TransactionStage::AwaitingSignatures {
                    if self.signing_method == XPubLikeRequestType::QR {
                        ui.checkbox(&mut self.prepared_tx.as_mut().unwrap().airgap_signer_window.visible, "Show QR Window");
                    }
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
            TransactionStage::BroadcastComplete => {
                self.stage = TransactionStage::Signed;
            }
            TransactionStage::NotCreated => {}
            TransactionStage::AwaitingSignatures => {
                self.stage = TransactionStage::Created;
            }
            TransactionStage::AwaitingBroadcastResponse => {
                self.stage = TransactionStage::Signed;
            }
        }
    }

    pub fn next_stage<G>(&mut self, g: &G, hot_info: &TransactionSignInfo, cold_info: &TransactionSignInfo) where G:  GuiDepends + Sized + Clone + Send + 'static {
        match self.stage {
            TransactionStage::NotCreated => {
                self.stage = TransactionStage::Created;
                self.awaiting_broadcast = false;
                self.rdg_broadcast_response = Arc::new(Mutex::new(None));
            }
            TransactionStage::Created => {
                let signing_info = match self.signing_method {
                    XPubLikeRequestType::Hot => { hot_info }
                    _ => {cold_info}
                };
                let option = self.prepared_tx.as_mut().unwrap();
                option.tsi = signing_info.clone();
                option.signing_method = self.signing_method.clone();
                option.file_input = self.file_input.clone();

                let (s, r) = flume::unbounded();
                let res = g.clone().sign_prepared_transaction(option, s);
                self.receiver = Some(r);
                self.stage = TransactionStage::AwaitingSignatures;
                self.rdg_broadcast_response = Arc::new(Mutex::new(None));
            }
            TransactionStage::Signed => {

                let arc = self.rdg_broadcast_response.clone();
                let option = self.prepared_tx.as_mut().unwrap();
                let (s, r) = flume::unbounded();
                let res = g.clone().broadcast_prepared_transaction(option, s);
                self.awaiting_broadcast = true;
                self.stage = TransactionStage::AwaitingBroadcastResponse;

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
            TransactionStage::BroadcastComplete => { "Complete" }
            _ => {""}
        };
        ret.unwrap_or(default.to_string())
    }


    pub fn view<G>(&mut self, ui: &mut Ui, g: &G, ksi: &TransactionSignInfo, cold_info: &TransactionSignInfo, allowed: &Vec<XPubLikeRequestType>) -> crate::components::tx_progress::TxProgressEvent
    where G:  GuiDepends + Sized + Clone + Send + 'static
    {

        if !allowed.contains(&self.signing_method) {
            self.signing_method = allowed[0].clone();
        }

        if self.stage == TransactionStage::AwaitingSignatures {
            if self.signing_method == XPubLikeRequestType::QR || self.signing_method == XPubLikeRequestType::File {
                let mut applied = false;
                if let Some(p) = self.prepared_tx.as_mut() {
                    p.airgap_signer_window.window_view(ui, g);
                    if p.airgap_signer_window.mode == AirgapWindowMode::CompletedDataReceipt {
                        if let Some(msg) = p.airgap_signer_window.message_response.as_ref() {
                            p.apply_airgap_response(msg.clone());
                            applied = true;
                        } else {
                            self.unsigned_info_box = "Failed to decode message response".to_string();
                        }
                    }
                }
                if applied {
                    self.signed(self.prepared_tx.clone(), None);
                }
            }
            if let Some(r) = self.receiver.as_ref() {
                if let Ok(res) = r.try_recv() {
                    if let Ok(res) = res {
                        if self.signing_method == XPubLikeRequestType::Hot || self.signing_method == XPubLikeRequestType::Cold {
                            self.signed(Some(res.clone()), None);
                        } else {
                            if self.signing_method == XPubLikeRequestType::QR || self.signing_method == XPubLikeRequestType::File {
                                self.prepared_tx = Some(res.clone());
                            }
                        }
                    } else if let Err(e) = res {
                        self.signed(None, Some(e.json_or()));
                    }
                }
            }
        }
        // todo: receiver here.
        if self.stage == TransactionStage::AwaitingBroadcastResponse {
            if self.awaiting_broadcast {
                if let Ok(mut r) = self.rdg_broadcast_response.lock() {
                    if r.is_some() {
                        let res = r.as_ref().unwrap();
                        self.broadcast_info = res.as_ref().map_err(|j| j.json_or()).cloned().combine();
                        self.stage = TransactionStage::BroadcastComplete;
                        self.awaiting_broadcast = false;
                        *r = None;
                    }
                }
            }
        }

        self.info_box_view_inner(ui, allowed);
        self.progress_buttons_inner(ui, g, ksi, cold_info)
    }

    pub fn progress_buttons_inner<G>(&mut self, ui: &mut Ui, g: &G, ksi: &TransactionSignInfo, cold_info: &TransactionSignInfo) -> TxProgressEvent
    where G:  GuiDepends + Sized + Clone + Send + 'static {

        let mut event = TxProgressEvent {
            reset: false,
            next_stage: false,
            next_stage_transition_from: None,
            next_stage_create: false
        };
        if let Ok(mut r) = self.rdg_broadcast_response.lock() {
            if r.is_some() && self.stage == TransactionStage::Signed {
                event.next_stage_transition_from = Some(self.stage.clone());
                let res = r.as_ref().unwrap().clone();
                self.broadcast_info = res.map_err(|e| e.json_or()).combine();
                self.stage = TransactionStage::BroadcastComplete;
                *r = None;
            }
        }

        let header = self.heading_details.get(&self.stage)
            .map(|h| h.clone()).unwrap_or(format!("{:?}", self.stage));
        //
        // ui.heading(header);

        ui.label(format!("Stage: {:?}", self.stage));
        // ui.label(format!("awaiting broadcast {:?}", self.awaiting_broadcast));

        ui.horizontal(|ui| {
            if big_button(ui, "Reset") {
                event.reset = true;
                self.reset();
            };
            if self.stage != TransactionStage::NotCreated {
                if self.stage != TransactionStage::BroadcastComplete {
                    let back = big_button(ui, "Back");
                    if back {
                        self.back();
                    }
                }
            }

            if self.stage != TransactionStage::BroadcastComplete &&
                self.stage_err.is_none() &&
                self.stage != TransactionStage::AwaitingBroadcastResponse {
                let changed = big_button(ui, self.stage_proceed_next_text());
                if changed {
                    if self.stage == TransactionStage::NotCreated {
                        event.next_stage_create = true;
                    };
                    event.next_stage_transition_from = Some(self.stage.clone());
                    event.next_stage = true;
                    self.next_stage(g, ksi, cold_info);
                }
            }
        });
        event
    }
}


#[derive(Clone, Debug)]
pub struct TxProgressEvent {
    pub reset: bool,
    pub next_stage: bool,
    pub next_stage_transition_from: Option<TransactionStage>,
    pub next_stage_create: bool,

}

