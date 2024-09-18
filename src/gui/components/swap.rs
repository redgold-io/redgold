use eframe::egui;
use eframe::egui::{Color32, ComboBox, RichText, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab::SendReceiveTabs;

fn currency_selection_box(ui: &mut Ui, currency_selector: &mut SupportedCurrency, label: impl Into<String>, supported: Vec<SupportedCurrency>) {
    ComboBox::from_label(label.into())
        .selected_text(format!("{:?}", currency_selector))
        .show_ui(ui, |ui| {
            let styles = supported;
            for style in styles {
                ui.selectable_value(currency_selector, style.clone(), format!("{:?}", style));
            }
        });
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
        if ls.swap_state.active {
            match ls.swap_state.stage {
                SwapStage::StartPreparing => {
                    ui.heading("Enter Swap Details");
                    ui.separator();
                    ui.horizontal(|ui| {
                        currency_selection_box(ui, &mut ls.swap_state.input_currency, "Input Currency", supported_swap_currencies());
                        let currency = ls.swap_state.input_currency;
                        currency_selection_box(ui, &mut ls.swap_state.output_currency, "Output Currency",
                                               supported_swap_currencies()
                                                   .iter().filter(|c| { c != &&currency }).cloned().collect());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Amount to swap");
                        ui.add(egui::TextEdit::singleline(&mut ls.swap_state.swap_amount_input_string).desired_width(200.0));
                        let allow_sats = ls.swap_state.input_currency == SupportedCurrency::Bitcoin || ls.swap_state.input_currency == SupportedCurrency::Redgold &&
                            !ls.swap_state.swap_amount_usd_denomination;
                        let swap_denom = if ls.swap_state.swap_amount_usd_denomination { "USD".to_string() } else { format!("{:?}{}", ls.swap_state.input_currency.clone(), if ls.swap_state.swap_amount_use_sats && allow_sats {" sats"} else {""} ) };
                        ui.label(format!("{}", swap_denom));
                        ui.checkbox(&mut ls.swap_state.swap_amount_usd_denomination, "USD Denomination");

                        if allow_sats {
                            ui.checkbox(&mut ls.swap_state.swap_amount_use_sats, "Use Sats");
                        }
                        let mut v = ls.swap_state.input_amount_value();
                        if ls.swap_state.swap_amount_usd_denomination {
                            v = v * 1.0;
                        } else {
                            let currency = ls.swap_state.input_currency.clone();
                            ls.price_map_usd_pair.get(&currency).map(|price| {
                                v = v * price;
                            });
                        }

                        ui.label("Value USD:");
                        let value = format!("${:.2}", v);
                        ui.label(RichText::new(value).color(Color32::GREEN));
                    });
                }
                SwapStage::ShowAmountsPromptSigning => {}
                SwapStage::ViewSignedAllowBroadcast => {}
                SwapStage::CompleteShowTrackProgress => {}
            }

        }
    }
}
