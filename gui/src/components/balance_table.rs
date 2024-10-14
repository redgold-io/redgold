use eframe::egui::{Color32, RichText, Ui};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::structs::{PublicKey, SupportedCurrency};
use redgold_schema::util::dollar_formatter::format_dollar_amount;
use crate::components::tables::text_table_advanced;
use crate::data_query::data_query::DataQueryInfo;

pub fn balance_table<E>(
    ui: &mut Ui,
    d: &DataQueryInfo<E>,
    nc: &NodeConfig,
    balances: Option<Vec<SupportedCurrency>>,
    pk_filter: Option<&PublicKey>
) where E: ExternalNetworkResources + Send + Clone + 'static {
    let balance_currencies = balances.unwrap_or(queryable_balances());
    let mut headers = vec!["Denomination".to_string()];
    for b in balance_currencies.iter() {
        headers.push(b.to_display_string());
    }
    let mut table_data: Vec<Vec<String>> = vec![];
    table_data.push(headers.clone());
    let balances = d.balance_totals(&nc.network, pk_filter);
    let mut row = vec!["NAV".to_string()];
    let ordered_cur = balance_currencies;
    for c in ordered_cur.iter() {
        let bal = balances.get(&c).cloned().unwrap_or(0.0);
        if bal > 1.0 {
            row.push(format!("{:.2}", bal));
        } else {
            row.push(format!("{:.8}", bal));
        }
    }
    table_data.push(row);

    let curs = ordered_cur.clone();
    let deltas = d.delta_24hr_external.clone();
    let func = move |ui: &mut Ui, row: usize, col: usize, val: &String| {
        if row == 1 && col > 0 {
            ui.label(val.clone());
            deltas.get(&curs[col - 1]).map(|d| {
                let str = format!("{:.2}%", d * 100.0);
                let (symbol_dir, color) = if *d > 0.0 {
                    (up_symbol_str(), Color32::GREEN)
                } else {
                    (down_symbol_str(), Color32::RED)
                };
                ui.label(RichText::new(format!("{} {}", symbol_dir, str)).color(color));
            });
            return true;
        }
        false
    };

    let mut row = vec!["USD/Pair Price".to_string()];
    let price_map = d.price_map_usd_pair_incl_rdg.clone();
    for c in ordered_cur.iter() {
        let price = price_map.get(&c).cloned().unwrap_or(0.0);
        if c == &SupportedCurrency::Usdt || c == &SupportedCurrency::Usdc {
            row.push(format!("${:.4}", price));
        } else {
            row.push(format!("${}", format_dollar_amount(price)));
        }
    }
    table_data.push(row);

    let mut row = vec!["NAV USD".to_string()];
    let nav_totals = d.nav_usd_by_currency(&nc.network, pk_filter);
    let row_idx = 2;
    let mut green_fields = (1..8).map(|x| (row_idx, x)).collect::<Vec<(usize, usize)>>();
    for (idx, c) in ordered_cur.iter().enumerate() {
        let amount = nav_totals.get(&c).cloned().unwrap_or(0.0);
        if amount == 0.0 {
            green_fields = green_fields.into_iter().filter(|x| x.1 != idx + 1).collect();
        }
        let dollar_amount = format_dollar_amount(amount);
        // let change_24h
        row.push(format!("${}", dollar_amount));
    }
    table_data.push(row);
    text_table_advanced(ui, table_data, false, false, None, green_fields, Some(func));
}

pub fn queryable_balances() -> Vec<SupportedCurrency> {
    vec![
        SupportedCurrency::Redgold,
        SupportedCurrency::Bitcoin,
        SupportedCurrency::Ethereum,
        SupportedCurrency::Monero,
        SupportedCurrency::Solana,
        SupportedCurrency::Usdt
    ]
}

pub fn down_symbol_str() -> String {
    "⬇".to_string()
}

pub fn up_symbol_str() -> String {
    "⬆".to_string()
}
