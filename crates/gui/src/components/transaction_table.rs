use eframe::egui;
use eframe::egui::{RichText, Ui};
use serde::{Deserialize, Serialize};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::explorer::{BriefTransaction, DetailedTransaction};
use redgold_schema::ShortString;
use redgold_schema::structs::{NetworkEnvironment, PublicKey};
use redgold_schema::util::times::ToTimeString;
use crate::common::green_label;
use crate::components::tables::{table_nonetype, text_table_advanced};
use crate::data_query::data_query::DataQueryInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TransactionTable {
    pub rows: Vec<BriefTransaction>,
    pub stake_mode: bool
}

impl Default for TransactionTable {
    fn default() -> Self {
        Self {
            rows: vec![],
            stake_mode: false,
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
        let mut headers = vec![
            "Hash", "From", "To", "Time", "First Amount", "Total Amount", "Incoming", "Type"
        ];
        if self.stake_mode {
            headers = vec!["Hash", "Time", "Amount", "Currency", "Fee"]
        }
        data.push(headers.iter().map(|s| s.to_string()).collect());
        for r in &self.rows {
            let mut row = vec![
                r.hash.clone(),
                r.from.clone(),
                r.to.clone(),
                r.timestamp.to_time_string_shorter_no_seconds(),
                format_fractional_currency_amount(r.first_amount),
                format_fractional_currency_amount(r.amount),
                r.incoming.map(|b| b.to_string()).unwrap_or("".to_string()),
                r.address_event_type.as_ref().map(|e| format!("{:?}", e)).unwrap_or("".to_string())
            ];
            if self.stake_mode {
                row = vec![
                    r.hash.clone(),
                    r.timestamp.to_time_string_shorter_no_seconds(),
                    format_fractional_currency_amount(r.amount),
                    r.currency.clone().map(|c| c.replace("\"", "")).unwrap_or("".to_string()),
                    format!("{} sats", r.fee.to_string()),
                ];
            }
            data.push(row);
        }
        let mode = self.stake_mode.clone();
        let func = move |ui: &mut Ui, row: usize, col: usize, val: &String| {
            let mut column_idxs = (0..3).into_iter().collect::<Vec<usize>>();
            if mode {
                column_idxs = (0..1).into_iter().collect::<Vec<usize>>();
            }
            if column_idxs.contains(&col) {
                ui.hyperlink_to(val.first_four_last_four_ellipses().unwrap_or("err".to_string()), network.explorer_hash_link(val.clone()));
                return true
            }
            if self.stake_mode {
                if col == 2 {
                    green_label(ui, val.clone());
                    return true
                }
            }
            false
        };
        text_table_advanced(ui, data, false, false, None, vec![], Some(func));

    }

    pub fn full_view<E>(
        &mut self,
        ui: &mut Ui,
        network: &NetworkEnvironment,
        d: &DataQueryInfo<E>,
        filter_pk: Option<&PublicKey>

    ) where E: ExternalNetworkResources + Clone + Send + 'static + Sync {

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                self.view(ui, &network);
            });

            // ui.separator();
            ui.spacing();
            ui.add(egui::Separator::default().vertical());

            ui.vertical(|ui| {
                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        let mut right_table = vec![vec!["RDG Incoming", "RDG Outgoing"].iter().map(|x| x.to_string()).collect::<Vec<String>>()];
                        right_table.push(vec![
                            d.total_incoming.lock().map(|t| t.clone())
                                .unwrap_or_default()
                                .iter()
                                .filter(|(k, _)| filter_pk.map(|f| f == *k).unwrap_or(true))
                                .map(|(_, v)| v)
                                .sum::<i64>().to_string(),
                            d.total_outgoing.lock().map(|t| t.clone()).unwrap_or_default().iter()
                                .filter(|(k, _)| filter_pk.map(|f| f == *k).unwrap_or(true))
                                .map(|(_, v)| v)
                                .sum::<i64>().to_string(),
                        ]);
                        text_table_advanced(ui, right_table, false, false, None, vec![], table_nonetype());


                        let mut right_table = vec![vec!["RDG UTXOs", "RDG Transactions"].iter().map(|x| x.to_string()).collect::<Vec<String>>()];
                        right_table.push(vec![
                            d.total_utxos.lock().map(|t| t.clone()).unwrap_or_default().iter()
                                .filter(|(k, _)| filter_pk.map(|f| f == *k).unwrap_or(true))
                                .map(|(_, v)| v)
                                .sum::<i64>().to_string(),
                            d.total_transactions.lock().map(|t| t.clone()).unwrap_or_default().iter()
                                .filter(|(k, _)| filter_pk.map(|f| f == *k).unwrap_or(true))
                                .map(|(_, v)| v)
                                .sum::<i64>().to_string(),
                        ]);
                        text_table_advanced(ui, right_table, false, false, None, vec![], table_nonetype());

                        // Todo: info on swaps, num RDG swaps. num stake tx
                        // should there be an endpoint specific to a particular public key?
                        let next_table = vec![vec!["Swaps", "Stake TX"].iter().map(|x| x.to_string()).collect::<Vec<String>>()];

                    });
            });
        });

    }

}

