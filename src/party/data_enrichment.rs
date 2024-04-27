use redgold_schema::structs::{CurrencyAmount, ErrorInfo, PartyData, PartyInfo, PublicKey, SupportedCurrency, Transaction};
use rocket::serde::{Deserialize, Serialize};
use std::collections::HashMap;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::eth::example::EthHistoricalClient;
use redgold_schema::{RgResult, SafeOption, structs};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::party::address_event::AddressEvent;
use crate::party::party_stream::{PartyEvents, TransactionWithObservationsAndPrice};
use crate::party::party_watcher::PartyWatcher;
use crate::party::price_query::PriceDataPointUsdQuery;
use crate::scrape::external_networks::ExternalNetworkData;

#[derive(Clone, Serialize, Deserialize)]
pub struct PartyInternalData {
    pub party_info: PartyInfo,
    pub network_data: HashMap<SupportedCurrency, ExternalNetworkData>,
    pub internal_data: Vec<Transaction>,
    // Technically network data / internal data above transactions are redundant in light of the
    // below field, can remove maybe later, but this is easy to use for now
    pub address_events: Vec<AddressEvent>,
    pub price_data: PriceDataPointUsdQuery,
    pub party_events: Option<PartyEvents>,
}

impl PartyInternalData {
    pub fn to_party_data(&self) -> PartyData {
        PartyData {
            json_party_internal_data: Some(self.json_or())
        }
    }

    pub fn not_debug(&self) -> bool {
        self.party_info.initiate.as_ref()
            .map(|p| p.purpose())
            .filter(|p| p != &structs::PartyPurpose::DebugPurpose)
            .is_some()
    }
}

impl PartyWatcher {

    pub async fn enrich_prepare_data(&self, active: Vec<PartyInfo>) -> Result<HashMap<PublicKey, PartyInternalData>, ErrorInfo> {
        let seeds = self.relay.node_config.seeds_now_pk();
        let mut shared_data = HashMap::new();
        for party in active {
            let pk = party.host_public_key().expect("pk missing");
            let prior_data = self.relay.ds.multiparty_store.party_data(&pk).await?
                .and_then(|pd| pd.json_party_internal_data)
                .and_then(|pid| pid.json_from::<PartyInternalData>().ok());

            let mut price_data = prior_data
                .as_ref()
                .map(|pd| pd.price_data.clone()).unwrap_or(PriceDataPointUsdQuery::new());

            // No filter is required here
            let btc = self.get_public_key_btc_data(&pk).await?;
            let max_eth_block = prior_data.as_ref().map(|pd| pd.network_data.get(&SupportedCurrency::Ethereum)
                .and_then(|d| d.max_block.as_ref())
            ).flatten().cloned();
            let eth = self.get_public_key_eth_data(&pk, max_eth_block).await?;
            let mut hm = HashMap::new();
            hm.insert(SupportedCurrency::Bitcoin, btc);
            hm.insert(SupportedCurrency::Ethereum, eth);
            // Change to time filter query to get prior data.
            let mut txs = self.relay.ds.transaction_store.get_all_tx_for_address(&pk.address()?, 1e9 as i64, 0).await?;
            txs.sort_by_key(|t| t.time().expect("time missing").clone());
            let mut address_events = vec![];
            for t in &txs {
                let h = t.hash_or();
                let obs = self.relay.ds.observation.select_observation_edge(&h).await?;
                let txo = TransactionWithObservationsAndPrice {
                    tx: t.clone(),
                    observations: obs,
                    price_usd: None,
                };
                let ae = AddressEvent::Internal(txo);
                address_events.push(ae);
            }

            address_events.sort_by(|a, b| {
                a.time(&seeds).cmp(&b.time(&seeds))
            });

            price_data.enrich_address_events(&mut address_events, &self.relay.ds).await?;

            let pid = PartyInternalData {
                party_info: party.clone(),
                network_data: hm,
                internal_data: txs,
                address_events,
                price_data,
                party_events: None,
            };

            shared_data.insert(pk.clone(), pid.clone());
        }
        Ok(shared_data)
    }

    // This is backed by a database, so the query parameter isn't really necessary here
    pub async fn get_public_key_btc_data(&self, pk: &PublicKey) -> RgResult<ExternalNetworkData> {
        let btc = self.relay.btc_wallet(pk)?;
        let all_tx = btc.get_all_tx()?;
        let raw_balance = btc.get_wallet_balance()?.confirmed;
        let amount = CurrencyAmount::from_btc(raw_balance as i64);
        let end = ExternalNetworkData {
            pk: pk.clone(),
            transactions: all_tx.clone(),
            balance: amount,
            currency: SupportedCurrency::Bitcoin,
            max_ts: all_tx.iter().flat_map(|t| t.timestamp).max(),
            max_block: None,
        };
        Ok(end)
    }

    pub async fn get_public_key_eth_data(&self, pk: &PublicKey, start_block: Option<u64>) -> RgResult<ExternalNetworkData> {
        let eth = EthHistoricalClient::new(&self.relay.node_config.network).ok_msg("eth client creation")??;
        let eth_addr = pk.to_ethereum_address_typed()?;
        let amount = eth.get_balance_typed(&eth_addr).await?;
        let eth_addr_str = eth_addr.render_string()?;
        let all_tx= eth.get_all_tx(&eth_addr_str, start_block).await?;
        let end = ExternalNetworkData {
            pk: pk.clone(),
            transactions: all_tx.clone(),
            balance: amount,
            currency: SupportedCurrency::Ethereum,
            max_ts: None,
            max_block: all_tx.iter().flat_map(|t| t.block_number).max()
        };
        Ok(end)
    }

}
