use eframe::egui;
use eframe::egui::{ScrollArea, TextEdit, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_gui::components::currency_input::{currency_combo_box, CurrencyInputBox, supported_wallet_currencies};
use redgold_schema::structs::{PublicKey, SupportedCurrency};
use crate::gui::app_loop::LocalState;
use crate::gui::tables;
use crate::gui::tables::text_table_advanced;
use crate::gui::tabs::transact::wallet_tab::{SendReceiveTabs};

#[derive(Clone, Default)]
pub struct Portfolio {
    pub name: String,
    pub rows: Vec<PortfolioRow>,
}

impl Portfolio {
    pub fn normalize_weight_update(&mut self) {
        let total_weight = self.rows.iter().map(|r| r.weight).sum::<f64>();
        for r in self.rows.iter_mut() {
            r.weight_normalized = r.weight / total_weight;
        }
    }

}

#[derive(Clone)]
pub struct PortfolioSelector {

}

#[derive(Clone)]
pub struct PortfolioState {
    pub tab: PortfolioTransactSubTab,
    pub rdg_input: CurrencyInputBox,
    pub portfolio_input_name: String,
    pub add_new_currency: SupportedCurrency,
    pub port: Portfolio,
    pub weight_input: String
}

impl Default for PortfolioState {
    fn default() -> Self {
        let mut box_input = CurrencyInputBox::default();
        box_input.allowed_currencies = Some(vec![SupportedCurrency::Redgold]);
        Self {
            tab: PortfolioTransactSubTab::View,
            rdg_input: box_input,
            portfolio_input_name: "new portfolio".to_string(),
            add_new_currency: SupportedCurrency::Bitcoin,
            port: Default::default(),
            weight_input: "1".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, EnumIter, EnumString, Debug)]
pub enum PortfolioTransactSubTab {
    View,
    Update,
    Create,
    Liquidate,
}


pub fn portfolio_view(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {
    port_subtabs(ui, ls, pk);
    ui.separator();
    ui.heading(format!("{:?}", ls.wallet.port.tab));
    ui.separator();
    match ls.wallet.port.tab {
        PortfolioTransactSubTab::View => {}
        PortfolioTransactSubTab::Update => {}
        PortfolioTransactSubTab::Create => {
            create_portfolio(ui, ls, pk)
        }
        PortfolioTransactSubTab::Liquidate => {}
    }
}

#[derive(Clone, PartialEq, EnumIter, EnumString, Debug)]
enum PortfolioEntryType {
    Currency,
    // Future impls
    // Product,
    // Portfolio
}

impl Default for PortfolioEntryType {
    fn default() -> Self {
        PortfolioEntryType::Currency
    }
}

#[derive(Clone, Default)]
struct PortfolioRow {
    entry_type: PortfolioEntryType,
    id: String,
    weight: f64,
    weight_normalized: f64,
    editable: bool,
    nav_usd: f64,
    nav_pair: f64,
    fulfillment_imbalance_usd: f64,
    fulfillment_imbalance_pair: f64,
    // Future impls
    // product: Product,
    // portfolio: Portfolio
}

impl PortfolioRow {

}

fn create_portfolio(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {
    ui.horizontal(|ui| {
        ui.label("Portfolio Name:");
        TextEdit::singleline(&mut ls.wallet.port.portfolio_input_name).desired_width(100.0).show(ui);
    });
    let price_map = ls.price_map_incl_rdg();
    ui.horizontal(|ui| {
        ui.label("Stake Input");
        ls.wallet.port.rdg_input.view(ui, &price_map);
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Add Portfolio Entry:");
        currency_combo_box(ui, &mut ls.wallet.port.add_new_currency, "Id", supported_wallet_currencies(), false);
        ui.label("Weight:");
        TextEdit::singleline(&mut ls.wallet.port.weight_input).desired_width(50.0).show(ui);
        if ui.button("Add").clicked() {
            let new_row = PortfolioRow {
                entry_type: PortfolioEntryType::Currency,
                id: format!("{:?}", ls.wallet.port.add_new_currency),
                weight: ls.wallet.port.weight_input.parse().unwrap(),
                weight_normalized: 0.0,
                editable: false,
                nav_usd: 0.0,
                nav_pair: 0.0,
                fulfillment_imbalance_usd: 0.0,
                fulfillment_imbalance_pair: 0.0,
            };
            ls.wallet.port.port.rows.push(new_row);
            ls.wallet.port.port.normalize_weight_update();
        }
    });
    ui.separator();
    ScrollArea::vertical().id_source("porttable")
        .max_height(250.0)
        .min_scrolled_height(150.0)
        .max_width(900.0)
        .min_scrolled_width(900.0)
        .auto_shrink(true)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                portfolio_table(ui, ls, pk);
            });
        });

}


pub fn portfolio_table(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {


    let data = ls.wallet.port.port.rows.clone();

    let headers = vec!["Type", "Id", "Weight", "Normalized Weight", "NAV USD", "NAV Quote", "Fill USD", "Fill Quote"];
    let mut data_str = vec![];
    data_str.push(headers.iter().map(|h| h.to_string()).collect());
    for row in data {
        data_str.push(vec![
            format!("{:?}", row.entry_type),
            row.id.clone(),
            format!("{:.2}", row.weight),
            format!("{:.2}", row.weight_normalized),
            format!("{:.2}", row.nav_usd),
            format!("{:.2}", row.nav_pair),
            format!("{:.2}", row.fulfillment_imbalance_usd),
            format!("{:.2}", row.fulfillment_imbalance_pair),
        ]);
    };
    let event = text_table_advanced(ui, data_str, true, false);
    if let Some(index) = event.delete_row_id.as_ref() {
        ls.wallet.port.port.rows.remove(index.clone());
        ls.wallet.port.port.normalize_weight_update();
    }
}



pub fn port_subtabs(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey) {
    ui.horizontal(|ui| {
        for t in PortfolioTransactSubTab::iter() {
            if ui.button(format!("{:?}", t)).clicked() {
                ls.wallet.port.tab = t.clone();
            }
        }
    });
}
