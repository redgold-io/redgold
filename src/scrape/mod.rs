pub mod external_networks;
pub mod crypto_compare;

use std::time::Duration;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::SupportedCurrency;
use crate::util;
use crate::util::{current_time_millis_i64, current_time_unix};


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

Kline/Candlestick chart intervals:

s-> seconds; m -> minutes; h -> hours; d -> days; w -> weeks; M -> months

1s 1m 3m 5m 15m 30m 1h 2h 4h 6h 8h 12h 1d 3d 1w 1M

`GET /api/v3/klines`

Kline/candlestick bars for a symbol.
Klines are uniquely identified by their open time.

**Weight(IP):** 2

**Parameters:**

|Name|Type|Mandatory|Description|
|---|---|---|---|
|symbol|STRING|YES||
|interval|ENUM|YES||
|startTime|LONG|NO||
|endTime|LONG|NO||
|timeZone|STRING|NO|Default: 0 (UTC)|
|limit|INT|NO|Default 500; max 1000.|

- If startTime and endTime are not sent, the most recent klines are returned.
- Supported values for `timeZone`:
    - Hours and minutes (e.g. `-1:00`, `05:45`)
    - Only hours (e.g. `0`, `8`, `4`)
    - Accepted range is strictly [-12:00 to +14:00] inclusive
- If `timeZone` provided, kline intervals are interpreted in that timezone instead of UTC.
- Note that `startTime` and `endTime` are always interpreted in UTC, regardless of `timeZone`.

**Data Source:** Database
 */


/*
Doesn't seem to work for new data, 6 months out of date FOR binance.US
// for binance.com, banned in some countries.
// does seem to work if you access from the website for historical data.
 */
pub async fn binance_klines_historical(
    start_time: u64, interval: String
) -> RgResult<()> {

    let url = "https://api.binance.com/api/v3/klines";

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .query(&[
            ("symbol", "BTCUSD"),
            ("interval", &*interval),
            // ("startTime", &*start_time.to_string()),
            // ("endTime", &*end_time.to_string()),
            // ("limit", &*limit.to_string())
        ])
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    println!("Response text: {}", text.clone());
    Ok(())
}

#[ignore]
#[tokio::test]
async fn test_binance_klines_historical() {
    let t = util::current_time_millis() - 1000*3600*24; // 1 hour ago
    println!("t: {}", t);
    // 1709607936485
    // let t = 1689273600000;
    // let t = 1709676464727;
    // let start_time = t;
    // let end_time = t + 1000*600;
    let interval = "1d".to_string();
    // let limit = 100;
    let r = binance_klines_historical(t, interval).await;
    assert!(r.is_ok());
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
#[allow(non_snake_case)]
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


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinbaseHistoricalDataResponse {
    data: CoinbaseHistoricalData

}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinbaseHistoricalData {
    base: String,
    currency: String,
    prices: Vec<CoinbasePriceTime>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinbasePriceTime {
    price: String,
    time: String
}

impl CoinbasePriceTime {
    pub fn price_f64(&self) -> RgResult<f64> {
        self.price.parse::<f64>()
            .error_info(format!("Failed to parse price {}", self.price))
    }
    pub fn time_i64(&self) -> RgResult<i64> {
        self.time.parse::<i64>()
            .error_info(format!("Failed to parse time {}", self.time))
    }
}


/*
Doesn't work
 */
// https://api.coinbase.com/v2/prices/BTC-USD/spot
pub async fn coinbase_historical(
    time: i64, supported_currency: SupportedCurrency
) -> RgResult<CoinbasePriceTime> {
    let product = translate_currency(supported_currency)?;
    let granularity = 3600;

    let url =
        format!(
            "https://api.coinbase.com/v2/prices/{}/historic?start={}&end={}granularity={}",
            product, time, time, granularity
        );
    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    // println!("Response text: {}", text.clone());
    let resp = text.json_from::<CoinbaseHistoricalDataResponse>()?;
    resp.data.prices.iter().filter_map(|a|
        a.time_i64().ok().map(|b| ((time - b).abs(), a))
    ).min_by(|a, b| a.0.cmp(&b.0))
        .map(|a| a.1)
        .cloned().ok_msg("Failed to find min").add(text)
}

fn translate_currency(supported_currency: SupportedCurrency) -> RgResult<String> {
    let product = match supported_currency {
        SupportedCurrency::Bitcoin => {
            "BTC-USD"
        },
        SupportedCurrency::Ethereum => {
            "ETH-USD"
        },
        SupportedCurrency::Usdt => {
            "USDT-USD"
        },
        // SupportedCurrency::Monero => {
        //     "XMR-USD"
        // }
        SupportedCurrency::Solana => {
            "SOL-USD"
        }
        SupportedCurrency::Usdc => {
            "USDC-USD"
        }
        _ => {
            return Err(error_info("Unsupported currency for translate_currency historical data".to_string()));
        }
    };
    Ok(product.to_string())
}


/*
Get index candlesticks history
Retrieve the candlestick charts of the index from recent years.

Rate Limit: 10 requests per 2 seconds
Rate limit rule: IP
HTTP Request
GET /api/v5/market/history-index-candles

Request Example

GET /api/v5/market/history-index-candles?instId=BTC-USD
Request Parameters
Parameter	Type	Required	Description
instId	String	Yes	Index, e.g. BTC-USD
after	String	No	Pagination of data to return records earlier than the requested ts
before	String	No	Pagination of data to return records newer than the requested ts. The latest data will be returned when using before individually
bar	String	No	Bar size, the default is 1m
e.g. [1m/3m/5m/15m/30m/1H/2H/4H]
Hong Kong time opening price k-line: [6H/12H/1D/1W/1M]
UTC time opening price k-line: [/6Hutc/12Hutc/1Dutc/1Wutc/1Mutc]
limit	String	No	Number of results per request. The maximum is 100; The default is 100
 */

pub async fn okx_historical_test(before: Option<i64>, after: i64, currency: SupportedCurrency) -> RgResult<Vec<OkxParsedRow>> {
    let url = "https://www.okx.com/api/v5/market/history-index-candles";

    // Default to previous ten minutes.
    let before = before.unwrap_or(after- 1000*60*10);

    let product = translate_currency(currency)?;

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .query(&[
            ("instId", &*product),
            ("before", &*before.to_string()),
            ("after", &*after.to_string()),
            // ("bar", "1d"),
            // ("limit", "100")
        ])
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    // println!("Response text: {}", text.clone());
    let res = text.json_from::<OkxHistoricalResponse>()?.data()?;
    if res.len() == 0 {
        return Err(error_info("No data found in response")).add(text)
    }
    let res = res.iter()
        .filter(|a| a.confirmed)
        .map(|a| a.clone()).collect_vec();
    if res.len() == 0 {
        return Err(error_info("No confirmed data found in response")).add(text)
    }
    Ok(res)

}


fn translate_currency_short(currency: SupportedCurrency) -> RgResult<String>  {
    let res = match currency {
        SupportedCurrency::Bitcoin => { "BTC" }
        SupportedCurrency::Ethereum => { "ETH" }
        SupportedCurrency::Usdc => { "USDC" }
        SupportedCurrency::Usdt => { "USDT" }
        SupportedCurrency::Monero => { "XMR" }
        SupportedCurrency::Solana => { "SOL" }
        _ => return "Unsupported currency translation".to_error()
    };
    Ok(res.to_string())
}
/*
Get index candlesticks history
Retrieve the candlestick charts of the index from recent years.

Rate Limit: 10 requests per 2 seconds
Rate limit rule: IP
HTTP Request
GET /api/v5/market/history-index-candles

Request Example

GET /api/v5/market/history-index-candles?instId=BTC-USD
Request Parameters
Parameter	Type	Required	Description
instId	String	Yes	Index, e.g. BTC-USD
after	String	No	Pagination of data to return records earlier than the requested ts
before	String	No	Pagination of data to return records newer than the requested ts. The latest data will be returned when using before individually
bar	String	No	Bar size, the default is 1m
e.g. [1m/3m/5m/15m/30m/1H/2H/4H]
Hong Kong time opening price k-line: [6H/12H/1D/1W/1M]
UTC time opening price k-line: [/6Hutc/12Hutc/1Dutc/1Wutc/1Mutc]
limit	String	No	Number of results per request. The maximum is 100; The default is 100
Response Example

{
    "code":"0",
    "msg":"",
    "data":[
     [
        "1597026383085",
        "3.721",
        "3.743",
        "3.677",
        "3.708",
        "1"
    ],
    [
        "1597026383085",
        "3.731",
        "3.799",
        "3.494",
        "3.72",
        "1"
    ]
    ]
}
Response Parameters
Parameter	Type	Description
ts	String	Opening time of the candlestick, Unix timestamp format in milliseconds, e.g. 1597026383085
o	String	Open price
h	String	highest price
l	String	Lowest price
c	String	Close price
confirm	String	The state of candlesticks.
0 represents that it is uncompleted, 1 represents that it is completed.
The data returned will be arranged in an array like this: [ts,o,h,l,c,confirm].
 */


/**
Get the closest point to the given time from OKX for the supported product type
This returns OHLC candlestick data for the previous closest time to the given time.

It has an accuracy of roughly 1 minute, and returns a 1 minute candlestick. Appears to be stable
going back at least a year, and is likely to be stable for the foreseeable future.

This is the only function tested in here that works properly over long time series.
 */
pub async fn okx_point(time: i64, supported_currency: SupportedCurrency) -> RgResult<OkxParsedRow> {
    let res = okx_historical_test(None, time, supported_currency).await
        .with_detail("time", time.to_string())
        .with_detail("currency", format!("{:?}", supported_currency))
        ?;
    res.iter()
        .map(|r| ((time - r.time).abs(), r))
        .min_by(|a, b| a.0.cmp(&b.0))
        .map(|a| a.1).cloned().ok_msg("No data found")

}

pub async fn get_24hr_delta_change_pct(supported_currency: SupportedCurrency) -> RgResult<f64> {
    if supported_currency == SupportedCurrency::Monero {
        return crypto_compare::xmr_lookup_dailies_delta().await;
    }
    let now = current_time_millis_i64();
    let minus_24 = now - 1000*60*60*24;
    let now = okx_point(now, supported_currency).await?;
    let past = okx_point(minus_24, supported_currency).await?;
    let now_close = now.close;
    let past_close = past.close;
    let delta = now_close - past_close;
    let pct_change = delta / past_close;
    // info!("{} {} {} {}", now_close, past_close, delta, pct_change);
    Ok(pct_change)
}

#[derive(Clone, Serialize, Deserialize)]
struct OkxHistoricalResponse {
    code: String,
    msg: String,
    data: Vec<Vec<String>>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OkxParsedRow {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub confirmed: bool
}


impl OkxHistoricalResponse {
    pub fn parse(row: &Vec<String>) -> RgResult<OkxParsedRow> {
        let time = row.get(0).ok_or(error_info("Failed to get time"))?;
        let open = row.get(1).ok_or(error_info("Failed to get open"))?;
        let high = row.get(2).ok_or(error_info("Failed to get high"))?;
        let low = row.get(3).ok_or(error_info("Failed to get low"))?;
        let close = row.get(4).ok_or(error_info("Failed to get close"))?;
        let confirmed = row.get(5).ok_or(error_info("Failed to get confirmed"))?;
        Ok(OkxParsedRow {
            time: time.parse::<i64>().error_info("Failed to parse time")?,
            open: open.parse::<f64>().error_info("Failed to parse open")?,
            high: high.parse::<f64>().error_info("Failed to parse high")?,
            low: low.parse::<f64>().error_info("Failed to parse low")?,
            close: close.parse::<f64>().error_info("Failed to parse close")?,
            confirmed: confirmed == "1"
        })
    }
    pub fn data(&self) -> RgResult<Vec<OkxParsedRow>> {
        self.data.iter().map(|row| Self::parse(row)).collect()
    }
}


// #[ignore]
#[tokio::test]
async fn okx_debug_routes() {
    // let c = coinbase_btc_spot_latest().await.unwrap();
    // println!("{}", c.json_or());
    // let start = current_time_millis_i64() - 1000*60*60*24*1;
    // let end = start + 60*10*1000;
    // let res = okx_historical_test(Some(start), end, SupportedCurrency::Bitcoin).await.unwrap();
    // for x in res {
    //     let delta = end - x.time;
    //     println!("delta {} json {}", delta, x.json_or());
    // }

    let start = current_time_millis_i64() - 1000*60*5;
    for i in 500..510 {
        let t = start - i*1000*60*60*24;
        let c = okx_point(t, SupportedCurrency::Ethereum).await.unwrap();
        let delta = (t- c.time).abs();
        let price = c.open.clone();
        println!("{} {} {}", t, delta, price);

    }
}

#[ignore]
#[tokio::test]
async fn debug_routes() {
    // let c = coinbase_btc_spot_latest().await.unwrap();
    // println!("{}", c.json_or());
    let start = current_time_unix() as i64;
    for i in 0..20 {
        let t = start - i*60*60*24;
        let c = coinbase_historical(t, SupportedCurrency::Bitcoin).await.unwrap();
        let delta = (t- c.time_i64().unwrap()).abs();
        let price = c.price.clone();
        println!("{} {} {}", t, delta, price);

    }
}

#[tokio::test]
async fn debug_okx() {
    let c = get_24hr_delta_change_pct(SupportedCurrency::Bitcoin).await.unwrap();
    println!("{}", c);
}