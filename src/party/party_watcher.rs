use crate::core::relay::Relay;
use crate::party::party_stream::PartyEventBuilder;
use crate::party::portfolio_request::PortfolioEventMethods;
use async_trait::async_trait;
use metrics::gauge;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common_no_wasm::stream_handlers::IntervalFold;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{NetworkEnvironment, PublicKey};
use redgold_schema::RgResult;
use std::collections::HashMap;
use tracing::info;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::monero::node_wrapper::PartySecretData;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::word_pass_support::{NodeConfigKeyPair, WordsPassNodeConfig};
use redgold_node_core::party_updates::creation::check_formations;
use redgold_schema::parties::PartyMetadata;

// TODO: Future event streaming solution here
#[derive(Clone)]
pub struct PartyWatcher<T> where T: ExternalNetworkResources + Send {
    pub relay: Relay,
    pub external_network_resources: T
}

impl<T> PartyWatcher<T> where T: ExternalNetworkResources + Send {
    pub fn new(relay: &Relay, t: T) -> Self {
        Self {
            relay: relay.clone(),
            external_network_resources: t,
        }
    }
    
    pub async fn tick(&mut self) -> RgResult<()> {


        // info!("Party watcher tick on node {}", self.relay.node_config.short_id().expect("Node ID"));
        let mut party_metadata = self.relay.ds.config_store
            .get_json::<PartyMetadata>("party_metadata").await?
            .unwrap_or(Default::default());
        let mut party_secrets = self.relay.ds.config_store
            .get_json::<PartySecretData>("party_secret").await?
            .unwrap_or(Default::default());

        let mut other_seeds = self.relay.node_config.non_self_seeds_pk();
        gauge!("redgold_party_initial_formation_non_self_peers").set(other_seeds.len() as f64);
        if self.relay.node_config.network == NetworkEnvironment::Local {
            other_seeds = self.relay.ds.peer_store.active_nodes(None).await.unwrap_or(vec![]);
        }
        if other_seeds.len() < 2 {
            info!("Not enough peers in local network for party formation");
            return Ok(())
        }

        let update_events = check_formations(
            &party_metadata,
            &self.external_network_resources,
            &self.relay.node_config.words().to_all_addresses_default(&self.relay.node_config.network)?,
            &other_seeds,
            &self.relay,
            &self.relay.node_config.public_key(),
            None,
            &self.relay.node_config.keypair().to_private_hex(),
            &self.relay.node_config.network,
            self.relay.node_config.words().clone()
        ).await.log_error().bubble_abort()?.ok();

        if let Some(u) = update_events {
            if let Some(m) = u.updated_metadata.as_ref() {
                party_metadata = m.clone();
                self.relay.ds.config_store.set_json("party_metadata", &party_metadata).await?;
            }
            for s in u.new_secrets.iter() {
                party_secrets.instances.push(s.clone());
            }
            self.relay.ds.config_store.set_json("party_secrets", &party_secrets).await?;
        }


        // TODO: Check for historical public keys associated with self node?;
        let active = party_metadata.active();

        gauge!("redgold_party_watcher_active_parties").set(active.len() as f64);
        gauge!("redgold_party_watcher_active_self_proposed_parties").set(party_metadata
            .active_proposed_by(&self.relay.node_config.public_key()).len() as f64);
        gauge!("redgold_party_watcher_total_parties").set(party_metadata.instances.len() as f64);

        let mut shared_data = self.enrich_prepare_data(party_metadata.clone()).await?;
        // TODO: self.merge_child_events
        self.calculate_party_stream_events(&mut shared_data).await?;
        if self.relay.node_config.enable_party_mode() {
            self.handle_order_fulfillment(&mut shared_data).await?;
        }
        // self.handle_key_rotations(&mut shared_data).await?;

        self.relay.external_network_shared_data.write(shared_data.clone()).await;
        if self.relay.node_config.enable_party_mode() {
        // info!("Party watcher tick num parties total {} active {}", party_metadata.clone().instances.len(), active.len());
            // self.tick_formations(&shared_data).await?;
            // info!("Completed party tick on node {}", self.relay.node_config.short_id().expect("Node ID"));
            // for (pk, pid) in shared_data.iter() {
            //     let mut pid2 = pid.clone();
            //     pid2.clear_sensitive();
            //     // info!("Party {} data {}", pk.hex(), pid2.json_or());
            // }
        }
        Ok(())
    }


    async fn calculate_party_stream_events(&self, data: &mut HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {
        for (k,v ) in data.iter_mut() {
            let mut pe = PartyEvents::new(&self.relay.node_config.network, &self.relay, v.metadata.address_by_currency());
            for e in v.address_events.iter() {
                pe.process_event(e).await?;
            }
            pe.calculate_update_portfolio_imbalance(&self.external_network_resources).await.log_error().bubble_abort()?.ok();
            pe.locally_fulfilled_orders = v.locally_fulfilled_orders.clone().unwrap_or(vec![]);
            v.party_events = Some(pe.clone());
            // let len = pe.unfulfilled_external_withdrawals.len();
            // if len  > 0 {
            //     info!("Party {} has unfulfilled external withdrawals {:?}", k.hex(), pe.unfulfilled_external_withdrawals.len());
            //     info!("pause here");
            // }
        }
        Ok(())
    }
}

#[async_trait]
impl<T> IntervalFold for PartyWatcher<T> where T: ExternalNetworkResources + Send + Sync + Clone {
    async fn interval_fold(&mut self) -> RgResult<()> {
        self.tick().await.log_error().bubble_abort()?.ok();
        Ok(())
    }
}