use std::collections::HashMap;
use eframe::egui::{Color32, RichText, Ui};
use eframe::egui::style::Interaction;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::explorer::{brief_transaction, BriefTransaction};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::party_events::{ConfirmedExternalStakeEvent, InternalStakeEvent, PendingExternalStakeEvent};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{CurrencyAmount, PublicKey, SupportedCurrency, UtxoEntry};
use redgold_schema::util::lang_util::WithMaxLengthString;
use crate::components::address_input_box::AddressInputBox;
use crate::components::balance_table::balance_table;
use crate::components::combo_box::combo_box;
use crate::components::currency_input::{supported_wallet_currencies, CurrencyInputBox};
use crate::components::transaction_table::TransactionTable;
use crate::components::tx_progress::{TransactionProgressFlow, TransactionStage};
use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};

#[derive(EnumString, EnumIter, Clone, Debug, PartialEq)]
pub enum StakeMode {
    View,
    Deposit,
    Withdrawal,
    Complete,
}


#[derive(EnumString, EnumIter, Clone, Debug, PartialEq)]
pub enum DepositMode {
    AMM,
    Portfolio
}

impl Default for DepositMode {
    fn default() -> Self {
        Self::AMM
    }
}

#[derive(Clone)]
pub struct StakeState {
    pub currency_input: CurrencyInputBox,
    pub deposit: TransactionProgressFlow,
    pub withdrawal: TransactionProgressFlow,
    pub mode: StakeMode,
    pub deposit_mode: DepositMode,
    pub complete: TransactionProgressFlow,
    pub party_identifier: PublicKey,
    pub show_balances: bool,
    pub withdrawal_label: String,
    // is there a way to actually use this if it's not specified on the stake utxo?
    pub withdrawal_address: AddressInputBox,
    pub complete_label: String,
}


impl Default for StakeState {
    fn default() -> Self {
        Self {
            currency_input: CurrencyInputBox::default(),
            deposit: Default::default(),
            withdrawal: Default::default(),
            mode: StakeMode::View,
            deposit_mode: Default::default(),
            complete: Default::default(),
            party_identifier: Default::default(),
            show_balances: false,
            withdrawal_label: "".to_string(),
            withdrawal_address: Default::default(),
            complete_label: "".to_string(),
        }
    }
}

impl StakeState {
    pub fn view<E,G>(
        &mut self,
        ui: &mut Ui,
        d: &DataQueryInfo<E>,
        g: &G,
        pk: &PublicKey,
        tsi: &TransactionSignInfo,
        nc: &NodeConfig,
        allowed_signing_methods: &Vec<XPubLikeRequestType>,
        csi: &TransactionSignInfo,
    )
    where E: ExternalNetworkResources + Clone + Send + 'static,
          G: GuiDepends + Clone + Send + 'static {

        let addr = pk.address().unwrap();
        let addrs = g.to_all_address(pk);

        let keys = d.party_keys();
        if keys.is_empty() {
            ui.label(RichText::new("No party data found, network error").color(Color32::RED));
            return;
        }
        let ai = d.address_infos.lock().unwrap().get(pk).cloned();
        if ai.is_none() {
            ui.label(RichText::new("No address info found, network error or refresh required to get UTXOs").color(Color32::RED));
            return;
        }

        if !keys.contains(&self.party_identifier) {
            self.party_identifier = keys[0].clone();
        }
        let pev = d.party_events(Some(&self.party_identifier));

        ui.horizontal(|ui| {
            ui.heading("Stake");
            for mode in StakeMode::iter() {
                if ui.button(format!("{:?}", mode)).clicked() {
                    self.mode = mode;
                }
            }
            ui.checkbox(&mut self.show_balances, "Show Balances");
        });

        if let Some(pev) = pev {
            let balances = pev.staking_balances(&addrs, None, None);
            if self.show_balances {
                let mut hm2 = HashMap::default();
                for c in supported_wallet_currencies() {
                    let bal = balances.get(&c)
                        .map(|c| c.to_fractional()).unwrap_or(0.0);
                    hm2.insert(c, bal);
                }
                balance_table(ui, d, &nc, None, Some(pk), Some(hm2), Some("stake_balance".to_string()));
            }
            let internal_ev = pev.internal_staking_events.iter()
                .filter(|e| {
                    addrs.contains(&e.withdrawal_address)
                }).collect::<Vec<&InternalStakeEvent>>();

            let tx1 = internal_ev.iter().map(|e| {
                let mut b = brief_transaction(&e.tx, None).unwrap();
                b.amount = e.amount.to_fractional();
                b.currency = Some(SupportedCurrency::Redgold.json_or());
                b.clone()
            })
                .collect::<Vec<BriefTransaction>>();
            let external_ev = pev.external_staking_events.iter()
                .filter(|e| {
                    addrs.contains(&e.pending_event.external_address)
                }).collect::<Vec<&ConfirmedExternalStakeEvent>>();
            let tx2 = external_ev.iter().map(|e| {
                let mut b = brief_transaction(&e.pending_event.tx, None).unwrap();
                b.amount = e.ett.currency_amount().to_fractional();
                b.currency = Some(e.ett.currency.json_or());
                b.clone()
            }).collect::<Vec<BriefTransaction>>();
            let mut tx_table = TransactionTable::default();
            tx_table.stake_mode = true;
            let mut all = tx1.clone();
            all.extend(tx2.clone());
            all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            tx_table.rows = all.iter().take(5).cloned().collect::<Vec<BriefTransaction>>();
            match self.mode {
                StakeMode::View => {
                    tx_table.view(ui, g.get_network());
                }
                StakeMode::Deposit => {
                    ui.horizontal(|ui| {
                        combo_box(ui, &mut self.deposit_mode, "Deposit Mode", DepositMode::iter().collect::<Vec<DepositMode>>(), self.deposit.locked(), 100.0, None);
                        self.currency_input.view(ui, &d.price_map_usd_pair_incl_rdg);
                    });
                    self.deposit.info_box_view(ui, allowed_signing_methods);
                    let ev = self.deposit.progress_buttons(ui, g, tsi, csi);
                    if ev.next_stage {
                        if self.deposit.stage == TransactionStage::Created {
                            let mut b = g.tx_builder();
                            b.with_input_address(&addr);
                            b.with_address_info(ai.unwrap()).unwrap();
                            if self.currency_input.input_currency == SupportedCurrency::Redgold {
                                b = b.with_internal_stake_usd_bounds(
                                    None, None, &addr, &pev.key_address, &self.currency_input.input_currency_amount(&d.price_map_usd_pair_incl_rdg)
                                ).clone();
                            } else {
                                let amount = self.currency_input.input_currency_amount(&d.price_map_usd_pair_incl_rdg);
                                let cur = self.currency_input.input_currency;
                                let party_addr = pev.party_pk_all_address.iter().filter(|a| {
                                    a.as_external().currency_or() == cur
                                }).next().cloned().unwrap();
                                b = b.with_external_stake_usd_bounds(
                                    None, None, &addr, &party_addr, &amount, &pev.key_address, &CurrencyAmount::from_rdg(100_000)
                                ).clone();
                            }
                            // TODO: Capture TX builder errors here, ideally with RgResult on tx progress flow
                            let tx = b.build();
                            self.deposit.with_built_rdg_tx(tx);
                        }
                    }

                }
                StakeMode::Withdrawal => {

                    let mut labels = vec![];
                    let mut external_label_map: HashMap<String, ConfirmedExternalStakeEvent> = HashMap::default();

                    for e in external_ev {
                        let u = e.pending_event.utxo_id.hex().last_n(12);
                        let amt = format!("{:.8}", e.ett.currency_amount().to_fractional());
                        let cur = e.ett.currency.abbreviated();
                        let label = format!("{} {} {}", u, amt, cur);
                        labels.push(label.clone());
                        external_label_map.insert(label.clone(), e.clone());
                    }


                    let mut internal_label_map: HashMap<String, InternalStakeEvent> = HashMap::default();
                    for e in internal_ev {
                        let u = e.utxo_id.hex().last_n(12);
                        let amt = format!("{:.8}", e.amount.to_fractional());
                        let cur = SupportedCurrency::Redgold.abbreviated();
                        let label = format!("{} {} {}", u, amt, cur);
                        internal_label_map.insert(label.clone(), e.clone());
                        labels.push(label);
                    }
                    combo_box(ui, &mut self.withdrawal_label, "Withdrawal UTXO",
                              labels,
                              self.withdrawal.locked(),
                              400.0,
                              None
                    );

                    self.withdrawal.info_box_view(ui, allowed_signing_methods);
                    let ev = self.withdrawal.progress_buttons(ui, g, tsi, csi);
                    if ev.next_stage {
                        if self.withdrawal.stage == TransactionStage::Created {
                            let mut b = g.tx_builder();
                            b.with_input_address(&addr);
                            b.with_address_info(ai.unwrap()).unwrap();
                            let mut utxo_id = None;
                            let mut withdrawal_addr = None;
                            if let Some(e) = internal_label_map.get(&self.withdrawal_label) {
                                utxo_id = Some(e.utxo_id.clone());
                                withdrawal_addr = Some(addr.clone());
                            } else if let Some(e) = external_label_map.get(&self.withdrawal_label) {
                                utxo_id = Some(e.pending_event.utxo_id.clone());
                                withdrawal_addr = Some(addrs.iter().filter(|a| a.as_external().currency_or() == e.ett.currency).next().cloned().unwrap());
                            }
                            // TODO: err indication here.
                            if let (Some(utxo_id), Some(withdrawal_addr)) = (utxo_id, withdrawal_addr) {
                                b.with_stake_withdrawal(&withdrawal_addr, &pev.key_address, &CurrencyAmount::from_rdg(100_000), &utxo_id);
                                let tx = b.build();
                                self.withdrawal.with_built_rdg_tx(tx);
                            }
                        }
                    }
                }
                StakeMode::Complete => {
                    let mut labels = vec![];
                    let mut pending_label_map: HashMap<String, PendingExternalStakeEvent> = HashMap::default();
                    let pending = pev.pending_external_staking_txs.iter()
                        .filter(|e| {
                            addrs.contains(&e.external_address)
                        }).collect::<Vec<&PendingExternalStakeEvent>>();

                    for e in pending {
                        let u = e.utxo_id.hex().last_n(12);
                        let amt = format!("{:.8}", e.amount.to_fractional());
                        let cur = e.external_currency.abbreviated();
                        let label = format!("{} {} {}", u, amt, cur);
                        labels.push(label.clone());
                        pending_label_map.insert(label.clone(), e.clone());
                    }

                    combo_box(ui, &mut self.complete_label, "Complete External Stake",
                              labels,
                              self.complete.locked(),
                              400.0,
                              None
                    );

                    if let Some(event) = pending_label_map.get(&self.complete_label) {
                        self.complete.info_box_view(ui, allowed_signing_methods);
                        let ev = self.complete.progress_buttons(ui, g, tsi, csi);
                        if ev.next_stage {
                            if self.complete.stage == TransactionStage::Created {
                                let (s, r) = flume::bounded(1);
                                let mut e = d.external.clone();
                                let currency = event.external_currency;
                                let destination_address = pev.address_for_currency(&currency).unwrap();
                                let amount = event.amount.clone();
                                let option = {
                                    let guard = d.address_infos.lock().unwrap();
                                    let ai = guard.get(pk).cloned();
                                    ai
                                };
                                let pk_inner = pk.clone();
                                let option1 = g.form_eth_address(pk).ok();
                                let tsii = tsi.clone();
                                let ncc = nc.clone();
                                let sender = s.clone();
                                let tx = async move {
                                    sender.send(TransactionProgressFlow::make_transaction(
                                        &ncc,
                                        &mut e,
                                        &currency,
                                        &pk_inner,
                                        &destination_address,
                                        &amount,
                                        option.as_ref(),
                                        None,
                                        None,
                                        option1,
                                        &tsii
                                    ).await).unwrap();
                                };
                                let res = g.spawn(tx);
                                let res = r.recv().unwrap();
                                self.complete.created(res.clone().ok(), res.err().map(|e| e.json_or()));
                            }
                        }
                    }
                }
            }
        }

    }
}