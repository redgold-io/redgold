use eframe::egui::{ComboBox, Ui};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;

fn currency_selection_box(ui: &mut Ui, currency_selector: &mut SupportedCurrency) {
    ComboBox::from_label("Currency")
        .selected_text(format!("{:?}", currency_selector))
        .show_ui(ui, |ui| {
            let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold];
            for style in styles {
                ui.selectable_value(currency_selector, style.clone(), format!("{:?}", style));
            }
        });
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumIter, EnumString)]
enum SwapStage {
    StartPreparing,
    ShowAmountsPromptSigning,
    ViewSignedAllowBroadcast,
    CompleteShowTrackProgress
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct SwapState {
    pub active: bool,
    pub input_currency: SupportedCurrency
}

impl SwapState {
    pub fn view(&self, ui: &mut Ui, ls: &mut LocalState) {
        ui.horizontal(|ui| {
            // currency_selection_box(ui, &mut ls.swap_state.input_currency);
        });
    }
}