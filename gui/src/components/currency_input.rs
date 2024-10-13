use std::collections::HashMap;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, RichText, Ui};
use serde::{Deserialize, Serialize};
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};

pub fn currency_combo_box(ui: &mut Ui, currency_selector: &mut SupportedCurrency, label: impl Into<String>, supported: Vec<SupportedCurrency>, locked: bool) -> bool {
    let mut changed = false;
    let mut c = currency_selector.clone();
    let currency_selector = if locked {
        &mut c
    } else {
        currency_selector
    };
    ComboBox::from_label(label.into())
        .width(80.0)
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

pub fn supported_wallet_currencies() -> Vec<SupportedCurrency> {
    vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold, SupportedCurrency::Ethereum]
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CurrencyInputBox {
    pub input_box_str: String,
    pub use_usd_input: bool,
    pub use_sats_input: bool,
    pub input_currency: SupportedCurrency,
    pub locked: bool,
    pub input_has_changed: bool,
    pub currency_has_changed: bool,
    pub currency_box_right_label: String,
    pub allowed_currencies: Option<Vec<SupportedCurrency>>
}

impl Default for CurrencyInputBox {
    fn default() -> Self {
        Self {
            input_box_str: "".to_string(),
            use_usd_input: true,
            use_sats_input: false,
            input_currency: SupportedCurrency::Redgold,
            locked: false,
            input_has_changed: false,
            currency_has_changed: false,
            currency_box_right_label: "Currency".to_string(),
            allowed_currencies: None,
        }
    }
}

impl CurrencyInputBox {

    pub fn reset(&mut self) {
        self.input_box_str = "".to_string();
        self.input_has_changed = false;
        self.currency_has_changed = false;
        self.use_usd_input = true;
        self.use_sats_input = false;
        self.input_currency = SupportedCurrency::Redgold;
        self.locked = false;
    }

    pub fn input_amount_value(&self) -> f64 {
        let input = self.input_box_str.parse::<f64>().unwrap_or(0.0);
        let amt_fract = if self.use_usd_input {
            input
        } else {
            if self.use_sats_input {
                input / 1e8
            } else {
                input
            }
        };
        amt_fract
    }

    pub fn input_currency_amount(&self, price_map: &HashMap<SupportedCurrency, f64>) -> CurrencyAmount {
        let v = self.input_amount_value();
        let amt = if self.use_usd_input {
            let price = price_map.get(&self.input_currency).unwrap_or(&1.0);
            v / price
        } else {
            v
        };
        let zero = CurrencyAmount::zero(self.input_currency.clone());
        CurrencyAmount::from_fractional_cur(amt, self.input_currency.clone()).unwrap_or(zero)
    }

    pub fn input_usd_value(&self, price_map: &HashMap<SupportedCurrency, f64>) -> f64 {
        let default = 0.0;
        self.input_currency_amount(price_map).to_fractional() * price_map.get(&self.input_currency).unwrap_or(&default)
    }

    pub fn from_currency(currency: SupportedCurrency, label: impl Into<String>) -> Self {
        Self {
            currency_box_right_label: label.into(),
            input_currency: currency,
            ..Default::default()
        }
    }
    pub fn view(&mut self, ui: &mut Ui, price_map_usd: &HashMap<SupportedCurrency, f64>) {
        ui.horizontal(|ui| {
            let allowed = self.allowed_currencies.clone().unwrap_or(supported_wallet_currencies());
            self.currency_has_changed = currency_combo_box(
                ui, &mut self.input_currency, self.currency_box_right_label.clone(),
                allowed, self.locked);
            self.input_box(ui);
            self.input_denomination(ui);
            self.usd_checkbox(ui);
            self.sats_checkbox(ui);
            self.value_show(price_map_usd, ui);
        });
    }

    fn value_show(&mut self, price_map_usd: &HashMap<SupportedCurrency, f64>, ui: &mut Ui) {
        ui.label("Value:");
        let value = if !self.use_usd_input {
            format!("${:.2} USD", self.input_usd_value(price_map_usd))
        } else {
            format!("{:.8} {:?}", self.input_currency_amount(price_map_usd).to_fractional(), self.input_currency)
        };
        ui.label(RichText::new(value).color(Color32::GREEN));
    }

    fn sats_checkbox(&mut self, ui: &mut Ui) {
        if !self.use_usd_input {
            let mut use_sats = &mut self.use_sats_input;
            let mut x2 = use_sats.clone();
            if self.locked {
                use_sats = &mut x2;
            }
            ui.checkbox(use_sats, "Sats");
        }
    }

    fn usd_checkbox(&mut self, ui: &mut Ui) {
        let mut check = &mut self.use_usd_input;
        let mut x1 = check.clone();
        if self.locked {
            check = &mut x1;
        }
        ui.checkbox(check, "USD");
    }

    fn input_denomination(&mut self, ui: &mut Ui) {
        let denominaton = if self.use_usd_input {
            "USD".to_string()
        } else {
            format!("{:?}{}", self.input_currency,
                    if self.use_sats_input && !self.use_usd_input { " sats" } else { "" })
        };
        ui.label(format!("{}", denominaton));
    }

    fn input_box(&mut self, ui: &mut Ui) {
        let mut text = &mut self.input_box_str;
        let mut string = text.clone();
        if self.locked {
            text = &mut string;
        }
        let edit = egui::TextEdit::singleline(text).desired_width(100.0);

        let response = ui.add(edit);
        if response.changed() {
            self.input_has_changed = true;
        }
    }
}