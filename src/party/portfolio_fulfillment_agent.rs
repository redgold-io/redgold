use async_trait::async_trait;
use redgold_schema::RgResult;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;
use redgold_common::external_resources::ExternalNetworkResources;

struct PortfolioFullfillmentAgent<T> where T: ExternalNetworkResources {
    pub relay: Relay,
    pub external_resources: T
}

impl<T> PortfolioFullfillmentAgent<T> where T: ExternalNetworkResources + Send {
    pub fn new(relay: &Relay, external_resources: T) -> Self {
        Self {
            relay: relay.clone(),
            external_resources
        }
    }
}

#[async_trait]
impl<T> IntervalFold for PortfolioFullfillmentAgent<T> where T: ExternalNetworkResources + Send {
    async fn interval_fold(&mut self) -> RgResult<()> {
        if let Some(apk) = self.relay.active_party_key().await {
            if let Some(nid) = self.relay.external_network_shared_data.clone_read().await.get(&apk) {
                if let Some(pev) = nid.party_events.as_ref() {
                    //pev.portfolio_request_events.current_portfolio_imbalance
                }
            }
        }
        Ok(())
    }
}