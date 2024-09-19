use eframe::egui;
use eframe::egui::{Color32, ComboBox, RichText, TextStyle, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use tracing_subscriber::fmt::format;
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab::SendReceiveTabs;

fn currency_selection_box(ui: &mut Ui, currency_selector: &mut SupportedCurrency, label: impl Into<String>, supported: Vec<SupportedCurrency>, locked: bool) -> bool {
    let mut changed = false;
    let mut c = currency_selector.clone();
    let currency_selector = if locked {
        &mut c
    } else {
        currency_selector
    };
    ComboBox::from_label(label.into())
        .selected_text(format!("{:?}", currency_selector))
        .show_ui(ui, |ui| {
            let styles = supported;
            for style in styles {
                if ui.selectable_value(currency_selector, style.clone(), format!("{:?}", style)).changed() {
                    changed = true;
                }
            }
        });
    changed
}

pub fn supported_swap_currencies() -> Vec<SupportedCurrency> {
    vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold, SupportedCurrency::Ethereum]
}

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
    pub input_currency: SupportedCurrency,
    pub output_currency: SupportedCurrency,
    pub stage: SwapStage,
    pub swap_amount_input_string: String,
    pub swap_amount_usd_denomination: bool,
    pub swap_amount_use_sats: bool
}

impl Default for SwapState {
    fn default() -> Self {
        Self {
            active: false,
            input_currency: SupportedCurrency::Ethereum,
            output_currency: SupportedCurrency::Redgold,
            stage: SwapStage::StartPreparing,
            swap_amount_input_string: "".to_string(),
            swap_amount_usd_denomination: true,
            swap_amount_use_sats: false,
        }
    }
}
impl SwapState {

    pub fn input_amount_value(&self) -> f64 {
        let input = self.swap_amount_input_string.parse::<f64>().unwrap_or(0.0);
        let amt_fract = if self.swap_amount_usd_denomination {
            input
        } else {
            let currency = self.input_currency.clone();
            if currency == SupportedCurrency::Bitcoin || currency == SupportedCurrency::Redgold {
                if self.swap_amount_use_sats {
                    input / 1e8
                } else {
                    input
                }
            } else {
                input
            }
        };
        amt_fract
    }

    pub fn view(ui: &mut Ui, ls: &mut LocalState) {
        ls.swap_state.active = ls.wallet_state.send_receive == Some(SendReceiveTabs::Swap);
        // if ui.button("Refresh Rates").clicked() {
        //     ls.price_map_usd_pair
        // }
        if ls.swap_state.active {
            match ls.swap_state.stage {
                SwapStage::StartPreparing => {
                    ui.heading("Enter Swap Details");
                    Self::swap_details(ui, ls, false);
                    let next_stage = SwapStage::ShowAmountsPromptSigning;
                    let button_text = "Start Swap";
                    Self::big_proceed_button(ui, ls, next_stage, button_text);
                }
                SwapStage::ShowAmountsPromptSigning => {
                    ui.heading("Swap Values Locked: Prepare to Sign");
                    Self::swap_details(ui, ls, true);
                    let next_stage = SwapStage::ShowAmountsPromptSigning;
                    let button_text = "Sign Transaction";
                    Self::big_proceed_button(ui, ls, next_stage, button_text);
                }
                SwapStage::ViewSignedAllowBroadcast => {
                    ui.heading("Swap Signed: Prepare to Broadcast");
                    Self::swap_details(ui, ls, true);
                    let next_stage = SwapStage::ShowAmountsPromptSigning;
                    let button_text = "Broadcast Transaction";
                    Self::big_proceed_button(ui, ls, next_stage, button_text);
                }
                SwapStage::CompleteShowTrackProgress => {
                    ui.heading("Swap Complete");
                    Self::swap_details(ui, ls, true);
                }
            }

        }
    }

    fn big_proceed_button(ui: &mut Ui, ls: &mut LocalState, next_stage: SwapStage, button_text: &str) {
        ui.horizontal(|ui| {
            let style = ui.style_mut();
            style.override_text_style = Some(TextStyle::Heading);
            if ui.button(button_text).clicked() {
                ls.swap_state.stage = next_stage;
            }
        });
    }

    fn swap_details(ui: &mut Ui, ls: &mut LocalState, locked: bool) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Swap From: ");
            let input_changed = currency_selection_box(ui, &mut ls.swap_state.input_currency, "Input Currency To", supported_swap_currencies(), locked);
            if input_changed {
                if ls.swap_state.input_currency == SupportedCurrency::Redgold {
                    ls.swap_state.output_currency = SupportedCurrency::Ethereum;
                } else {
                    ls.swap_state.output_currency = SupportedCurrency::Redgold;
                }
            }
            let currency = ls.swap_state.input_currency;
            let filtered_swap_outputs = Self::filter_swap_output(&currency);
            currency_selection_box(ui, &mut ls.swap_state.output_currency, "Output Destination Currency",
                                   filtered_swap_outputs, locked);
        });
        let use_usd = ls.swap_state.swap_amount_usd_denomination.clone();

        ui.horizontal(|ui| {
            ui.label("Amount to swap");
            let mut text = &mut ls.swap_state.swap_amount_input_string;
            let mut string = text.clone();
            if locked {
                text = &mut string;
            }
            ui.add(egui::TextEdit::singleline(text).desired_width(200.0));
            let allow_sats = ls.swap_state.input_currency == SupportedCurrency::Bitcoin || ls.swap_state.input_currency == SupportedCurrency::Redgold &&
                !ls.swap_state.swap_amount_usd_denomination;
            let swap_denom = if ls.swap_state.swap_amount_usd_denomination {
                "USD".to_string()
            } else {
                format!("{:?}{}", ls.swap_state.input_currency.clone(),
                        if ls.swap_state.swap_amount_use_sats && allow_sats { " sats" } else { "" })
            };
            ui.label(format!("{}", swap_denom));
            let mut check = &mut ls.swap_state.swap_amount_usd_denomination;
            let mut x1 = check.clone();
            if locked {
                check = &mut x1;
            }
            ui.checkbox(check, "USD Denomination");

            if allow_sats {
                let mut use_sats = &mut ls.swap_state.swap_amount_use_sats;
                let mut x2 = use_sats.clone();
                if locked {
                    use_sats = &mut x2;
                }
                ui.checkbox(use_sats, "Use Sats");
            }
        });
        let mut no_data = false;
        ui.horizontal(|ui| {
            let is_ask = ls.swap_state.output_currency == SupportedCurrency::Redgold;
            let get_prices_of_currency = if is_ask {
                ls.swap_state.input_currency.clone()
            } else {
                ls.swap_state.output_currency.clone()
            };

            if let Some(cp) = ls.first_party.as_ref()
                .and_then(|p| p.party_events.as_ref())
                .and_then(|pe| pe.central_prices.get(&get_prices_of_currency)) {

                // ETH => RDG for example, get_prices = ETH
                // RDG => BTC for example, get_prices = BTC
                let pair_price_usd = ls.price_map_usd_pair.get(&get_prices_of_currency).map(|x| x.clone());

                let mut price_usd_est = cp.min_bid_estimated.clone();
                let mut price_usd_est_inverse = cp.min_bid_estimated.clone();
                if let Some(p) = pair_price_usd {
                    if is_ask {
                        price_usd_est = p.clone()
                    } else {
                        price_usd_est_inverse = p.clone()
                    }
                };


                let input_amount_value = ls.swap_state.input_amount_value();
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

                let prefix = if !use_usd {
                    "Value USD:".to_string()
                } else {
                    format!("Value {:?}:", ls.swap_state.input_currency.clone())
                };
                ui.label(prefix);
                let value = if !use_usd {
                    format!("${:.2}", usd_value)
                } else {
                    format!("{:.8} {:?}", pair_value, get_prices_of_currency.clone())
                };
                ui.label(RichText::new(value).color(Color32::GREEN));

                let x = pair_value * 1e8;
                let fulfilled_amt = cp.dummy_fulfill(x as u64, is_ask, &ls.node_config.network);
                let mut fulfilled_value_usd = fulfilled_amt * price_usd_est_inverse;
                let mut fulfilled_str = format!("{:?} fulfilled", ls.swap_state.output_currency);
                ui.label(fulfilled_str);
                let fulfilled_usd_str = format!("${:.2} USD", fulfilled_value_usd);
                ui.label(RichText::new(format!("{:.8}", fulfilled_amt)).color(Color32::GREEN));


                if ls.node_config.opts.development_mode {
                    ui.label("Bid value:");
                    ui.label(RichText::new(fulfilled_usd_str).color(Color32::RED));
                }
            } else {
                ui.label("No price data available");
                no_data = true;
            }
        });
    }

    fn filter_swap_output(currency: &SupportedCurrency) -> Vec<SupportedCurrency> {
        let remaining = supported_swap_currencies()
            .iter().filter(|c| { c != &currency }).cloned().collect();
        if currency != &SupportedCurrency::Redgold {
            vec![SupportedCurrency::Redgold]
        } else {
            remaining
        }
    }
}
