use std::collections::HashMap;
use bdk::Utxo::Local;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, RichText, TextStyle, Ui};
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use tracing_subscriber::fmt::format;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey, SupportedCurrency};
use crate::gui::app_loop::{LocalState, LocalStateAddons};
use redgold_gui::common;
use redgold_gui::components::currency_input::{currency_combo_box, supported_wallet_currencies, CurrencyInputBox};
use redgold_gui::components::tx_progress::TransactionProgressFlow;
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_keys::address_external::ToBitcoinAddress;
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::util::dollar_formatter::{format_dollar_amount, format_dollar_amount_with_prefix, format_dollar_amount_with_prefix_and_suffix};
use crate::gui::ls_ext::{create_swap_tx};
use crate::gui::tabs::transact::wallet_tab::SendReceiveTabs;
use crate::gui::tabs::transact::wallet_tab::SendReceiveTabs::Swap;
use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString)]
pub enum SwapStage {
    StartPreparing,
    ShowAmountsPromptSigning,
    ViewSignedAllowBroadcast,
    CompleteShowTrackProgress
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwapState {
    pub active: bool,
    pub output_currency: SupportedCurrency,
    pub stage: SwapStage,
    pub tx_progress: TransactionProgressFlow,
    pub currency_input_box: CurrencyInputBox,
    pub changing_stages: bool,
    pub swap_valid: bool,
    pub invalid_reason: String,
}

impl Default for SwapState {
    fn default() -> Self {
        Self {
            active: false,
            output_currency: SupportedCurrency::Redgold,
            stage: SwapStage::StartPreparing,
            tx_progress: Default::default(),
            currency_input_box: CurrencyInputBox::from_currency(SupportedCurrency::Ethereum, "Input".to_string()),
            changing_stages: false,
            swap_valid: false,
            invalid_reason: "".to_string(),
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
    ) where E: ExternalNetworkResources + Send + Clone {

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
            let fp = {
                let r = data.first_party.lock().unwrap();
                r.clone().party_events
            };
            if let Some(cpp) = data.central_price_pair(None, self.other_currency()) {
                // cpp.fulfill_taker_order()
            } else {
                self.invalid_reason = "Missing party network data".to_string();
                self.swap_valid = false;
                return
            }

            self.swap_valid = self.currency_input_box.input_amount_value() > 0.0;
        }

    }

    pub fn view<G>(
        ui: &mut Ui,
        ls: &mut LocalState,
        depends: &G,
        pk: &PublicKey,
        allowed: &Vec<XPubLikeRequestType>,
        csi: &TransactionSignInfo,
        tsi: &TransactionSignInfo
    ) where G: GuiDepends + Clone + Send + 'static {
        ls.swap_state.active = ls.wallet.send_receive == SendReceiveTabs::Swap;
        // if ui.button("Refresh Rates").clicked() {
        //     ls.price_map_usd_pair
        // }
        if ls.swap_state.active {

            ls.swap_state.check_valid(&ls.data, &depends.get_network(), pk);

            ui.horizontal(|ui| {
                ui.heading("Swap");
                Self::party_explorer_link(ui, &ls);
                if ls.swap_state.currency_input_box.input_currency == SupportedCurrency::Redgold {
                    let output = ls.swap_state.output_currency.clone();
                    let cpp = ls.data.central_price_pair(None, output);
                    if let Some(c) = cpp {
                        ui.label("Pair Balance:");
                        let vol = c.pair_quote_volume.to_fractional();
                        let b = format!("{:.8} {} ", vol, output.abbreviated());
                        ui.label(b);
                        let usd_vol = c.pair_quote_price_estimate * vol;
                        ui.label(format_dollar_amount_with_prefix_and_suffix(usd_vol));
                    }
                }
            });
            // ui.separator();
            let mut next_stage = SwapStage::ShowAmountsPromptSigning;;
            let mut previous_stage = SwapStage::StartPreparing;;
            let mut button_text = "Start Swap";

            match ls.swap_state.stage {
                SwapStage::StartPreparing => {
                    Self::swap_details(ui, ls, false);
                    // ui.heading("Lock Swap Details");
                    next_stage = SwapStage::ShowAmountsPromptSigning;
                    button_text = "Start Swap";
                }
                SwapStage::ShowAmountsPromptSigning => {
                    Self::swap_details(ui, ls, true);
                    // ui.heading("Swap Values Locked: Prepare to Sign");
                    next_stage = SwapStage::ViewSignedAllowBroadcast;
                    button_text = "Sign Transaction";
                }
                SwapStage::ViewSignedAllowBroadcast => {
                    Self::swap_details(ui, ls, true);
                    // ui.heading("Swap Signed: Prepare to Broadcast");
                    previous_stage = SwapStage::ShowAmountsPromptSigning;
                    next_stage = SwapStage::CompleteShowTrackProgress;
                    button_text = "Broadcast Transaction";
                }
                SwapStage::CompleteShowTrackProgress => {
                    Self::swap_details(ui, ls, true);
                    previous_stage = SwapStage::StartPreparing;
                    // ui.heading("Swap Complete");
                }
            }

            let ev = ls.swap_state.tx_progress.view(ui,depends, tsi, csi, allowed);
            if ev.next_stage_create {
                create_swap_tx(ls);
            }

            ui.horizontal(|ui| {
                // ui.horizontal(|ui| {
                //     let style = ui.style_mut();
                //     style.override_text_style = Some(TextStyle::Heading);
                //     if ui.button("Reset").clicked() {
                //         ls.swap_state.tx_progress.reset();
                //         ls.swap_state.currency_input_box.input_box_str = "".to_string();
                //         ls.swap_state.stage = SwapStage::StartPreparing;
                //     }
                // });
                // if ls.swap_state.stage != SwapStage::StartPreparing {
                //     ui.horizontal(|ui| {
                //         let style = ui.style_mut();
                //         style.override_text_style = Some(TextStyle::Heading);
                //         let mut text = "Back";
                //         if ls.swap_state.stage == SwapStage::CompleteShowTrackProgress {
                //             text = "Start New Swap";
                //         }
                //         if ui.button(text).clicked() {
                //             ls.swap_state.stage = previous_stage;
                //
                //             if ls.swap_state.stage == SwapStage::CompleteShowTrackProgress {
                //                 ls.swap_state.tx_progress.reset();
                //                 ls.swap_state.currency_input_box.input_box_str = "".to_string();
                //                 ls.swap_state.stage = SwapStage::StartPreparing;
                //             }
                //         }
                //     });
                // }

                if !ls.swap_state.swap_valid {
                    ui.label(RichText::new("Invalid Swap: ").color(Color32::RED));
                    ui.label(RichText::new(ls.swap_state.invalid_reason.clone()).color(Color32::RED));
                } else {
                    // if ls.swap_state.stage != SwapStage::CompleteShowTrackProgress {
                    //     if ls.swap_state.tx_progress.stage_err.is_none() {
                    //         if !ls.swap_state.changing_stages {
                    //             let changed = Self::big_proceed_button(ui, ls, next_stage, button_text);
                    //             if changed {
                    //                 // All these stages are off by one because they've just "changed" already.
                    //                 match ls.swap_state.stage {
                    //                     SwapStage::StartPreparing => {}
                    //                     SwapStage::ShowAmountsPromptSigning => {
                    //                         // create_swap_tx(ls);
                    //                     }
                    //                     SwapStage::ViewSignedAllowBroadcast => {
                    //                         // sign_swap(ls, ls.swap_state.tx_progress.prepared_tx.clone().unwrap())
                    //                     }
                    //                     SwapStage::CompleteShowTrackProgress => {
                    //                         // broadcast_swap(ls, ls.swap_state.tx_progress.prepared_tx.clone().unwrap())
                    //                     }
                    //                 }
                    //             }
                    //         }
                    //     }
                    // }
                }
            });

            if SwapStage::CompleteShowTrackProgress == ls.swap_state.stage {
                ui.label("Track your swap progress on the party AMM explorer link above");
            }
        }
    }

    fn big_proceed_button(ui: &mut Ui, ls: &mut LocalState, next_stage: SwapStage, button_text: &str) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            let style = ui.style_mut();
            style.override_text_style = Some(TextStyle::Heading);
            changed = ui.button(button_text).clicked();
            if changed {
                ls.swap_state.stage = next_stage;
            }
        });
        changed
    }

    fn swap_details(ui: &mut Ui, ls: &mut LocalState, locked: bool) {
        ui.separator();
        let price_map_incl = ls.price_map_incl_rdg();

        ui.horizontal(|ui| {
            // ui.label("Swap From: ");
            ls.swap_state.currency_input_box.locked = locked;
            ls.swap_state.currency_input_box.view(ui, &price_map_incl);
        });
        ui.horizontal(|ui| {
            // ui.label("Swap To: ");
            let input_changed = ls.swap_state.currency_input_box.currency_has_changed;
            // let input_changed = currency_selection_box(ui, &mut ls.swap_state.input_currency, "To", supported_wallet_currencies(), locked);
            if input_changed {
                if ls.swap_state.currency_input_box.input_currency == SupportedCurrency::Redgold {
                    ls.swap_state.output_currency = SupportedCurrency::Ethereum;
                } else {
                    ls.swap_state.output_currency = SupportedCurrency::Redgold;
                }
            }
            let currency = ls.swap_state.currency_input_box.input_currency;
            let filtered_swap_outputs = Self::filter_swap_output(&currency);
            currency_combo_box(ui, &mut ls.swap_state.output_currency, "Destination",
                               filtered_swap_outputs, locked);

            let use_usd = ls.swap_state.currency_input_box.use_usd_input.clone();

            let mut no_data = false;

            let is_ask = ls.swap_state.output_currency == SupportedCurrency::Redgold;
            let get_prices_of_currency = if is_ask {
                ls.swap_state.currency_input_box.input_currency.clone()
            } else {
                ls.swap_state.output_currency.clone()
            };

            if let Some(cp) = ls.first_party.as_ref()
                .and_then(|p| p.party_events.as_ref())
                .and_then(|pe| pe.central_prices.get(&get_prices_of_currency)) {

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


                let input_amount_value = ls.swap_state.currency_input_box.input_amount_value();
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
                let fulfilled_amt = cp.dummy_fulfill(x as u64, is_ask, &ls.node_config.network, get_prices_of_currency);
                if fulfilled_amt == 0.0 {
                    ls.swap_state.swap_valid = false;
                    ls.swap_state.invalid_reason = "Order below minimum amount, or insufficient party liquidity".to_string();
                }
                let mut fulfilled_value_usd = fulfilled_amt * price_usd_est_inverse;
                let mut fulfilled_str = format!("{:?} fulfilled", ls.swap_state.output_currency);
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

    fn party_explorer_link(ui: &mut Ui, ls: &&mut LocalState) {
        if let Some(pa) = ls.first_party.as_ref()
            .and_then(|p| p.party_info.party_key.as_ref())
            .and_then(|p| p.to_bitcoin_address_typed(&ls.node_config.network).ok())
            .and_then(|p| p.render_string().ok())
        {
            ui.hyperlink_to("Party Explorer Link", ls.node_config.network.explorer_hash_link(pa));
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


#[tokio::test]
async fn debug_fulfill() {
    let nc = NodeConfig::default_env(NetworkEnvironment::Dev).await;
    let pev = nc.api_rg_client().party_data().await.unwrap().into_values().next().unwrap().party_events.unwrap();
    let cpp = pev.central_prices.get(&SupportedCurrency::Ethereum).unwrap();
    let f = cpp.dummy_fulfill(16500000 as u64, false, &nc.network, SupportedCurrency::Ethereum);
    println!("{}", f);
    println!("{}", cpp.json_or());
}