use async_trait::async_trait;
use redgold_schema::RgResult;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_keys::address_external::{ToBitcoinAddress, ToCurrencyAddress};
use redgold_schema::structs::{CurrencyAmount, PublicKey, SupportedCurrency};
use crate::party::party_stream::PartyEvents;

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
    pub async fn attempt_fulfillment(&mut self, currency: &SupportedCurrency, amount: &CurrencyAmount, apk: &PublicKey) -> RgResult<()> {
        let b = self.external_resources.self_balance(currency.clone()).await?;
        if let Some(min) = PartyEvents::minimum_stake_amount_total(currency.clone()) {
            let delta = b - min.clone();
            let to_send = if delta > amount.clone() {
                amount.clone()
            } else {
                delta.clone()
            };
            if to_send > min {
                let dest = apk.to_currency_address(currency, &self.relay.node_config.network)?;
                if let Ok(mut dest) = dest {
                    dest.mark_external();
                    self.external_resources.send(&dest, &to_send, true, None, None).await?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<T> IntervalFold for PortfolioFullfillmentAgent<T> where T: ExternalNetworkResources + Send {
    async fn interval_fold(&mut self) -> RgResult<()> {
        if let Some(apk) = self.relay.active_party_key().await {
            if let Some(nid) = self.relay.external_network_shared_data.clone_read().await.get(&apk) {
                if let Some(pev) = nid.party_events.as_ref() {
                    let imba = pev.portfolio_request_events.current_portfolio_imbalance.clone();
                    for (c, a) in imba.iter() {
                        if a.to_fractional() > 0.0 {
                            // this represents a request for more stake
                            if PartyEvents::meets_minimum_stake_amount(a) {
                                // proceed potentially with fulfillment.
                                self.attempt_fulfillment(c, a, &apk)?
                            }
                        } else {
                            // this represents a request for a stake withdrawal.
                        }
                    }
                }
            }
        }
        Ok(())
    }
}