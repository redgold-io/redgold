use std::collections::HashMap;
use async_trait::async_trait;
use log::info;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::RgResult;
use redgold_schema::party::all_parties::AllParties;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::PublicKey;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;
use crate::party::data_enrichment::PartyInternalData;
use crate::party::party_stream::PartyEvents;

// TODO: Future event streaming solution here
#[derive(Clone)]
pub struct PartyWatcher {
    pub(crate) relay: Relay
}

impl PartyWatcher {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }
    pub async fn tick(&self) -> RgResult<()> {
        let parties = self.relay.ds.multiparty_store.all_party_info_with_key().await?;

        let all_parties = AllParties::new(parties.clone());
        let active = all_parties.active;

        let mut shared_data = self.enrich_prepare_data(active.clone()).await?;
        // TODO: self.merge_child_events
        self.calculate_party_stream_events(&mut shared_data).await?;
        if self.relay.node_config.opts.enable_party_mode {
            self.handle_order_fulfillment(&mut shared_data).await?;
        }
        self.handle_key_rotations(&mut shared_data).await?;

        // Persist shared data
        for (pk, pid) in shared_data.iter() {
            self.relay.ds.multiparty_store.update_party_data(&pk, pid.to_party_data()).await?;
        }
        self.relay.external_network_shared_data.write(shared_data.clone()).await;
        if self.relay.node_config.opts.enable_party_mode {
            // info!("Party watcher tick num parties total {} active {}", parties.len(), active.len());
            self.tick_formations(&shared_data).await?;
            info!("Completed party tick on node {}", self.relay.node_config.short_id().expect("Node ID"));
            for (pk, pid) in shared_data.iter() {
                let mut pid2 = pid.clone();
                pid2.clear_sensitive();
                info!("Party {} data {}", pk.hex(), pid2.json_or());
            }
        }
        Ok(())
    }


    async fn calculate_party_stream_events(&self, data: &mut HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {
        for (k,v ) in data.iter_mut() {
            let mut pe = PartyEvents::new(k, &self.relay.node_config.network, &self.relay);
            for e in v.address_events.iter() {
                pe.process_event(e).await?;
            }
            v.party_events = Some(pe);
        }
        Ok(())
    }
}

#[async_trait]
impl IntervalFold for PartyWatcher {
    async fn interval_fold(&mut self) -> RgResult<()> {
        self.tick().await.log_error().bubble_abort()?.ok();
        Ok(())
    }
}