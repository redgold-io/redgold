use rocket::serde::{Deserialize, Serialize};
use std::collections::HashMap;
use redgold_data::data_store::DataStore;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::RgResult;
use redgold_schema::structs::SupportedCurrency;
use crate::party::address_event::AddressEvent;
use crate::scrape::okx_point;

#[derive(Clone, Serialize, Deserialize)]
pub struct UsdPrice {
    currency: SupportedCurrency,
    price: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PriceDataPointUsdQuery {
    inner: HashMap<i64, UsdPrice>,
}

impl PriceDataPointUsdQuery {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new()
        }
    }
    pub async fn query_price(&mut self, time: i64, currency: SupportedCurrency, ds: &DataStore) -> RgResult<f64> {
        let price = self.inner.get(&time);
        if let Some(p) = price {
            return Ok(p.price);
        }
        let price = ds.price_time.select_price(time, currency).await?;
        if let Some(price) = price {
            return Ok(price);
        }
        let price = okx_point(time, currency).await?.close;
        self.inner.insert(time, UsdPrice {
            currency,
            price
        });
        ds.price_time.store_price_time(price, time, currency).await?;
        Ok(price)
    }

    pub async fn enrich_address_events(&mut self, events: &mut Vec<AddressEvent>, ds: &DataStore) -> RgResult<()> {
        for event in events.iter_mut() {
            match event {
                AddressEvent::Internal(txo) => {
                    let time = txo.tx.time()?.clone();
                    let currency = txo.tx.external_destination_currency();
                    if let Some(c) = currency {
                        let price = self.query_price(time, c, ds).await?;
                        txo.price_usd = Some(price);
                    }
                }
                AddressEvent::External(e) => {
                    if e.incoming {
                        if let Some(t) = e.timestamp {
                            let price = self.query_price(t as i64, e.currency, ds).await?;
                            e.price_usd = Some(price);
                        }
                    }
                }
            }
        }
        Ok(())

    }

}
