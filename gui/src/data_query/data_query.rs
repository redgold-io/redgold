use crate::components::currency_input::supported_wallet_currencies;
use crate::dependencies::gui_depends::GuiDepends;
use crate::state::local_state::PricesPartyInfoAndDelta;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::explorer::{BriefTransaction, DetailedAddress};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::party::central_price::CentralPricePair;
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, CurrencyAmount, NetworkEnvironment, PublicKey, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use log::info;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DataQueryInfo<T> where T: ExternalNetworkResources + Clone + Send + 'static {
    pub external: T,
    pub address_infos: Arc<Mutex<HashMap<PublicKey, AddressInfo>>>,
    pub metrics: Arc<Mutex<Vec<(String, String)>>>,
    pub metrics_hm: Arc<Mutex<HashMap<String, String>>>,
    pub table_sizes: Arc<Mutex<Vec<(String, i64)>>>,
    pub about_node: Arc<Mutex<AboutNodeResponse>>,
    pub s3_hash: Arc<Mutex<String>>,
    pub party_data: Arc<Mutex<HashMap<PublicKey, PartyInternalData>>>,
    pub first_party: Arc<Mutex<PartyInternalData>>,
    pub price_map_usd_pair_incl_rdg: HashMap<SupportedCurrency, f64>,
    pub external_balances: Arc<Mutex<HashMap<(PublicKey, NetworkEnvironment, SupportedCurrency), CurrencyAmount>>>,
    pub external_tx: Arc<Mutex<HashMap<(PublicKey, NetworkEnvironment), Vec<ExternalTimedTransaction>>>>,
    pub detailed_address: Arc<Mutex<HashMap<PublicKey, Vec<DetailedAddress>>>>,
    pub total_incoming: Arc<Mutex<HashMap<PublicKey, i64>>>,
    pub total_outgoing: Arc<Mutex<HashMap<PublicKey, i64>>>,
    pub total_utxos: Arc<Mutex<HashMap<PublicKey, i64>>>,
    pub total_transactions: Arc<Mutex<HashMap<PublicKey, i64>>>,
    pub delta_24hr_external: HashMap<SupportedCurrency, f64>,
    // not yet used, would require a channel update on completion or collecting all the async in
    // future.
    pub recent_tx_sorted: Arc<Mutex<Vec<BriefTransaction>>>,
    pub party_nav: Arc<Mutex<f64>>,
    pub daily_one_year: Arc<Mutex<HashMap<SupportedCurrency, Vec<(i64, f64)>>>>
}


impl<T> DataQueryInfo<T> where T: ExternalNetworkResources + Clone + Send {
    pub fn refresh_swap_history(&self, p0: &PublicKey) {

    }
    pub fn party_keys(&self) -> Vec<PublicKey> {
        self.party_data.lock().unwrap().keys().cloned().collect::<Vec<PublicKey>>()
    }

    pub fn party(&self, party: Option<&PublicKey>) -> Option<PartyInternalData> {
        if let Some(p) = party {
            self.party_data.lock().unwrap().get(p).cloned()
        } else {
            Some(self.first_party.lock().unwrap().clone())
        }
    }
    pub fn party_events(&self, party: Option<&PublicKey>) -> Option<PartyEvents> {
        self.party(party)
            .and_then(|p| p.party_events.clone())
    }
    pub fn central_price_pair(&self, party: Option<&PublicKey>, cur: SupportedCurrency) -> Option<CentralPricePair> {
        self.party_events(party).and_then(|ev| ev.central_prices.get(&cur).cloned())
    }

    pub fn recent_tx<G>(
        &self, pubkey_filter: Option<&PublicKey>, limit: Option<usize>, include_ext: bool,
        currency_filter: Option<SupportedCurrency>, g: &G) -> Vec<BriefTransaction> where G: GuiDepends + Send + Clone + 'static {
        let addrs = self.detailed_address.lock().unwrap().clone();
        let mut brief = addrs.iter()
            .filter(|(pk, _)| pubkey_filter.map(|f| f == *pk).unwrap_or(true))
            .flat_map(|x| x.1.iter())
            .flat_map(|x| x.recent_transactions.clone())
            .collect::<Vec<BriefTransaction>>();
        let mut all = vec![];
        all.extend(brief);
        let parties = self.party_data.lock().unwrap().clone();
        if include_ext {
            for (_, ett) in self.external_tx.lock().unwrap().iter()
                .filter(|((pk, _), _)| pubkey_filter.map(|f| f == pk).unwrap_or(true))
                .filter(|((_, net), _)| &g.get_network() == net)
                .flat_map(|((pk, _), x)| x.iter().map(|ett| (pk.clone(), ett)))
            {
                let mut transaction = ett.to_brief();
                parties.iter().for_each(|(pk, party)| {
                    if let Some(pev) = party.party_events.as_ref() {
                        if let Some(ev) = pev.determine_event_type(&transaction.hash) {
                            transaction.address_event_type = Some(ev);
                        }
                    }
                });
                all.push(transaction);
            }
        }
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let limited: Vec<BriefTransaction> = all.iter().take(limit.unwrap_or(5)).map(|x| x.clone()).collect();

        limited
    }

    pub fn nav_usd(&self, nett: &NetworkEnvironment, filter_pk: Option<&PublicKey>) -> f64 {
        let mut nav = 0.0;
        if let Some(b) = self.external_balances.lock().ok() {
            for ((pk, net, cur), amt) in b.iter() {
                if *net != *nett {
                    continue
                }
                if let Some(filter_pk) = filter_pk {
                    if filter_pk != pk {
                        continue
                    }
                }
                if let Some(price) = self.price_map_usd_pair_incl_rdg.get(&cur) {
                    nav += amt.to_fractional() * price;
                }
            }
        }
        let rdg_nav = self.rdg_nav_usd(filter_pk);
        nav += rdg_nav;
        nav
    }

    pub fn nav_usd_by_currency(&self, nett: &NetworkEnvironment, filter_pk: Option<&PublicKey>) -> HashMap<SupportedCurrency, f64> {
        let mut hm = HashMap::new();
        if let Some(b) = self.external_balances.lock().ok() {
            for ((pk, net, cur), amt) in b.iter() {
                if *net != *nett {
                    continue
                }
                if let Some(filter_pk) = filter_pk {
                    if filter_pk != pk {
                        continue
                    }
                }
                if let Some(price) = self.price_map_usd_pair_incl_rdg.get(&cur) {
                    let usd_amt = amt.to_fractional() * price;
                    let current = hm.get(cur).unwrap_or(&0.0);
                    hm.insert(cur.clone(), current + usd_amt);
                }
            }
        }
        let rdg_nav = self.rdg_nav_usd(filter_pk);
        hm.insert(SupportedCurrency::Redgold, rdg_nav);
        hm
    }

    fn rdg_nav_usd(&self, filter_pk: Option<&PublicKey>) -> f64 {
        let mut total = 0i64;
        let rdg_price = self.price_map_usd_pair_incl_rdg.get(&SupportedCurrency::Redgold).unwrap_or(&100.0);
        if let Some(ai) = self.address_infos.lock().ok() {
            for (pk, info) in ai.iter() {
                if let Some(filter_pk) = filter_pk {
                    if filter_pk != pk {
                        continue
                    }
                }
                total += info.balance
            }
        }
        let rdg_nav = CurrencyAmount::from(total).to_fractional() * rdg_price;
        rdg_nav
    }

    pub fn balance_totals(&self, nett: &NetworkEnvironment, filter_pk: Option<&PublicKey>) -> HashMap<SupportedCurrency, f64> {
        let mut totals = HashMap::new();
        if let Some(b) = self.external_balances.lock().ok() {
            for ((pk, net, cur), amt) in b.iter() {
                if *net != *nett {
                    continue
                }
                if let Some(filter_pk) = filter_pk {
                    if filter_pk != pk {
                        continue
                    }
                }
                totals
                    .entry(cur.clone())
                    .and_modify(|e| *e += amt.to_fractional()).or_insert(amt.to_fractional());
            }
        }
        let mut total = 0i64;
        if let Some(ai) = self.address_infos.lock().ok() {
            for (pk, info) in ai.iter() {
                if let Some(filter_pk) = filter_pk {
                    if filter_pk != pk {
                        continue
                    }
                }
                total += info.balance;
            }
        }
        totals.insert(SupportedCurrency::Redgold, CurrencyAmount::from(total).to_fractional());
        totals
    }

    pub fn new(t: &T) -> Self {
        Self {
            external: t.clone(),
            address_infos: Arc::new(Mutex::new(Default::default())),
            metrics: Arc::new(Mutex::new(vec![])),
            metrics_hm: Arc::new(Mutex::new(Default::default())),
            table_sizes: Arc::new(Mutex::new(vec![])),
            about_node: Arc::new(Mutex::new(Default::default())),
            s3_hash: Arc::new(Mutex::new("".to_string())),
            party_data: Arc::new(Mutex::new(Default::default())),
            first_party: Arc::new(Mutex::new(Default::default())),
            price_map_usd_pair_incl_rdg: Default::default(),
            external_balances: Arc::new(Mutex::new(Default::default())),
            external_tx: Arc::new(Mutex::new(Default::default())),
            detailed_address: Arc::new(Mutex::new(Default::default())),
            total_incoming: Arc::new(Mutex::new(Default::default())),
            total_outgoing: Arc::new(Mutex::new(Default::default())),
            total_utxos: Arc::new(Mutex::new(Default::default())),
            total_transactions: Arc::new(Mutex::new(Default::default())),
            delta_24hr_external: Default::default(),
            recent_tx_sorted: Arc::new(Mutex::new(vec![])),
            party_nav: Arc::new(Mutex::new(0.0)),
            daily_one_year: Arc::new(Mutex::new(Default::default())),
        }
    }

    pub fn refresh_all_pk<G>(&self, pk: &PublicKey, g: &G) where G: GuiDepends + Send + Clone + 'static  {
        self.refresh_pks(vec![pk], g);
        self.refresh_external_tts(vec![pk], g);
        self.refresh_detailed_address_pks(vec![pk], g);
        self.refresh_external_balances(vec![pk], g, &self.external, &g.get_network());
    }

    pub fn refresh_pks<G>(&self, pks: Vec<&PublicKey>, g: &G) where G: GuiDepends + Send + Clone + 'static {
        for pk in pks {
            let arc = self.address_infos.clone();
            let pk = pk.clone();
            let g2 = g.clone();
            g.spawn(async move {
                let address_info = g2.get_address_info(&pk).await.log_error();
                if let Ok(address_info) = address_info {
                    let mut ai = arc.lock().unwrap();
                    ai.insert(pk, address_info);
                }
            });
        }
    }


    pub fn refresh_external_tts<G>(&self, pks: Vec<&PublicKey>, g: &G) where G: GuiDepends + Send + Clone + 'static {
        for pk in pks {
            let arc = self.external_tx.clone();
            let pk = pk.clone();
            let mut g2 = g.clone();
            g.spawn(async move {
                let mut txs = g2.get_external_tx(&pk, SupportedCurrency::Bitcoin).await.log_error().unwrap_or_default();
                let tx_eth = g2.get_external_tx(&pk, SupportedCurrency::Ethereum).await.log_error().unwrap_or(vec![]);
                txs.extend(tx_eth);
                info!("Refresh external txs for pk: {} count: {}", pk, txs.len());

                arc.lock().unwrap().insert((pk, g2.get_network().clone()), txs);
            });
        }
    }


    pub fn refresh_detailed_address_pks<G>(&self, pks: Vec<&PublicKey>, g: &G) where G: GuiDepends + Send + Clone + 'static {
        for pk in pks {
            let arc = self.detailed_address.clone();
            let total_incoming = self.total_incoming.clone();
            let total_outgoing = self.total_outgoing.clone();
            let total_utxos = self.total_utxos.clone();
            let total_transactions = self.total_transactions.clone();
            let pk = pk.clone();
            let g2 = g.clone();
            g.spawn(async move {
                let address_info = g2.get_detailed_address(&pk).await.log_error();
                if let Ok(address_info) = address_info {
                    let mut inc = total_incoming.lock().unwrap();
                    inc.insert(pk.clone(), 0);
                    let mut out = total_outgoing.lock().unwrap();
                    out.insert(pk.clone(), 0);
                    let mut utxos = total_utxos.lock().unwrap();
                    utxos.insert(pk.clone(), 0);
                    let mut total = total_transactions.lock().unwrap();
                    total.insert(pk.clone(), 0);
                    for ai in &address_info {
                        let updated = inc.get(&pk).map(|v| v.clone() + ai.incoming_count).unwrap_or(ai.incoming_count);
                        inc.insert(pk.clone(), updated);
                        let updated = out.get(&pk).map(|v| v.clone() + ai.outgoing_count).unwrap_or(ai.outgoing_count);
                        out.insert(pk.clone(), updated);
                        let updated = utxos.get(&pk).map(|v| v.clone() + ai.total_utxos).unwrap_or(ai.total_utxos);
                        utxos.insert(pk.clone(), updated);
                        let updated = total.get(&pk).map(|v| v.clone() + ai.total_count).unwrap_or(ai.total_count);
                        total.insert(pk.clone(), updated);
                    }
                    let mut ai = arc.lock().unwrap();
                    ai.insert(pk, address_info);
                }
            });
        }
    }

    pub fn refresh_external_balances<G,E>(&self, pks: Vec<&PublicKey>, g: &G, e: &E, network: &NetworkEnvironment)
    where G: GuiDepends + Send + Clone + 'static, E: ExternalNetworkResources + Clone + Send + 'static {
        for pk in pks {
            for cur in supported_wallet_currencies() {
                if cur == SupportedCurrency::Redgold {
                    continue
                }
                let arc = self.external_balances.clone();
                let pk = pk.clone();
                let e2 = e.clone();
                let n = network.clone();
                let c2 = cur.clone();
                g.spawn(async move {
                    let address_info = e2.get_balance_no_cache(&n, &c2, &pk).await;
                    if let Ok(amt) = address_info {
                        let mut ai = arc.lock().unwrap();
                        ai.insert((pk, n, c2), amt);
                    }
                });
            }
        }
    }

    pub fn refresh_party_data<G>(&self, g: &G) where G: GuiDepends + Send + Clone + 'static
    {
        let arc = self.party_data.clone();
        let arc2 = self.first_party.clone();
        let nav = self.party_nav.clone();
        let pm = self.price_map_usd_pair_incl_rdg.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let party_data = g2.party_data().await.log_error().map(|mut r| {
                r.iter_mut().for_each(|(k, v)| {
                    v.party_events.as_mut().map(|pev| {
                        pev.portfolio_request_events.enriched_events = Some(pev.portfolio_request_events.calculate_current_fulfillment_by_event());
                    });
                });
                r
            });
            if let Ok(party_data) = party_data {
                if let Some(pd) = party_data.iter().next().clone() {
                    let mut a2 = arc2.lock().unwrap();
                    let mut data = pd.1.clone();
                    let mut total = 0.0;
                    if let Some(bm) = data.party_events.as_ref()
                        .map(|pev| pev.balance_map.clone()) {
                        for (k, v) in bm.iter() {
                            if k != &SupportedCurrency::Redgold {
                                pm.get(&k).map(|price| {
                                    total += v.to_fractional() * price;
                                });
                            }
                        }
                    }
                    *nav.lock().unwrap() = total;
                    *a2 = data;
                }
                let mut ai = arc.lock().unwrap();
                *ai = party_data;
            }
        });
    }

    pub fn load_party_data_and_prices(&mut self, prices_party_info_and_delta: PricesPartyInfoAndDelta)
    {
        *self.daily_one_year.lock().unwrap() = prices_party_info_and_delta.daily_one_year.clone();
        self.price_map_usd_pair_incl_rdg = prices_party_info_and_delta.prices.clone();
        self.delta_24hr_external = prices_party_info_and_delta.delta_24hr.clone();
        let arc = self.party_data.clone();
        let arc2 = self.first_party.clone();
        let nav = self.party_nav.clone();
        let pm = self.price_map_usd_pair_incl_rdg.clone();

        let mut party_data = prices_party_info_and_delta.party_info.clone();
        party_data.iter_mut().for_each(|(k, v)| {
            v.party_events.as_mut().map(|pev| {
                pev.portfolio_request_events.enriched_events = Some(pev.portfolio_request_events.calculate_current_fulfillment_by_event());
            });
        });

        if let Some(pd) = party_data.iter().next().clone() {
            let mut a2 = arc2.lock().unwrap();
            let mut data = pd.1.clone();
            let mut total = 0.0;
            if let Some(bm) = data.party_events.as_ref()
                .map(|pev| pev.balance_map.clone()) {
                for (k, v) in bm.iter() {
                    if k != &SupportedCurrency::Redgold {
                        pm.get(&k).map(|price| {
                            total += v.to_fractional() * price;
                        });
                    }
                }
            }
            *nav.lock().unwrap() = total;
            *a2 = data;
        }
        let mut ai = arc.lock().unwrap();
        *ai = party_data;
    }

    pub fn refresh_network_info<G>(&self, g: &G) where G: GuiDepends + Send + Clone + 'static {
        let arc = self.metrics.clone();
        let hm = self.metrics_hm.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let metrics = g2.metrics().await.log_error();
            if let Ok(metrics) = metrics {
                let mut hashmap = HashMap::new();
                for (k, v) in metrics.iter() {
                    hashmap.insert(k.clone(), v.clone());
                }
                let mut mghm = hm.lock().unwrap();
                *mghm = hashmap;
                let mut ai = arc.lock().unwrap();
                *ai = metrics;

            }
        });
        let arc = self.table_sizes.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let table_sizes = g2.table_sizes().await.log_error();
            if let Ok(table_sizes) = table_sizes {
                let mut ai = arc.lock().unwrap();
                *ai = table_sizes;
            }
        });

        let arc = self.about_node.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let about_node = g2.about_node().await.log_error();
            if let Ok(about_node) = about_node {
                let mut ai = arc.lock().unwrap();
                *ai = about_node;
            }
        });

        let arc = self.s3_hash.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let s3_hash = g2.s3_checksum().await.log_error().unwrap_or("error".to_string());
            let mut ai = arc.lock().unwrap();
            *ai = s3_hash;
        });

    }

}