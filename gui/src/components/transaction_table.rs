use eframe::egui;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::explorer::{BriefTransaction, DetailedTransaction};
use redgold_schema::ShortString;
use redgold_schema::structs::{NetworkEnvironment, PublicKey};
use redgold_schema::util::times::ToTimeString;
use crate::components::tables::{table_nonetype, text_table_advanced};
use crate::data_query::data_query::DataQueryInfo;

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

    pub fn full_view<E>(
        &mut self,
        ui: &mut Ui,
        network: &NetworkEnvironment,
        d: &DataQueryInfo<E>,
        filter_pk: Option<&PublicKey>

    ) where E: ExternalNetworkResources + Clone + Send + 'static {

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

