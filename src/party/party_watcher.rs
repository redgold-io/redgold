use std::collections::HashMap;
use async_trait::async_trait;
use itertools::Itertools;
use log::info;
use rocket::form::validate::Contains;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::eth::example::EthHistoricalClient;
use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::{RgResult, SafeOption, util};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::party::all_parties::AllParties;
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, Hash, PartyInfo, PublicKey, SupportedCurrency, Transaction};
use crate::core::relay::Relay;
use crate::core::stream_handlers::{IntervalFold, IntervalFoldOrReceive};
use crate::party::party_stream::{AddressEvent, TransactionWithObservations};
use crate::scrape::external_networks::ExternalNetworkData;

// TODO: Future event streaming solution here

pub struct PartyWatcher {
    relay: Relay
}

#[derive(Clone)]
pub struct PartyInternalData {
    pub party_info: PartyInfo,
    pub network_data: HashMap<SupportedCurrency, ExternalNetworkData>,
    pub internal_data: Vec<Transaction>,
    // Technically network data / internal data above transactions are redundant in light of the
    // below field, can remove maybe later, but this is easy to use for now
    pub address_events: Vec<AddressEvent>
}

impl PartyWatcher {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }
    pub async fn tick(&self) -> RgResult<()> {
        let parties = self.relay.ds.multiparty_store.all_party_info_with_key().await?;
        let all_parties = AllParties::new(parties);
        let active = all_parties.active;
        let seeds = self.relay.node_config.seeds_now_pk();
        let mut shared_data = HashMap::new();
        for party in active {
            let pk = party.host_public_key().expect("pk missing");
            let prior_data = self.relay.ds.multiparty_store.party_data(&pk).await?;

            let btc = self.get_public_key_btc_data(&pk).await?;
            let eth = self.get_public_key_eth_data(&pk).await?;
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
                let txo = TransactionWithObservations {
                    tx: t.clone(),
                    observations: obs,
                };
                let ae = AddressEvent::Internal(txo);
                address_events.push(ae);
            }

            address_events.sort_by(|a, b| {
                a.time(&seeds).cmp(&b.time(&seeds))
            });

            let pid = PartyInternalData {
                party_info: party.clone(),
                network_data: hm,
                internal_data: txs,
                address_events,
            };


            shared_data.insert(pk.clone(), pid);
        }
        self.relay.external_network_shared_data.write(shared_data);

        Ok(())
    }

    pub async fn get_public_key_btc_data(&self, pk: &PublicKey) -> RgResult<ExternalNetworkData> {
        let btc = SingleKeyBitcoinWallet::new_wallet_db_backed(
            pk.clone(), self.relay.node_config.network.clone(), true,
            self.relay.node_config.env_data_folder().bdk_sled_path()
        )?;
        let all_tx = btc.get_all_tx()?;
        let raw_balance = btc.get_wallet_balance()?.confirmed;
        let amount = CurrencyAmount::from_btc(raw_balance as i64);
        let end = ExternalNetworkData {
            pk: pk.clone(),
            transactions: all_tx,
            balance: amount,
            currency: SupportedCurrency::Bitcoin,
        };
        Ok(end)
    }

    pub async fn get_public_key_eth_data(&self, pk: &PublicKey) -> RgResult<ExternalNetworkData> {
        let eth = EthHistoricalClient::new(&self.relay.node_config.network).ok_msg("eth client creation")??;
        let eth_addr = pk.to_ethereum_address_typed()?;
        let amount = eth.get_balance_typed(&eth_addr).await?;
        let eth_addr_str = eth_addr.render_string()?;
        let all_tx= eth.get_all_tx(&eth_addr_str).await?;
        let end = ExternalNetworkData {
            pk: pk.clone(),
            transactions: all_tx,
            balance: amount,
            currency: SupportedCurrency::Ethereum,
        };
        Ok(end)
    }


}

#[async_trait]
impl IntervalFold for PartyWatcher {
    async fn interval_fold(&mut self) -> RgResult<()> {
        self.tick().await.bubble_abort()?
    }
}