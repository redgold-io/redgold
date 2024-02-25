use std::time::Duration;
use serde::{Deserialize, Serialize};
use redgold_schema::{EasyJson, error_info, RgResult};
use crate::util;


// "https://api.coinbase.com/v2/exchange-rates?currency=BTC".to_string();


/*
[
  {
    "id": 28457,
    "price": "4.00000100",
    "qty": "12.00000000",
    "quote_qty": "48.000012",
    "time": 1499865549590,
    "is_buyer_maker": true,
    "is_best_match": true
  }
]

 */
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BinanceTradesResponse {
    id: u64,
    price: String,
    qty: String,
    quote_qty: String,
    time: u64,
    is_buyer_maker: bool,
    is_best_match: bool,
}

pub async fn binance_trades_recent() -> RgResult<Vec<BinanceTradesResponse>> {

    let url = "https://api.binance.us/api/v3/trades";

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .query(&[("symbol", "BTCUSD")])
        .send();
    let response = sent.await;
    match response {
        Ok(r) => {
            let text = r.text().await
                .map_err(|e| error_info(format!("{} {}", "Failed to get response text ", e.to_string())))?;
            println!("Response text: {}", text.clone());
            let resp = serde_json::from_str::<Vec<BinanceTradesResponse>>(&*text.clone())
                .map_err(|e| error_info(format!("{} {}", e.to_string(), text)))?;
            Ok(resp)
        },
        Err(e) => Err(error_info(e.to_string())),
    }
}


/*
[
  {
    "a": 26129,         // Aggregate tradeId
    "p": "0.01633102",  // Price
    "q": "4.70443515",  // Quantity
    "f": 27781,         // First tradeId
    "l": 27781,         // Last tradeId
    "T": 1498793709153, // Timestamp
    "m": true,          // Was the buyer the maker?
    "M": true           // Was the trade the best price match?
  }
]
 */
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BinanceAggTradesResponse {
    a: u64,
    p: String,
    q: String,
    f: u64,
    l: u64,
    t: u64,
    m: bool,
    M: bool,
}

pub async fn binance_trades_aggregate() -> RgResult<Vec<BinanceAggTradesResponse>> {

    let neg_offset = 1000*5*24;
    let window_dur = 1000*60;
    let ct = util::current_time_millis();
    let end_time = ct - neg_offset;
    let start_time = end_time - window_dur;


    let url = "https://api.binance.us/api/v3/aggTrades";

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .query(&[("symbol", "BTCUSD")])
        .query(&[("startTime", &*start_time.to_string())])
        .query(&[("endTime", &*end_time.to_string())])
        .send();
    let response = sent.await;
    match response {
        Ok(r) => {
            let text = r.text().await
                .map_err(|e| error_info(format!("{} {}", "Failed to get response text ", e.to_string())))?;
            println!("Response text: {}", text.clone());
            let resp = serde_json::from_str::<Vec<BinanceAggTradesResponse>>(&*text.clone())
                .map_err(|e| error_info(format!("{} {}", e.to_string(), text)))?;
            Ok(resp)
        },
        Err(e) => Err(error_info(e.to_string())),
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinbaseBtcSpotLatestData {
    amount: String,
    currency: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinbaseBtcSpotLatest {
    data: CoinbaseBtcSpotLatestData,
}

impl CoinbaseBtcSpotLatest {
    pub fn usd_btc(&self) -> RgResult<f64> {
        let amt = self.data.amount.parse::<f64>()
            .map_err(|e| error_info(format!("{} {}", "Failed to parse amount ", e.to_string())))?;
        Ok(amt)
    }
}


// https://api.coinbase.com/v2/prices/BTC-USD/spot
pub async fn coinbase_btc_spot_latest() -> RgResult<CoinbaseBtcSpotLatest> {

    let url = "https://api.coinbase.com/v2/prices/BTC-USD/spot";

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .send();
    let response = sent.await;
    match response {
        Ok(r) => {
            let text = r.text().await
                .map_err(|e| error_info(format!("{} {}", "Failed to get response text ", e.to_string())))?;
            println!("Response text: {}", text.clone());
            let resp = serde_json::from_str::<CoinbaseBtcSpotLatest>(&*text.clone())
                .map_err(|e| error_info(format!("{} {}", e.to_string(), text)))?;
            Ok(resp)
        },
        Err(e) => Err(error_info(e.to_string())),
    }
}




#[tokio::test]
async fn debug_routes() {
    let c = coinbase_btc_spot_latest().await.unwrap();
    println!("{}", c.json_or());
}