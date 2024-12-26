use std::collections::HashMap;
use eframe::egui::{Color32, ComboBox, RichText, TextStyle, Ui};
use log::info;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey, SupportedCurrency};
use crate::components::currency_input::{currency_combo_box, supported_wallet_currencies, CurrencyInputBox};
use crate::components::tx_progress::TransactionProgressFlow;
use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::util::dollar_formatter::format_dollar_amount_with_prefix_and_suffix;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString)]
pub enum SwapStage {
    StartPreparing,
    ShowAmountsPromptSigning,
    ViewSignedAllowBroadcast,
    CompleteShowTrackProgress
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
        depends: &G,
        pk: &PublicKey,
        allowed: &Vec<XPubLikeRequestType>,
        csi: &TransactionSignInfo,
        tsi: &TransactionSignInfo,
        data: &DataQueryInfo<E>
    ) -> bool where G: GuiDepends + Clone + Send + 'static,
            E: ExternalNetworkResources + Send + Clone {

        let mut create_swap_tx_bool = false;
        self.check_valid(data, &depends.get_network(), pk);

        ui.horizontal(|ui| {
            ui.heading("Swap");
            Self::party_explorer_link(ui, &data, &depends.get_network(), depends);
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
        });

        let locked = self.tx_progress.locked();
        self.swap_details(ui,locked, data, depends.get_network().clone());

        let ev = self.tx_progress.view(ui,depends, tsi, csi, allowed);
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

        create_swap_tx_bool
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

    fn swap_details<E>(&mut self, ui: &mut Ui, locked: bool, data: &DataQueryInfo<E>, net: NetworkEnvironment) where E: ExternalNetworkResources + Send + Clone {
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
                let fulfilled_amt = cp.dummy_fulfill(x as u64, is_ask, &net, get_prices_of_currency);
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
    ) where E: ExternalNetworkResources + Send + Clone, G: GuiDepends + Clone + Send + 'static {
        if let Some(pa) = data.first_party.as_ref()
            .lock()
            .ok()
            .and_then(|p| p.party_info.party_key.clone())
            .and_then(|p| g.form_btc_address(&p).ok())
            .and_then(|p| p.render_string().ok())
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