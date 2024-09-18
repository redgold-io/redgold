use eframe::egui::{ComboBox, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;

fn currency_selection_box(ui: &mut Ui, currency_selector: &mut SupportedCurrency) {
    ComboBox::from_label("Input")
        .selected_text(format!("{:?}", currency_selector))
        .show_ui(ui, |ui| {
            let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold, SupportedCurrency::Ethereum];
            for style in styles {
                ui.selectable_value(currency_selector, style.clone(), format!("{:?}", style));
            }
        });
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
    pub stage: SwapStage
}

impl Default for SwapState {
    fn default() -> Self {
        Self {
            active: false,
            input_currency: SupportedCurrency::Ethereum,
            stage: SwapStage::StartPreparing,
        }
    }
}
impl SwapState {
    pub fn view(ui: &mut Ui, ls: &mut LocalState) {
        currency_selection_box(ui, &mut ls.swap_state.input_currency);
    }
}
