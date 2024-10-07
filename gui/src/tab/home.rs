use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::GuiDepends;
use eframe::egui;
use eframe::egui::{Color32, RichText, Ui};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::structs::{NetworkEnvironment, PublicKey, SupportedCurrency};

use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::util::dollar_formatter::format_dollar_amount;
use crate::common::data_item;
use crate::components::tables::{text_table_advanced};

pub fn gui_status_networks() -> Vec<NetworkEnvironment> {
    let _vec = NetworkEnvironment::status_networks();
    vec![NetworkEnvironment::Dev, NetworkEnvironment::Staging]
    // vec
}

#[derive(Clone)]
pub struct HomeState {
    pub last_query_started_time: Option<i64>,
    pub ran_once: bool,
    pub network_healthy: bool,
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            last_query_started_time: None,
            ran_once: false,
            network_healthy: false,
        }
    }
}

impl HomeState {

    pub fn home_screen<G, E>(
        &mut self,
        ui: &mut Ui,
        _ctx: &egui::Context,
        g: &G,
        e: &E,
        d: &DataQueryInfo<E>,
        nc: &NodeConfig,
        loaded_pks: Vec<&PublicKey>,
        current_time: i64,
    ) where
        G: GuiDepends + Send + Clone + 'static,
        E: ExternalNetworkResources + Send + Clone + 'static
    {

        ui.horizontal(|ui| {
            ui.heading("Home");
            let nav_usd = d.nav_usd(&nc.network);
            let str = format_dollar_amount(nav_usd);
            let str = format!("${}", str);
            ui.label("NAV USD: ");
            ui.label(RichText::new(str).color(Color32::GREEN));
            if let Some(p) = g.config_df_path_label() {
                data_item(ui, "Config Path", p);
            };
        });
        ui.separator();
        if self.last_query_started_time
            .map(|q| (current_time - q) > 1000 * 60)
            .unwrap_or(true) {
            self.last_query_started_time = Some(current_time);
            d.refresh_network_info(g);
            if !loaded_pks.is_empty() {
                d.refresh_pks(loaded_pks.clone(), g)
            }
            d.refresh_party_data(g);
            if !self.ran_once {
                self.ran_once = true;
                d.refresh_external_balances(loaded_pks.clone(), g, e, &nc.network);
            }
        }
        self.network_stats_table(ui, d, nc);


        let headers = vec!["Denomination", "Redgold", "Bitcoin", "Ethereum"]
            .iter().map(|x| x.to_string()).collect::<Vec<String>>();
        let mut table_data: Vec<Vec<String>> = vec![];
        table_data.push(headers.clone());
        let balances = d.balance_totals(&nc.network);
        let mut row = vec!["Native".to_string()];
        let ordered_cur = vec![SupportedCurrency::Redgold, SupportedCurrency::Bitcoin, SupportedCurrency::Ethereum];
        for c in ordered_cur.iter() {
            let bal = balances.get(&c).cloned().unwrap_or(0.0);
            if bal > 1.0 {
                row.push(format!("{:.2}", bal));
            } else {
                row.push(format!("{:.8}", bal));
            }
        }
        table_data.push(row);

        let mut row = vec!["USD/Pair Price".to_string()];
        let price_map = d.price_map_usd_pair_incl_rdg.clone();
        for c in ordered_cur.iter() {
            let price = price_map.get(&c).cloned().unwrap_or(0.0);
            row.push(format!("${:.2} USD", price));
        }

        let mut row = vec!["NAV USD".to_string()];
        let nav_totals = d.nav_usd_by_currency(&nc.network);
        let row_idx = 1;
        let mut green_fields = (1..4).map(|x| (row_idx, x)).collect::<Vec<(usize, usize)>>();
        for c in ordered_cur.iter() {
            row.push(format!("${} USD", format_dollar_amount(nav_totals.get(&c).cloned().unwrap_or(0.0))));
        }
        table_data.push(row);
        text_table_advanced(ui, table_data, false, false, None, green_fields);


        // row.push(loaded_pks.len().to_string());


    }

    fn network_stats_table<T>(&mut self, ui: &mut Ui, d: &DataQueryInfo<T>, nc: &NodeConfig)
    where T: ExternalNetworkResources + Clone + Send
    {
        let mut table_data: Vec<Vec<String>> = vec![];
        let headers = vec![
            "Network", "Explorer", "Status", "S3 Hash", "API Hash", "Peers", "Transactions", "Observations", "UTXO"
        ].iter().map(|x| x.to_string()).collect::<Vec<String>>();
        table_data.push(headers.clone());

        let mut status = "Offline".to_string();
        let mut s3_hash = "".to_string();
        let mut api_hash = "".to_string();
        let mut peers = "".to_string();
        let mut transactions = "".to_string();
        let mut observations = "".to_string();
        let mut utxo = "".to_string();

        if let Ok(a) = d.about_node.lock() {
            status = "Online".to_string();
            self.network_healthy = true;
            peers = (a.num_active_peers + 1).to_string();
            transactions = a.total_accepted_transactions.to_string();
            if let Some(nmd) = a.latest_node_metadata.as_ref() {
                if let Some(d) = nmd.node_metadata().ok() {
                    d.version_info.map(|v| {
                        let c = v.executable_checksum;
                        let len = c.len();
                        let start = len - 9;
                        if start > 0 {
                            api_hash = c[start..].to_string();
                        }
                    });
                }
            }
        }
        if let Ok(a) = d.s3_hash.lock() {
            s3_hash = a.clone();
        }
        if let Ok(m) = d.metrics_hm.lock() {
            if let Some(t) = m.get("redgold_observation_total") {
                observations = t.to_string();
            }
            if let Some(u) = m.get("redgold_utxo_total") {
                utxo = u.to_string();
            }
        }

        let data = vec![
            nc.network.to_std_string(),
            nc.network.explorer_link(),
            status,
            s3_hash,
            api_hash,
            peers,
            transactions,
            observations,
            utxo
        ];
        table_data.push(data);
        text_table_advanced(ui, table_data, false, false, Some((1, vec!["Link".to_string()])), vec![]);
    }
}