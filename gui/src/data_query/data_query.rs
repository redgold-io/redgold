use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, CurrencyAmount, Hash, NetworkEnvironment, PublicKey, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use crate::components::currency_input::supported_wallet_currencies;
use crate::dependencies::gui_depends::GuiDepends;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DataQueryInfo<T> where T: ExternalNetworkResources + Clone + Send {
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
}


impl<T> DataQueryInfo<T> where T: ExternalNetworkResources + Clone + Send {

    pub fn nav_usd(&self, nett: &NetworkEnvironment) -> f64 {
        let mut nav = 0.0;
        if let Some(b) = self.external_balances.lock().ok() {
            for ((pk, net, cur), amt) in b.iter() {
                if *net != *nett {
                    continue
                }
                if let Some(price) = self.price_map_usd_pair_incl_rdg.get(&cur) {
                    nav += amt.to_fractional() * price;
                }
            }
        }

        nav
    }

    pub fn nav_usd_by_currency(&self, nett: &NetworkEnvironment) -> HashMap<SupportedCurrency, f64> {
        let mut hm = HashMap::new();
        if let Some(b) = self.external_balances.lock().ok() {
            for ((pk, net, cur), amt) in b.iter() {
                if *net != *nett {
                    continue
                }
                if let Some(price) = self.price_map_usd_pair_incl_rdg.get(&cur) {
                    let usd_amt = amt.to_fractional() * price;
                    let current = hm.get(cur).unwrap_or(&0.0);
                    hm.insert(cur.clone(), current + usd_amt);
                }
            }
        }

        hm
    }

    pub fn balance_totals(&self, nett: &NetworkEnvironment) -> HashMap<SupportedCurrency, f64> {
        let mut totals = HashMap::new();
        if let Some(b) = self.external_balances.lock().ok() {
            for ((pk, net, cur), amt) in b.iter() {
                if *net != *nett {
                    continue
                }
                totals
                    .entry(cur.clone())
                    .and_modify(|e| *e += amt.to_fractional()).or_insert(amt.to_fractional());
            }
        }
        let mut total = 0i64;
        if let Some(ai) = self.address_infos.lock().ok() {
            for (pk, info) in ai.iter() {
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
        }
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
        let g2 = g.clone();
        g.spawn(async move {
            let party_data = g2.party_data().await.log_error();
            if let Ok(party_data) = party_data {
                if let Some(pd) = party_data.iter().next().clone() {
                    let mut a2 = arc2.lock().unwrap();
                    *a2 = pd.1.clone();
                }
                let mut ai = arc.lock().unwrap();
                *ai = party_data;
            }
        });
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