use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::GuiDepends;
use eframe::egui;
use eframe::egui::{Color32, RichText, Ui};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::structs::{NetworkEnvironment, PublicKey, SupportedCurrency};

use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::explorer::BriefTransaction;
use redgold_schema::conf::local_stored_state::LocalStoredState;
use redgold_schema::util::dollar_formatter::{format_dollar_amount, format_dollar_amount_with_prefix};
use crate::common::data_item;
use crate::components::balance_table::balance_table;
use crate::components::tables::{table_nonetype, text_table_advanced};
use crate::components::transaction_table::TransactionTable;

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
    pub recent_tx: TransactionTable
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            last_query_started_time: None,
            ran_once: false,
            network_healthy: false,
            recent_tx: Default::default(),
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
        lss: &LocalStoredState
    ) where
        G: GuiDepends + Send + Clone + 'static,
        E: ExternalNetworkResources + Send + Clone + 'static
    {

        ui.horizontal(|ui| {
            ui.heading("Home");
            let nav_usd = d.nav_usd(&nc.network, None);
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
                d.refresh_pks(loaded_pks.clone(), g);
                d.refresh_detailed_address_pks(loaded_pks.clone(), g)
            }
            d.refresh_party_data(g);
            if !self.ran_once {
                self.ran_once = true;
                d.refresh_external_balances(loaded_pks.clone(), g, e, &nc.network);
            }
        }
        self.network_stats_table(ui, d, nc);

        ui.separator();

        balance_table(ui, d, &nc, None, None, None, Some("home_balance".to_string()));

        self.recent_tx.rows = d.recent_tx(None, None, false, None);
        ui.separator();
        self.recent_tx.full_view::<E>(ui, &nc.network, d, None);
        ui.separator();

        let headers =
            vec!["Servers", "Public Keys", "XPubs", "Mnemonics", "Private Keys", "Contacts",
                 "Identities", "Watched", "Addresses"]
                .iter().map(|x| x.to_string()).collect::<Vec<String>>();

        let mut table_data: Vec<Vec<String>> = vec![];
        table_data.push(headers.clone());
        let row = vec![
            lss.servers.as_ref().map(|x| x.len()).unwrap_or(0).to_string(),
            loaded_pks.len().to_string(),
            // lss.xpubs.len().to_string(),
            lss.keys.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
            lss.mnemonics.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
            lss.private_keys.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
            // lss.contacts.len().to_string(),
            // lss.identities.len().to_string(),
            // lss.watched_address.len().to_string(),
            lss.contacts.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
            lss.identities.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
            lss.watched_address.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
            lss.saved_addresses.as_ref().map(|m| m.len()).unwrap_or(0).to_string()
        ];
        table_data.push(row);
        text_table_advanced(ui, table_data, false, false, None, vec![], table_nonetype());

    }

    fn network_stats_table<T>(&mut self, ui: &mut Ui, d: &DataQueryInfo<T>, nc: &NodeConfig)
    where T: ExternalNetworkResources + Clone + Send
    {
        let mut table_data: Vec<Vec<String>> = vec![];
        let headers = vec![
            "Network", "Explorer", "Status", "S3 Hash", "API Hash", "Peers",
            "Transactions", "Observations", "UTXO", "Party External NAV", "Party Events"
        ].iter().map(|x| x.to_string()).collect::<Vec<String>>();
        table_data.push(headers.clone());

        let mut status = "Offline".to_string();
        let mut s3_hash = "".to_string();
        let mut api_hash = "".to_string();
        let mut peers = "".to_string();
        let mut transactions = "".to_string();
        let mut observations = "".to_string();
        let mut utxo = "".to_string();
        let mut party_nav_usd = "".to_string();
        let mut party_events = "".to_string();
        if let Ok(a) = d.party_nav.lock() {
            party_nav_usd = format_dollar_amount_with_prefix(a.clone());
        }
        if let Ok(m) = d.metrics.lock() {
            if let Some((_, v)) = m.iter().filter(|(k, v)| k.starts_with("redgold_party_num_events"))
                .next() {
                party_events = v.clone()
            }
        }

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
            utxo,
            party_nav_usd,
            party_events
        ];
        table_data.push(data);
        text_table_advanced(ui, table_data, false, false, Some((1, vec!["Link".to_string()])), vec![], table_nonetype());
    }
}