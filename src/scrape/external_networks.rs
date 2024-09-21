use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::{structs, RgResult};
use redgold_schema::structs::SupportedCurrency;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;

// TODO: Future event streaming solution here

pub struct ExternalNetworkScraper {
    relay: Relay
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ExternalNetworkData {
    pub pk: structs::PublicKey,
    pub transactions: Vec<ExternalTimedTransaction>,
    // pub balance: CurrencyAmount,
    pub currency: SupportedCurrency,
    pub max_ts: Option<u64>,
    pub max_block: Option<u64>
}

// Fut streaming impl here, unused now.
impl ExternalNetworkScraper {

    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }
    pub async fn tick(&self) -> RgResult<()> {
        let groups = self.relay.ds.multiparty_store.all_party_info_with_key().await?;
        for groups in groups {

        }
        Ok(())
    }
}

#[async_trait]
impl IntervalFold for ExternalNetworkScraper {
    async fn interval_fold(&mut self) -> RgResult<()> {
        self.tick().await.bubble_abort()?
    }
}