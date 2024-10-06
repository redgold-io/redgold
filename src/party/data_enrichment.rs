use redgold_schema::structs::{ErrorInfo, PartyInfo, PublicKey, SupportedCurrency};
use std::collections::HashMap;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::party::address_event::{AddressEvent, TransactionWithObservationsAndPrice};
use crate::party::party_watcher::PartyWatcher;
use redgold_schema::party::external_data::PriceDataPointUsdQuery;
use redgold_schema::party::external_data::ExternalNetworkData;
use redgold_schema::party::party_internal_data::PartyInternalData;
use crate::party::price_query::PriceDataPointQueryImpl;

impl<T> PartyWatcher<T> where T: ExternalNetworkResources + Send {

    pub async fn enrich_prepare_data(&self, active: Vec<PartyInfo>) -> Result<HashMap<PublicKey, PartyInternalData>, ErrorInfo> {
        let seeds = self.relay.node_config.seeds_now_pk();
        let mut shared_data = HashMap::new();
        for party in active {
            let pk = party.party_key.safe_get_msg("party key missing")?;
            let prior_data = self.relay.ds.multiparty_store.party_data(&pk).await?
                .and_then(|pd| pd.json_party_internal_data)
                .and_then(|pid| pid.json_from::<PartyInternalData>().ok());

            let mut price_data = prior_data
                .as_ref()
                .map(|pd| pd.price_data.clone()).unwrap_or(PriceDataPointUsdQuery::new());

            let prior_local_fulfilled = prior_data.as_ref()
                .and_then(|pd| pd.locally_fulfilled_orders.clone());

            // No filter is required here
            let btc = self.get_public_key_btc_data(&pk).await?;
            let max_eth_block = prior_data.as_ref().map(|pd| pd.network_data.get(&SupportedCurrency::Ethereum)
                .and_then(|d| d.max_block.as_ref())
            ).flatten().cloned();
            let eth = self.get_public_key_eth_data(&pk, max_eth_block).await?;
            let mut hm = HashMap::new();
            hm.insert(SupportedCurrency::Bitcoin, btc.clone());
            hm.insert(SupportedCurrency::Ethereum, eth.clone());
            // Change to time filter query to get prior data.
            let mut txs = self.relay.ds.transaction_store.get_all_tx_for_address(&pk.address()?, 1e9 as i64, 0)
                .await?;
            txs.sort_by_key(|t| t.time().expect("time missing").clone());
            let mut address_events = vec![];
            for t in &txs {
                let h = t.hash_or();
                let obs = self.relay.ds.observation.select_observation_edge(&h).await?;
                let txo = TransactionWithObservationsAndPrice {
                    tx: t.clone(),
                    observations: obs,
                    price_usd: None,
                    all_relevant_prices_usd: Default::default(),
                };
                let ae = AddressEvent::Internal(txo);
                address_events.push(ae);
            }
            for t in btc.transactions.iter() {
                address_events.push(AddressEvent::External(t.clone()));
            }
            for t in eth.transactions.iter() {
                address_events.push(AddressEvent::External(t.clone()));
            }

            address_events.sort_by(|a, b| {
                a.time(&seeds).cmp(&b.time(&seeds))
            });


            // Filter out all orders before initiation period (testing mostly.)

            let party_start = party.initiate.safe_get_msg("initiate")?.time.clone();
            // info!("enrich data Address events len: {} party start {}", address_events.len(), party_start);
            address_events.retain(|ae| {
                ae.time(&seeds).unwrap_or(0) >= party_start
            });
            // info!("enrich data Address events len after filter: {}", address_events.len());

            // info!("enrich data eth: {}", eth.json_or());
            // if btc.transactions.len() > 2 {
            //     info!("btc transactions exceed bp");
            //     info!("btc transactions exceed bp");
            // }

            price_data.enrich_address_events(&mut address_events, &self.relay.ds, &self.external_network_resources).await?;

            // info!("events with prices: {}", address_events.json_or());
            price_data.daily_enrichment(&self.external_network_resources, &self.relay.ds).await?;

            let pid = PartyInternalData {
                party_info: party.clone(),
                network_data: hm,
                internal_data: txs,
                address_events,
                price_data,
                party_events: None,
                locally_fulfilled_orders: prior_local_fulfilled,
            };

            shared_data.insert(pk.clone(), pid.clone());
        }
        Ok(shared_data)
    }

    // This is backed by a database, so the query parameter isn't really necessary here
    pub async fn get_public_key_btc_data(&self, pk: &PublicKey) -> RgResult<ExternalNetworkData> {
        // let arc = self.relay.btc_wallet(pk).await?;
        // let btc = arc.lock().await;
        // let all_tx = btc.get_all_tx()?;
        let all_tx = self.external_network_resources.get_all_tx_for_pk(pk, SupportedCurrency::Bitcoin, None).await?;
        // let raw_balance = btc.get_wallet_balance()?.confirmed;
        // let amount = CurrencyAmount::from_btc(raw_balance as i64);
        let end = ExternalNetworkData {
            pk: pk.clone(),
            transactions: all_tx.clone(),
            // balance: amount,
            currency: SupportedCurrency::Bitcoin,
            max_ts: all_tx.iter().flat_map(|t| t.timestamp).max(),
            max_block: None,
        };
        Ok(end)
    }

    pub async fn get_public_key_eth_data(&self, pk: &PublicKey, start_block: Option<u64>) -> RgResult<ExternalNetworkData> {
        // let eth = EthHistoricalClient::new(&self.relay.node_config.network).ok_msg("eth client creation")??;
        // let eth_addr = pk.to_ethereum_address_typed()?;
        // let amount = eth.get_balance_typed(&eth_addr).await?;
        // let eth_addr_str = eth_addr.render_string()?;

        // Ignoring for now to debug
        // let start_block_arg = None;
        // let start_block_arg = start_block;
        // let all_tx= eth.get_all_tx_with_retries(&eth_addr_str, start_block_arg, None, None).await?;
        let all_tx = self.external_network_resources.get_all_tx_for_pk(pk, SupportedCurrency::Ethereum, None).await?;
        let end = ExternalNetworkData {
            pk: pk.clone(),
            transactions: all_tx.clone(),
            // balance: amount,
            currency: SupportedCurrency::Ethereum,
            max_ts: None,
            max_block: all_tx.iter().flat_map(|t| t.block_number).max()
        };
        Ok(end)
    }

}
