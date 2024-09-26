use std::collections::HashMap;
use bdk::Utxo::Local;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, RichText, TextStyle, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use tracing_subscriber::fmt::format;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use crate::gui::app_loop::LocalState;
use redgold_gui::common;
use redgold_gui::components::currency_input::{currency_combo_box, supported_wallet_currencies, CurrencyInputBox};
use redgold_gui::components::tx_progress::TransactionProgressFlow;
use redgold_keys::address_external::ToBitcoinAddress;
use crate::gui::tabs::transact::wallet_tab::SendReceiveTabs;
use crate::gui::tabs::transact::wallet_tab::SendReceiveTabs::Swap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString)]
pub enum SwapStage {
    StartPreparing,
    ShowAmountsPromptSigning,
    ViewSignedAllowBroadcast,
    CompleteShowTrackProgress
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SwapState {
    pub active: bool,
    pub output_currency: SupportedCurrency,
    pub stage: SwapStage,
    pub tx_progress: TransactionProgressFlow,
    pub currency_input_box: CurrencyInputBox,
    pub changing_stages: bool
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
        }
    }
}
impl SwapState {

    pub fn view(ui: &mut Ui, ls: &mut LocalState) {
        ls.swap_state.active = ls.wallet.send_receive == Some(SendReceiveTabs::Swap);
        // if ui.button("Refresh Rates").clicked() {
        //     ls.price_map_usd_pair
        // }
        if ls.swap_state.active {
            ui.horizontal(|ui| {
                ui.heading("Swap");
                Self::party_explorer_link(ui, &ls);
            });
            // ui.separator();
            let mut next_stage = SwapStage::ShowAmountsPromptSigning;;
            let mut previous_stage = SwapStage::StartPreparing;;
            let mut button_text = "Start Swap";

            match ls.swap_state.stage {
                SwapStage::StartPreparing => {
                    Self::swap_details(ui, ls, false);
                    ui.heading("Lock Swap Details");
                    next_stage = SwapStage::ShowAmountsPromptSigning;
                    button_text = "Start Swap";
                }
                SwapStage::ShowAmountsPromptSigning => {
                    Self::swap_details(ui, ls, true);
                    ui.heading("Swap Values Locked: Prepare to Sign");
                    next_stage = SwapStage::ViewSignedAllowBroadcast;
                    button_text = "Sign Transaction";
                }
                SwapStage::ViewSignedAllowBroadcast => {
                    Self::swap_details(ui, ls, true);
                    ui.heading("Swap Signed: Prepare to Broadcast");
                    previous_stage = SwapStage::ShowAmountsPromptSigning;
                    next_stage = SwapStage::CompleteShowTrackProgress;
                    button_text = "Broadcast Transaction";
                }
                SwapStage::CompleteShowTrackProgress => {
                    Self::swap_details(ui, ls, true);
                    previous_stage = SwapStage::StartPreparing;
                    ui.heading("Swap Complete");
                }
            }

            ls.swap_state.tx_progress.view(ui);

            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                let style = ui.style_mut();
                style.override_text_style = Some(TextStyle::Heading);
                if ui.button("Reset").clicked() {
                    ls.swap_state.tx_progress.reset();
                    ls.swap_state.currency_input_box.input_box_str = "".to_string();
                    ls.swap_state.stage = SwapStage::StartPreparing;
                }
                });
                if ls.swap_state.stage != SwapStage::StartPreparing {
                    ui.horizontal(|ui| {
                        let style = ui.style_mut();
                        style.override_text_style = Some(TextStyle::Heading);
                        let mut text = "Back";
                        if ls.swap_state.stage == SwapStage::CompleteShowTrackProgress {
                            text = "Start New Swap";
                        }
                        if ui.button(text).clicked() {
                            ls.swap_state.stage = previous_stage;

                            if ls.swap_state.stage == SwapStage::CompleteShowTrackProgress {
                                ls.swap_state.tx_progress.reset();
                                ls.swap_state.currency_input_box.input_box_str = "".to_string();
                                ls.swap_state.stage = SwapStage::StartPreparing;
                            }
                        }
                    });
                }

            if ls.swap_state.stage != SwapStage::CompleteShowTrackProgress {
                if ls.swap_state.tx_progress.stage_err.is_none() {
                    if !ls.swap_state.changing_stages {
                        let changed = Self::big_proceed_button(ui, ls, next_stage, button_text);
                        if changed {
                            // All these stages are off by one because they've just "changed" already.
                            match ls.swap_state.stage {
                                SwapStage::StartPreparing => {}
                                SwapStage::ShowAmountsPromptSigning => {
                                    LocalState::create_swap_tx(ls);
                                }
                                SwapStage::ViewSignedAllowBroadcast => {
                                    LocalState::sign_swap(ls, ls.swap_state.tx_progress.prepared_tx.clone().unwrap())
                                }
                                SwapStage::CompleteShowTrackProgress => {
                                    LocalState::broadcast_swap(ls, ls.swap_state.tx_progress.prepared_tx.clone().unwrap())
                                }
                            }
                        }
                    }
                }
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
                let fulfilled_amt = cp.dummy_fulfill(x as u64, is_ask, &ls.node_config.network);
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
