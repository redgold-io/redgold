use async_trait::async_trait;
use bdk::bitcoin::PublicKey;
use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::{RgResult, structs};
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use crate::core::relay::Relay;
use crate::core::stream_handlers::{IntervalFold, IntervalFoldOrReceive};

// TODO: Future event streaming solution here

pub struct ExternalNetworkScraper {
    relay: Relay
}

#[derive(Clone)]
pub struct ExternalNetworkData {
    pub pk: structs::PublicKey,
    pub transactions: Vec<ExternalTimedTransaction>,
    pub balance: CurrencyAmount,
    pub currency: SupportedCurrency
}

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