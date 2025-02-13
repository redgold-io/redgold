use crate::components::currency_input::{currency_combo_box, supported_wallet_currencies, CurrencyInputBox};
use crate::components::tables::text_table_advanced;
use crate::components::transaction_table::{format_fractional_currency_amount, TransactionTable};
use crate::components::tx_progress::TransactionProgressFlow;
use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use eframe::egui::{Color32, ComboBox, RichText, TextStyle, Ui};
use itertools::Itertools;
use log::info;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::explorer::SwapStatus;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::party_events::OrderFulfillment;
use redgold_schema::party::search_events::PartyEventSearch;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey, SupportedCurrency};
use redgold_schema::trust::FloatRoundedConverti64;
use redgold_schema::util::dollar_formatter::format_dollar_amount_with_prefix_and_suffix;
use redgold_schema::util::times::ToTimeString;
use redgold_schema::ShortString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_schema::proto_serde::ProtoSerde;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString)]
pub enum SwapStage {
    StartPreparing,
    ShowAmountsPromptSigning,
    ViewSignedAllowBroadcast,
    CompleteShowTrackProgress
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString)]
pub enum SwapSubTab {
    New,
    History
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwapState {
    pub output_currency: SupportedCurrency,
    pub stage: SwapStage,
    pub tx_progress: TransactionProgressFlow,
    pub currency_input_box: CurrencyInputBox,
    pub changing_stages: bool,
    pub swap_valid: bool,
    pub invalid_reason: String,
    pub swap_subtab: SwapSubTab
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserSwapInfoRow {
    pub txid: String,
    pub input_currency: SupportedCurrency,
    pub output_currency: SupportedCurrency,
    pub input_amount: f64,
    pub input_amount_usd: f64,
    pub output_amount: f64,
    pub output_amount_usd: f64,
    pub time: i64,
    pub status: SwapStatus,
    pub party_id: PublicKey,
    pub fulfillment_txid: String
}

impl Default for SwapState {
    fn default() -> Self {
        Self {
            output_currency: SupportedCurrency::Redgold,
            stage: SwapStage::StartPreparing,
            tx_progress: Default::default(),
            currency_input_box: CurrencyInputBox::from_currency(SupportedCurrency::Ethereum, "Input".to_string()),
            changing_stages: false,
            swap_valid: false,
            invalid_reason: "".to_string(),
            swap_subtab: SwapSubTab::New,
        }
    }
}
impl SwapState {

    pub fn other_currency(&self) -> SupportedCurrency {
        if self.currency_input_box.input_currency == SupportedCurrency::Redgold {
            self.output_currency
        } else {
            self.currency_input_box.input_currency.clone()
        }
    }

    pub fn check_valid<E>(
        &mut self,
        data: &DataQueryInfo<E>,
        network: &NetworkEnvironment,
        pk: &PublicKey
    ) where E: ExternalNetworkResources + Send + Clone  + Sync{

        if self.currency_input_box.input_has_changed {
            let balances = data.balance_totals(&network, Some(pk));
            let cur = self.currency_input_box.input_currency;
            let bal = balances.get(&cur).cloned().unwrap_or(0.0);
            let input = self.currency_input_box.input_currency_amount(&data.price_map_usd_pair_incl_rdg).to_fractional();
            if input > bal {
                info!("Insufficient balance: balance: {} < input: {}: balances: {}, cur: {}", bal, input, balances.json_or(), cur.json_or()
                );
                self.invalid_reason = "Insufficient Balance".to_string();
                self.swap_valid = false;
                return
            }
            let _fp = {
                let r = data.first_party.lock().unwrap();
                r.clone().party_events
            };
            if let Some(_cpp) = data.central_price_pair(None, self.other_currency()) {
                // cpp.fulfill_taker_order()
            } else {
                self.invalid_reason = "Missing party network data".to_string();
                self.swap_valid = false;
                return
            }
            self.swap_valid = self.currency_input_box.input_amount_value() > 0.0;
        }

    }

    pub fn view<G, E>(
        &mut self,
        ui: &mut Ui,
        g: &mut G,
        pk: &PublicKey,
        allowed: &Vec<XPubLikeRequestType>,
        csi: &TransactionSignInfo,
        tsi: &TransactionSignInfo,
        data: &DataQueryInfo<E>
    ) -> bool where G: GuiDepends + Clone + Send + 'static + Sync,
            E: ExternalNetworkResources + Send + Clone + Sync {

        let mut create_swap_tx_bool = false;
        self.check_valid(data, &g.get_network(), pk);

        ui.horizontal(|ui| {
            ui.heading("Swap");
            Self::party_explorer_link(ui, &data, &g.get_network(), g);
            if self.currency_input_box.input_currency == SupportedCurrency::Redgold {
                let output = self.output_currency.clone();
                let cpp = data.central_price_pair(None, output);
                if let Some(c) = cpp {
                    ui.label("Pair Balance:");
                    let vol = c.pair_quote_volume.to_fractional();
                    let b = format!("{:.8} {} ", vol, output.abbreviated());
                    ui.label(b);
                    let usd_vol = c.pair_quote_price_estimate * vol;
                    ui.label(format_dollar_amount_with_prefix_and_suffix(usd_vol));
                }
            }
            for swap_subtab in SwapSubTab::iter() {
                if ui.button(format!("{:?}", swap_subtab)).clicked() {
                    self.swap_subtab = swap_subtab;
                }
            }
            if self.swap_subtab == SwapSubTab::History {
                if ui.button("Refresh").clicked() {
                    data.refresh_swap_history(pk);
                }
            }
        });

        if self.swap_subtab == SwapSubTab::New {
            let locked = self.tx_progress.locked();
            self.swap_details(ui, locked, data, g.get_network().clone());

            let ev = self.tx_progress.view(ui, g, tsi, csi, allowed);
            if ev.next_stage_create {
                create_swap_tx_bool = true;
            }

            ui.horizontal(|ui| {
                if !self.swap_valid {
                    ui.label(RichText::new("Invalid Swap: ").color(Color32::RED));
                    ui.label(RichText::new(self.invalid_reason.clone()).color(Color32::RED));
                } else {

                }
            });
        } else if self.swap_subtab == SwapSubTab::History {
            let addrs = g.to_all_address(&pk);
            let parties = data.party_data.lock().unwrap().clone();
            let config = g.get_config();
            let pending_external_events= config
                .local.clone().unwrap_or_default().internal_stored_data.clone()
                .unwrap_or_default()
                .pending_external_swaps.clone().unwrap_or_default();
            let swap_events = parties.iter().flat_map(|(pk, pid)|
                pid.party_events.as_ref().iter().flat_map(|pe|
                    pe.find_swaps_for_addresses(&addrs)
            ).collect_vec()).collect_vec();
            
            let mut events = vec![];
            let pm = data.price_map_usd_pair_incl_rdg.clone();
            for ev in pending_external_events {
                if swap_events.iter()
                    .find(|(of, s, _)| s.identifier() == ev.external_tx.tx_id)
                    .is_some() {
                    let mut config2 = config.clone();
                    if let Some(l) = config2.local.as_mut() {
                        if let Some(iss) = l.internal_stored_data.as_mut() {
                            if let Some(pes) = &mut iss.pending_external_swaps {
                                pes.retain(|p| p.external_tx.tx_id != ev.external_tx.tx_id);
                            }
                        }
                    }
                    g.set_config(&config2, false);
                    continue
                }
                let input_amount = ev.external_tx.currency_amount().to_fractional();
                let output = ev.expected_amount.to_fractional();
                let row = UserSwapInfoRow {
                    txid: ev.external_tx.tx_id.clone(),
                    input_currency: ev.external_tx.currency.clone(),
                    output_currency: ev.destination_currency.clone(),
                    input_amount: input_amount,
                    input_amount_usd: pm.get(&ev.external_tx.currency).map(|x| x * input_amount).unwrap_or(0.0),
                    output_amount: output,
                    output_amount_usd: pm.get(&ev.destination_currency).map(|x| x * output).unwrap_or(0.0),
                    time: ev.external_tx.timestamp.unwrap_or(0) as i64,
                    status: SwapStatus::Pending,
                    party_id: ev.party_id.clone(),
                    fulfillment_txid: "".to_string(),
                };
                events.push(row);
            }

            let converted = Self::translate_swap_events(swap_events, pm);
            events.extend(converted);

            let mut data = vec![];
            let mut headers = vec![
                "Txid", "From", "To", "Amount", "USD", "Filled", "USD", "Time", "Status", "Fill Txid"
            ].iter().map(|s| s.to_string()).collect_vec();
            data.push(headers.iter().map(|s| s.to_string()).collect());
            for r in events {
                data.push(vec![
                    r.txid.clone(),
                    format!("{:?}", r.input_currency),
                    format!("{:?}", r.output_currency),
                    format_fractional_currency_amount(r.input_amount),
                    format_dollar_amount_with_prefix_and_suffix(r.input_amount_usd),
                    format_fractional_currency_amount(r.output_amount),
                    format_dollar_amount_with_prefix_and_suffix(r.output_amount_usd),
                    r.time.to_time_string_shorter_no_seconds(),
                    format!("{:?}", r.status),
                    r.fulfillment_txid.clone(),
                ]);
            }

            let func = move |ui: &mut Ui, row: usize, col: usize, val: &String| {
                let mut column_idxs = vec![0, 9];
                if column_idxs.contains(&col) {
                    ui.hyperlink_to(val.first_four_last_four_ellipses().unwrap_or("err".to_string()),
                                    g.get_network().explorer_hash_link(val.clone()));
                    return true
                }
                false
            };
            text_table_advanced(ui, data, false, false, None, vec![], Some(func));

            // ui.label("History coming soon, please check party address for events");
        }

        create_swap_tx_bool
    }

    pub fn translate_swap_events(swap_events: Vec<(OrderFulfillment, AddressEvent, AddressEvent)>, pm: HashMap<SupportedCurrency, f64>) -> Vec<UserSwapInfoRow> {
        let mut rows = vec![];
        for (of, swap, fulfillment) in swap_events {
            let mut row = UserSwapInfoRow::default();
            row.output_amount = of.fulfilled_currency_amount().to_fractional();
            match swap {
                AddressEvent::External(e) => {
                    row.txid = e.tx_id.clone();
                    row.input_currency = e.currency.clone();
                    row.input_amount = e.currency_amount().to_fractional();
                    row.input_amount_usd = e.currency_amount().to_fractional() * pm.get(&e.currency)
                        .cloned().unwrap_or(0.0);
                    row.time = e.timestamp.unwrap_or(0) as i64;
                    row.status = SwapStatus::Complete;
                    row.output_currency = SupportedCurrency::Redgold;
                    match fulfillment {
                        AddressEvent::External(_) => {}
                        AddressEvent::Internal(e) => {
                            row.party_id = e.tx.first_input_proof_public_key().cloned().unwrap();
                            row.fulfillment_txid = e.tx.hash_hex();
                        }
                    }
                }
                AddressEvent::Internal(e) => {
                    row.txid = e.tx.hash_hex();
                    row.input_currency = SupportedCurrency::Redgold;
                    row.input_amount = e.tx.non_remainder_amount_rdg_typed().to_fractional();
                    row.input_amount_usd = e.tx.non_remainder_amount_rdg_typed().to_fractional() *
                        pm.get(&SupportedCurrency::Redgold).cloned().unwrap_or(0.0);
                    row.time = e.tx.time().cloned().unwrap_or(0);
                    row.status = SwapStatus::Complete;
                    match fulfillment {
                        AddressEvent::External(e) => {
                            row.output_currency = e.currency.clone();
                            row.fulfillment_txid = e.tx_id.clone();
                        }
                        AddressEvent::Internal(_) => {}
                    }
                }
            }
            row.output_amount_usd = pm.get(&row.output_currency).map(|x| x * row.output_amount).unwrap_or(0.0);
            rows.push(row);
        }
        rows
    }

    fn big_proceed_button(&mut self, ui: &mut Ui, next_stage: SwapStage, button_text: &str) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            let style = ui.style_mut();
            style.override_text_style = Some(TextStyle::Heading);
            changed = ui.button(button_text).clicked();
            if changed {
                self.stage = next_stage;
            }
        });
        changed
    }

    fn swap_details<E>(&mut self, ui: &mut Ui, locked: bool, data: &DataQueryInfo<E>, net: NetworkEnvironment) where E: ExternalNetworkResources + Send + Clone  + Sync{
        ui.separator();
        let price_map_incl = data.price_map_usd_pair_incl_rdg.clone();

        ui.horizontal(|ui| {
            // ui.label("Swap From: ");
            self.currency_input_box.locked = locked;
            self.currency_input_box.view(ui, &price_map_incl);
        });
        ui.horizontal(|ui| {
            // ui.label("Swap To: ");
            let input_changed = self.currency_input_box.currency_has_changed;
            // let input_changed = currency_selection_box(ui, &mut self.input_currency, "To", supported_wallet_currencies(), locked);
            if input_changed {
                if self.currency_input_box.input_currency == SupportedCurrency::Redgold {
                    self.output_currency = SupportedCurrency::Ethereum;
                } else {
                    self.output_currency = SupportedCurrency::Redgold;
                }
            }
            let currency = self.currency_input_box.input_currency;
            let filtered_swap_outputs = Self::filter_swap_output(&currency);
            currency_combo_box(ui, &mut self.output_currency, "Destination",
                               filtered_swap_outputs, locked);

            let use_usd = self.currency_input_box.use_usd_input.clone();

            let mut no_data = false;

            let is_ask = self.output_currency == SupportedCurrency::Redgold;
            let get_prices_of_currency = if is_ask {
                self.currency_input_box.input_currency.clone()
            } else {
                self.output_currency.clone()
            };

            if let Some(cp) = data.first_party
                .as_ref()
                .lock()
                .ok()
                .and_then(|p| p.party_events.clone())
                .and_then(|pe| pe.central_prices.get(&get_prices_of_currency).cloned()) {

                // ETH => RDG for example, get_prices = ETH
                // RDG => BTC for example, get_prices = BTC
                let pair_price_usd = price_map_incl.get(&get_prices_of_currency).map(|x| x.clone());

                let mut price_usd_est = cp.min_bid_estimated.clone();
                let mut price_usd_est_inverse = cp.min_bid_estimated.clone();
                if let Some(p) = pair_price_usd {
                    if is_ask {
                        price_usd_est = p.clone()
                    } else {
                        price_usd_est_inverse = p.clone()
                    }
                };


                let input_amount_value = self.currency_input_box.input_amount_value();
                let mut usd_value = if use_usd {
                    input_amount_value
                } else {
                    input_amount_value * price_usd_est
                };

                let mut pair_value = if use_usd {
                    input_amount_value / price_usd_est
                } else {
                    input_amount_value
                };

                let x = pair_value * 1e8;
                let oat = CurrencyAmount::from_fractional_cur(pair_value, self.currency_input_box.input_currency.clone()).unwrap();
                let fulfilled_amt = cp.dummy_fulfill(oat, x as u64, is_ask, &net, get_prices_of_currency);
                if fulfilled_amt == 0.0 {
                    self.swap_valid = false;
                    self.invalid_reason = "Order below minimum amount, or insufficient party liquidity".to_string();
                }
                let mut fulfilled_value_usd = fulfilled_amt * price_usd_est_inverse;
                let mut fulfilled_str = format!("{:?} fulfilled", self.output_currency);
                ui.label(fulfilled_str);
                let fulfilled_usd_str = format!("${:.2} USD", fulfilled_value_usd);
                ui.label(RichText::new(format!("{:.8}", fulfilled_amt)).color(Color32::GREEN));


                ui.label("Bid value:");
                ui.label(RichText::new(fulfilled_usd_str).color(Color32::RED));
            } else {
                ui.label("No price data available");
                no_data = true;
            }
        });
    }

    fn party_explorer_link<E,G>(
        ui: &mut Ui,
        data: &DataQueryInfo<E>,
        net: &NetworkEnvironment,
        g: &G
    ) where E: ExternalNetworkResources + Send + Clone + Sync, G: GuiDepends + Clone + Send + 'static + Sync {
        if let Some(pa) = data.first_party.as_ref()
            .lock()
            .ok()
            .map(|p| p.proposer_key.clone().hex())
        {
            ui.hyperlink_to("Party Explorer Link", net.explorer_hash_link(pa));
        }
    }

    fn filter_swap_output(currency: &SupportedCurrency) -> Vec<SupportedCurrency> {
        let remaining = supported_wallet_currencies()
            .iter().filter(|c| { c != &currency }).cloned().collect();
        if currency != &SupportedCurrency::Redgold {
            vec![SupportedCurrency::Redgold]
        } else {
            remaining
        }
    }
}

//
// #[ignore]
// #[tokio::test]
// async fn debug_fulfill() {
//     let nc = NodeConfig::default_env(NetworkEnvironment::Dev).await;
//     let pev = nc.api_rg_client().party_data().await.unwrap().into_values().next().unwrap().party_events.unwrap();
//     let cpp = pev.central_prices.get(&SupportedCurrency::Ethereum).unwrap();
//     let f = cpp.dummy_fulfill(16500000 as u64, false, &nc.network, SupportedCurrency::Ethereum);
//     println!("{}", f);
//     println!("{}", cpp.json_or());
// }