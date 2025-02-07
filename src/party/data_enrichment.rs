use crate::party::party_watcher::PartyWatcher;
use crate::party::price_query::PriceDataPointQueryImpl;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::party::address_event::{AddressEvent, TransactionWithObservationsAndPrice};
use redgold_schema::party::external_data::ExternalNetworkData;
use redgold_schema::party::external_data::PriceDataPointUsdQuery;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{Address, ErrorInfo, PublicKey, SupportedCurrency};
use redgold_schema::{RgResult, SafeOption};
use std::collections::HashMap;
use itertools::Itertools;
use redgold_schema::parties::{PartyInfo, PartyInstance, PartyMetadata};

impl<T> PartyWatcher<T> where T: ExternalNetworkResources + Send {

    pub async fn enrich_prepare_data(
        &self,
        metadata: PartyMetadata
    ) -> Result<HashMap<PublicKey, PartyInternalData>, ErrorInfo> {
        let mut data = self.relay.ds.config_store
            .get_json::<HashMap<PublicKey, PartyInternalData>>("party_internal_data").await?
            .unwrap_or_default();

        let seeds = self.relay.node_config.seeds_now_pk();
        let group_by = metadata.group_by_proposer();


        for (pk, pmd) in group_by.iter() {

            let mut party_internal_data = data.get(&pk).cloned().unwrap_or_default();
            let earliest_time = pmd.earliest_time();

            for party_instance in pmd.instances.iter() {

                let this_instance_address = party_instance.address.safe_get_msg("address")?;

                if party_instance.currency() == SupportedCurrency::Bitcoin {
                    let mut btc = self.get_public_key_btc_data(&this_instance_address).await?;
                    let prior = party_internal_data.network_data.get(&SupportedCurrency::Bitcoin)
                        .cloned()
                        .unwrap_or_default();
                    btc.transactions.extend(prior.transactions);
                    btc.transactions = btc.transactions.iter().unique_by(|t| t.tx_id.clone()).collect_vec();
                }
                if party_instance.currency() == SupportedCurrency::Ethereum {
                    let mut eth = self.get_public_key_eth_data(&this_instance_address, None).await?;
                    let prior = party_internal_data.network_data.get(&SupportedCurrency::Ethereum)
                        .cloned()
                        .unwrap_or_default();
                    eth.transactions.extend(prior.transactions);
                    eth.transactions = eth.transactions.iter().unique_by(|t| t.tx_id.clone()).collect_vec();
                }
                if party_instance.currency() == SupportedCurrency::Redgold {

                    let mut txs = self.relay.ds.transaction_store
                        .get_all_tx_for_address(&this_instance_address, 1e9 as i64, 0)
                        .await?;
                    let mut address_events = vec![];
                    for t in &txs {
                        let h = t.hash_or();
                        let obs = self.relay.ds.observation.select_observation_edge(&h).await?;
                        let txo = TransactionWithObservationsAndPrice {
                            tx: t.clone(),
                            observations: obs,
                            price_usd: None,
                            all_relevant_prices_usd: Default::default(),
                            queried_address: this_instance_address.clone(),
                        };
                        let ae = AddressEvent::Internal(txo);
                        address_events.push(ae);
                    }
                    let prior = party_internal_data.internal_data.clone();
                    txs.extend(prior);
                    txs = txs.iter().unique().collect_vec();
                    txs.sort_by_key(|t| t.time().unwrap_or(0));
                    party_internal_data.internal_data = txs;
                    address_events.extend(party_internal_data.internal_address_events.clone());
                    party_internal_data.address_events = address_events.iter().unique();
                }

            }

            let mut new_events = party_internal_data.internal_address_events.clone();
            for (cur, nd) in party_internal_data.network_data.iter() {
                for tx in nd.transactions.iter() {
                    new_events.push(AddressEvent::External(tx.clone()));
                }
            }
            new_events.sort_by(|a, b| {
                a.time(&seeds).cmp(&b.time(&seeds))
            });


            // Filter out all orders before initiation period (testing mostly.)
            // info!("enrich data Address events len: {} party start {}", address_events.len(), party_start);
            new_events.retain(|ae| {
                ae.time(&seeds).unwrap_or(0) >= earliest_time
            });
            party_internal_data.price_data.enrich_address_events(&mut new_events, &self.relay.ds, &self.external_network_resources).await?;

            // info!("events with prices: {}", address_events.json_or());
            party_internal_data.price_data.daily_enrichment(&self.external_network_resources, &self.relay.ds).await?;

            data.insert(pk.clone(), party_internal_data.clone());
        }
        Ok(data)
    }

    // This is backed by a database, so the query parameter isn't really necessary here
    pub async fn get_public_key_btc_data(&self, address: &Address) -> RgResult<ExternalNetworkData> {
        // let arc = self.relay.btc_wallet(pk).await?;
        // let btc = arc.lock().await;
        // let all_tx = btc.get_all_tx()?;
        let all_tx = self.external_network_resources
            .get_all_tx_for_address(address, SupportedCurrency::Bitcoin, None).await?;
        // let raw_balance = btc.get_wallet_balance()?.confirmed;
        // let amount = CurrencyAmount::from_btc(raw_balance as i64);
        let end = ExternalNetworkData {
            address: address.clone(),
            transactions: all_tx.clone(),
            // balance: amount,
            currency: SupportedCurrency::Bitcoin,
            max_ts: all_tx.iter().flat_map(|t| t.timestamp).max(),
            max_block: None,
        };
        Ok(end)
    }

    pub async fn get_public_key_eth_data(&self, address: &Address, start_block: Option<u64>) -> RgResult<ExternalNetworkData> {
        // let eth = EthHistoricalClient::new(&self.relay.node_config.network).ok_msg("eth client creation")??;
        // let eth_addr = pk.to_ethereum_address_typed()?;
        // let amount = eth.get_balance_typed(&eth_addr).await?;
        // let eth_addr_str = eth_addr.render_string()?;

        // Ignoring for now to debug
        // let start_block_arg = None;
        // let start_block_arg = start_block;
        // let all_tx= eth.get_all_tx_with_retries(&eth_addr_str, start_block_arg, None, None).await?;
        let all_tx = self.external_network_resources.get_all_tx_for_address(address, SupportedCurrency::Ethereum, None).await?;
        let end = ExternalNetworkData {
            address: address.clone(),
            transactions: all_tx.clone(),
            // balance: amount,
            currency: SupportedCurrency::Ethereum,
            max_ts: None,
            max_block: all_tx.iter().flat_map(|t| t.block_number).max()
        };
        Ok(end)
    }

}
