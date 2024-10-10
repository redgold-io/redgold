use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use redgold_schema::explorer::{BriefTransaction, DetailedTransaction};
use redgold_schema::ShortString;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::util::times::ToTimeString;
use crate::components::tables::text_table_advanced;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TransactionTable {
    pub rows: Vec<BriefTransaction>
}

impl Default for TransactionTable {
    fn default() -> Self {
        Self {
            rows: vec![]
        }
    }
}

pub fn format_fractional_currency_amount(amount: f64) -> String {
    if amount > 1.0 {
        format!("{:.2}", amount)
    } else {
        format!("{:.8}", amount)
    }
}

impl TransactionTable {
    pub fn view(&mut self, ui: &mut Ui, network: &NetworkEnvironment) {
        let mut data = vec![];
        let headers = vec!["Hash", "From", "To", "Time", "First Amount", "Total Amount", "Incoming"
         ,"Fee",
        ];
        data.push(headers.iter().map(|s| s.to_string()).collect());
        for r in &self.rows {
            data.push(vec![
                r.hash.clone(),
                r.from.clone(),
                r.to.clone(),
                r.timestamp.to_time_string_shorter_no_seconds(),
                format_fractional_currency_amount(r.first_amount),
                format_fractional_currency_amount(r.amount),
                r.incoming.map(|b| b.to_string()).unwrap_or("".to_string()),
                format!("{} sats", r.fee.to_string()),
            ]);
        }
        let func = move |ui: &mut Ui, row: usize, col: usize, val: &String| {
            if (0..3).into_iter().collect::<Vec<usize>>().contains(&col) {
                ui.hyperlink_to(val.first_four_last_four_ellipses().unwrap_or("err".to_string()), network.explorer_hash_link(val.clone()));
                return true
            }
            false
        };
        text_table_advanced(ui, data, false, false, None, vec![], Some(func));

    }
}

