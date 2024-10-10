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

        let headers = vec!["Denomination", "Redgold", "Bitcoin", "Ethereum", "Monero", "Solana", "USDT"]
            .iter().map(|x| x.to_string()).collect::<Vec<String>>();
        let mut table_data: Vec<Vec<String>> = vec![];
        table_data.push(headers.clone());
        let balances = d.balance_totals(&nc.network);
        let mut row = vec!["NAV".to_string()];
        let ordered_cur = Self::queryable_balances();
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
                    let str = format!("{:.2}%", d*100.0);
                    let (symbol_dir, color) = if *d > 0.0 {
                        (Self::up_symbol_str(), Color32::GREEN)
                    } else {
                        (Self::down_symbol_str(), Color32::RED)
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
        let nav_totals = d.nav_usd_by_currency(&nc.network);
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

        self.recent_tx.rows = {
            let addrs = d.detailed_address.lock().unwrap().clone();
            let mut brief = addrs.values()
                .flat_map(|x| x.iter()
                    .flat_map(|a| a.recent_transactions.clone())
                ).collect::<Vec<BriefTransaction>>();
            brief.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            brief.iter().take(5).map(|x| x.clone()).collect()
        };
        ui.separator();

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                self.recent_tx.view(ui, &nc.network);
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
                            d.total_incoming.lock().map(|t| t.clone()).unwrap_or(0).to_string(),
                            d.total_outgoing.lock().map(|t| t.clone()).unwrap_or(0).to_string()
                        ]);
                        text_table_advanced(ui, right_table, false, false, None, vec![], table_nonetype());


                        let mut right_table = vec![vec!["RDG UTXOs", "RDG Transactions"].iter().map(|x| x.to_string()).collect::<Vec<String>>()];
                        right_table.push(vec![
                            d.total_utxos.lock().map(|t| t.clone()).unwrap_or(0).to_string(),
                            d.total_transactions.lock().map(|t| t.clone()).unwrap_or(0).to_string()
                        ]);
                        text_table_advanced(ui, right_table, false, false, None, vec![], table_nonetype());
                    });
            });
        });

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
            lss.xpubs.as_ref().map(|m| m.len()).unwrap_or(0).to_string(),
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

    fn down_symbol_str() -> String {
        "⬇".to_string()
    }

    fn up_symbol_str() -> String {
        "⬆".to_string()
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