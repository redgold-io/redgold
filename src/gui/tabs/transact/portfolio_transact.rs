use std::collections::HashMap;
use std::str::FromStr;
use eframe::egui::{Color32, RichText, ScrollArea, TextEdit, Ui};
use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_gui::components::balance_table::balance_table;
use redgold_gui::components::combo_box::combo_box;
use redgold_gui::components::currency_input::{currency_combo_box, supported_wallet_currencies, CurrencyInputBox};
use redgold_gui::components::tx_progress::{TransactionProgressFlow, TransactionStage};
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_schema::ShortString;
use redgold_schema::structs::{AddressInfo, CurrencyAmount, PublicKey, SupportedCurrency};
use redgold_gui::components::tables::{table_nonetype, text_table_advanced};
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::party::portfolio::PortfolioRequestEventInstance;
use redgold_schema::util::dollar_formatter::format_dollar_amount_with_prefix;
use redgold_schema::util::times::ToTimeString;
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
    pub weight_input: String,
    pub tx: TransactionProgressFlow,
    pub party_identifier: PublicKey,
    pub show_balances: bool,
    pub liquidation_label: String,
    pub liquidation_tx: TransactionProgressFlow
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
            tx: Default::default(),
            party_identifier: Default::default(),
            show_balances: true,
            liquidation_label: "".to_string(),
            liquidation_tx: Default::default(),
        }
    }
}

#[derive(Clone, PartialEq, EnumIter, EnumString, Debug)]
pub enum PortfolioTransactSubTab {
    View,
    // Update,
    Create,
    Liquidate,
}

impl PortfolioState {

    pub fn port_subtabs(&mut self, ui: &mut Ui, pk: &PublicKey) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading(format!("{:?}", self.tab));
            for t in PortfolioTransactSubTab::iter() {
                if ui.button(format!("{:?}", t)).clicked() {
                    self.tab = t.clone();
                }
            }
            ui.checkbox(&mut self.show_balances, "Show Balances");
        });

        ui.separator();
    }
    pub fn view<G, E>(&mut self, ui: &mut Ui, pk: &PublicKey, g: &G, tsi: &TransactionSignInfo, nc: &NodeConfig,
                      d: &DataQueryInfo<E>,
                      csi: &TransactionSignInfo,
                      allowed: &Vec<XPubLikeRequestType>)
    where
        G: GuiDepends + Clone + Send + 'static,
        E: ExternalNetworkResources + Clone + Send + 'static
    {


        let addr = pk.address().unwrap();
        let addrs = g.to_all_address(pk);

        let keys = d.party_keys();
        if keys.is_empty() {
            ui.label(RichText::new("No party data found, network error").color(Color32::RED));
            return;
        }
        let ai = d.address_infos.lock().unwrap().get(pk).cloned();
        if ai.is_none() {
            ui.label(RichText::new("No address info found, network error or refresh required to get UTXOs").color(Color32::RED));
            return;
        }

        if !keys.contains(&self.party_identifier) {
            self.party_identifier = keys[0].clone();
        }
        let pev = d.party_events(Some(&self.party_identifier));

        if let Some(pev) = pev {
            let balances = pev.staking_balances(&addrs, Some(false), Some(true));
            if self.show_balances {
                let mut hm2 = HashMap::default();
                for c in supported_wallet_currencies() {
                    let bal = balances.get(&c)
                        .map(|c| c.to_fractional()).unwrap_or(0.0);
                    hm2.insert(c, bal);
                }
                balance_table(ui, d, &nc, None, Some(pk), Some(hm2), Some("port_balance".to_string()));
            }
            let map = pev.portfolio_request_events.enriched_events.clone().unwrap();
            let matching_portfolios = pev.portfolio_request_events.events
            .iter()
            .filter(|p| p.tx.input_addresses().contains(&addr))
                .map(|x| (map.get(&x.tx.hash_or()).cloned(), x.clone()))
            .collect_vec();

            self.port_subtabs(ui, pk);

            match self.tab {
                PortfolioTransactSubTab::View => {
                    if matching_portfolios.is_empty() {
                        ui.label("No portfolios found for this address");
                    } else {
                        let mut table_data = vec![];
                        let headers = Self::portfolio_row_headers();
                        table_data.push(headers);
                        for (fulfills, p) in matching_portfolios.iter() {
                            table_data.push(Self::portfolio_row(d, fulfills, p));
                        };
                        let network = g.get_network().clone();
                        let func = move |ui: &mut Ui, row: usize, col: usize, val: &String| {
                            if col == 4 {
                                ui.hyperlink_to(val.first_four_last_four_ellipses().unwrap_or("err".to_string()), network.explorer_hash_link(val.clone()));
                                return true
                            }
                            false
                        };
                        text_table_advanced(ui, table_data, false, false, None, vec![], Some(func));
                    }
                }
                PortfolioTransactSubTab::Create => {
                    self.create_portfolio(
                        ui, pk, g, tsi, &d.price_map_usd_pair_incl_rdg, &pev, ai.as_ref().unwrap(), csi, allowed);
                }
                PortfolioTransactSubTab::Liquidate => {
                    let mut label_to_instance = HashMap::new();
                    let labels = matching_portfolios.iter().map(|(_, p)| {
                        let tx_hash = p.tx.hash_hex();
                        let portfolio_name = p.portfolio_request.portfolio_id.as_ref().and_then(|pi| pi.name.clone()).unwrap_or("".to_string());
                        let label = format!("{} - {}", portfolio_name, tx_hash);
                        label_to_instance.insert(label.clone(), p.clone());
                        label
                    }).collect_vec();
                    let mut selected = self.portfolio_input_name.clone();
                    combo_box(
                        ui,
                        &mut selected,
                        "Portfolio",
                        labels,
                        self.liquidation_tx.locked(),
                        400.0,
                        Some("liquidation".to_string())
                    );
                    let event = self.liquidation_tx.view(ui, g, &tsi, csi, allowed);
                    if let Some(TransactionStage::NotCreated) = event.next_stage_transition_from {
                        let p = label_to_instance.get(&selected).unwrap();
                        // let ports = p.portfolio_request.portfolio_id.as_ref().unwrap().clone();
                        // let tx = g.tx_builder()
                        //     .with_input_address(&pk.address().unwrap())
                        //     .with_utxos(ai.as_ref().unwrap().utxo_entries.as_ref())
                        //     .unwrap()
                        //     .with_portfolio_liquidation(ports)
                        //     .build();
                        // self.liquidation_tx.with_built_rdg_tx(tx);
                    }
                }
            }
        }
    }

    fn portfolio_row_headers() -> Vec<String> {
        let mut headers = vec!["Name", "Start USD", "Value USD", "Created", "TX Hash"]
            .iter().map(|h| h.to_string()).collect_vec();
        // for cur in supported_wallet_currencies().iter() {
        //     headers.push(format!("{} Filled", cur.abbreviated()));
        //     headers.push(format!("{} Remaining", cur.abbreviated()));
        // };
        headers.push("Unfilled USD".to_string());
        headers
    }
    fn portfolio_row<E>(
        d: &DataQueryInfo<E>, fulfills: &Option<HashMap<SupportedCurrency, (f64, f64)>>, p: &PortfolioRequestEventInstance
    ) -> Vec<String>where E: ExternalNetworkResources + Clone + Send + 'static {
        let starting_value_usd = p.value_at_time;
        // let starting_p.value_at_time
        let create_time = p.time.to_time_string_shorter_no_seconds_am_pm();
        let tx_hash = p.tx.hash_hex();
        let portfolio_name = p.portfolio_request.portfolio_id.as_ref().and_then(|pi| pi.name.clone()).unwrap_or("".to_string());

        let mut row_data = vec![];

        row_data.push(portfolio_name);
        row_data.push(format_dollar_amount_with_prefix(starting_value_usd));
        let mut fbalances = fulfills.clone().unwrap_or_default();
        let mut total_cur_value = 0.0;
        fbalances.iter().map(|(c, (f, r))| {
            d.price_map_usd_pair_incl_rdg.get(c).map(|p| {
                total_cur_value += f * p;
            });
            d.price_map_usd_pair_incl_rdg.get(&SupportedCurrency::Redgold).map(|p| {
                total_cur_value += r * p;
            });
        }).count();
        row_data.push(format_dollar_amount_with_prefix(total_cur_value));
        row_data.push(create_time);
        row_data.push(tx_hash);

        let rdg_remaining_bal = fbalances
            .iter()
            .filter(|(k, _)| k != &&SupportedCurrency::Redgold)
            .map(|(_, (f, r))| r.clone()).sum::<f64>();
        //
        // for cur in supported_wallet_currencies().iter() {
        //     let (f, r) = fbalances.get(cur).unwrap_or(&(0.0, 0.0));
        //     row_data.push(format_dollar_amount_with_prefix(*f));
        //     row_data.push(format_dollar_amount_with_prefix(*r));
        // }
        row_data.push(format_dollar_amount_with_prefix(rdg_remaining_bal));
        row_data
    }


    fn create_portfolio<G>(
        &mut self,
        ui: &mut Ui,
        pk: &PublicKey,
        g: &G,
        ksi: &TransactionSignInfo,
        pm: &HashMap<SupportedCurrency, f64>,
        pev: &PartyEvents,
        ai: &AddressInfo,
        csi: &TransactionSignInfo,
        allowed: &Vec<XPubLikeRequestType>
    ) where G: GuiDepends + Clone + Send + 'static {
        let locked = self.tx.locked();
        ui.horizontal(|ui| {
            ui.label("Portfolio Name:");
            let mut locked_text = self.portfolio_input_name.clone();
            let mut text = &mut self.portfolio_input_name;
            if locked {
                text = &mut locked_text;
            }

            TextEdit::singleline(text).desired_width(150.0).show(ui);
            // TODO: Privacy options.
            // ui.checkbox(&mut ls.wallet.port.portfolio_input_name, "Editable");
        });
        let price_map = pm.clone();
        ui.horizontal(|ui| {
            ui.label("Stake Input");
            self.rdg_input.locked = locked;
            self.rdg_input.view(ui, &price_map);
        });
        ui.separator();
        if !locked {
            ui.horizontal(|ui| {
                ui.label("Add Portfolio Entry:");
                currency_combo_box(ui, &mut self.add_new_currency, "Id", supported_wallet_currencies(), false);
                ui.label("Weight:");
                TextEdit::singleline(&mut self.weight_input).desired_width(50.0).show(ui);
                if ui.button("Add").clicked() {
                    let new_row = PortfolioRow {
                        entry_type: PortfolioEntryType::Currency,
                        id: format!("{:?}", self.add_new_currency),
                        weight: self.weight_input.parse().unwrap(),
                        weight_normalized: 0.0,
                        editable: false,
                        nav_usd: 0.0,
                        nav_pair: 0.0,
                        fulfillment_imbalance_usd: 0.0,
                        fulfillment_imbalance_pair: 0.0,
                    };
                    self.port.rows.push(new_row);
                    self.port.normalize_weight_update();
                }
            });
        }

        let event = self.tx.view(ui, g, &ksi, csi, allowed);
        if event.reset {
            self.portfolio_input_name = "".to_string();
        }
        if event.next_stage {
            match self.tx.stage {
                TransactionStage::Created => {
                    let ports = self.port.rows.iter()
                        .filter(|r| { r.entry_type == PortfolioEntryType::Currency })
                        .map(|r| SupportedCurrency::from_str(&*r.id).map(|s| (s, r.weight)).ok())
                        .flatten()
                        .collect_vec();
                    let tx = g.tx_builder()
                        .with_input_address(&pk.address().unwrap())
                        .with_utxos(ai.utxo_entries.as_ref())
                        .unwrap()
                        .with_portfolio_request(
                            ports,
                            &CurrencyAmount::from_rdg(100_000),
                            &pev.key_address,
                            &self.portfolio_input_name
                        )
                        .build();
                    self.tx.with_built_rdg_tx(tx);
                }
                _ => {

                }
            }
        }
        ui.separator();
        ScrollArea::vertical().id_source("porttable")
            .max_height(250.0)
            .min_scrolled_height(150.0)
            .max_width(900.0)
            .min_scrolled_width(900.0)
            .auto_shrink(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    self.portfolio_table(ui, pk);
                });
            });


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


impl PortfolioState {

    pub fn portfolio_table(&mut self, ui: &mut Ui, pk: &PublicKey) {
        let data = self.port.rows.clone();

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
        let event = text_table_advanced(ui, data_str, true, false, None, vec![], table_nonetype());
        if let Some(index) = event.delete_row_id.as_ref() {
            self.port.rows.remove(index.clone());
            self.port.normalize_weight_update();
        }
    }
}


