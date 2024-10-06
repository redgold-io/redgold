use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, PublicKey, SupportedCurrency};
use crate::dependencies::gui_depends::GuiDepends;

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct DataQueryInfo {
    pub address_infos: Arc<Mutex<HashMap<PublicKey, AddressInfo>>>,
    pub metrics: Arc<Mutex<Vec<(String, String)>>>,
    pub table_sizes: Arc<Mutex<Vec<(String, i64)>>>,
    pub about_node: Arc<Mutex<AboutNodeResponse>>,
    pub party_data: Arc<Mutex<HashMap<PublicKey, PartyInternalData>>>,
    // TODO: Implement
    pub price_map_usd_pair: HashMap<SupportedCurrency, f64>,
}


impl DataQueryInfo {

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

    pub fn refresh_party_data<G>(&self, g: &G) where G: GuiDepends + Send + Clone + 'static
    {
        let arc = self.party_data.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let party_data = g2.party_data().await.log_error();
            if let Ok(party_data) = party_data {
                let mut ai = arc.lock().unwrap();
                *ai = party_data;
            }
        });
    }
    pub fn refresh_network_info<G>(&self, g: &G) where G: GuiDepends + Send + Clone + 'static {
        let arc = self.metrics.clone();
        let g2 = g.clone();
        g.spawn(async move {
            let metrics = g2.metrics().await.log_error();
            if let Ok(metrics) = metrics {
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

    }

}