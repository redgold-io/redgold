use std::collections::{HashMap, HashSet};
use redgold_data::data_store::DataStore;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::RgResult;
use redgold_schema::structs::SupportedCurrency;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::external_data::{PriceDataPointUsdQuery, UsdPrice};
use crate::party::portfolio_request::get_most_recent_day_millis;



pub trait PriceDataPointQueryImpl {
    async fn daily_enrichment<T: ExternalNetworkResources + Send>(&mut self, t: &T, ds: &DataStore) -> RgResult<()>;
    fn new() -> Self;
    async fn query_price<T: ExternalNetworkResources + Send>(
        &mut self, time: i64, currency: SupportedCurrency, ds: &DataStore, external_network_resources: &T
    ) -> RgResult<f64>;
    async fn enrich_address_events<T: ExternalNetworkResources + Send>(
        &mut self, events: &mut Vec<AddressEvent>, ds: &DataStore, external_network_resources: &T
    ) -> RgResult<()>;
}

impl PriceDataPointQueryImpl for PriceDataPointUsdQuery {

    async fn daily_enrichment<T: ExternalNetworkResources + Send>(&mut self, t: &T, ds: &DataStore) -> RgResult<()> {
        let recent = get_most_recent_day_millis();

        self.query_price(recent, SupportedCurrency::Bitcoin, ds, t).await?;
        self.query_price(recent, SupportedCurrency::Ethereum, ds, t).await?;
        Ok(())
    }

    fn new() -> Self {
        Self {
            inner: HashMap::new()
        }
    }
    async fn query_price<T: ExternalNetworkResources + Send>(
        &mut self, time: i64, currency: SupportedCurrency, ds: &DataStore, external_network_resources: &T
    ) -> RgResult<f64> {
        let price = self.inner.get(&time);
        if let Some(p) = price {
            return Ok(p.price);
        }
        let price = ds.price_time.select_price(time, currency).await?;
        if let Some(price) = price {
            return Ok(price);
        }
        // let price = okx_point(time, currency).await?.close;
        let price = external_network_resources.query_price(time, currency).await?;
        self.inner.insert(time, UsdPrice {
            currency,
            price
        });
        ds.price_time.store_price_time(price, time, currency).await?;
        Ok(price)
    }

    async fn enrich_address_events<T: ExternalNetworkResources + Send>(
        &mut self, events: &mut Vec<AddressEvent>, ds: &DataStore, external_network_resources: &T
    ) -> RgResult<()> {
        for event in events.iter_mut() {
            match event {
                AddressEvent::Internal(txo) => {
                    let time = txo.tx.time()?.clone();
                    let currency = txo.tx.external_destination_currency();
                    if let Some(c) = currency {
                        let price = self.query_price(time, c, ds, external_network_resources).await?;
                        txo.price_usd = Some(price);
                    }
                    if let Some(pr) = txo.tx.portfolio_request() {
                        let mut hs = HashSet::new();
                        if let Some(pi) = pr.portfolio_info.as_ref() {
                            for pw in pi.portfolio_weightings.iter() {
                                if let Some(c) = pw.currency.as_ref() {
                                    if let Some(c) = SupportedCurrency::from_i32(*c) {
                                        hs.insert(c);
                                    }
                                }
                                hs.insert(pw.currency());
                            }
                        }
                        for c in hs {
                            let price = self.query_price(time, c, ds, external_network_resources).await?;
                            txo.all_relevant_prices_usd.insert(c, price);
                        }
                    }
                }
                AddressEvent::External(e) => {
                    if e.incoming {
                        if let Some(t) = e.timestamp {
                            let price = self.query_price(t as i64, e.currency, ds, external_network_resources).await?;
                            e.price_usd = Some(price);
                        }
                    }
                }
            }
        }
        Ok(())

    }

}
