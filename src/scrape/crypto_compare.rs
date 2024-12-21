use std::collections::HashMap;
use std::time::Duration;
use itertools::Itertools;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::structs::SupportedCurrency;
use rocket::serde::{Deserialize, Serialize};
use crate::scrape::translate_currency_short;
use crate::util;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct XMRResponse {
    USD: f64
}

pub async fn xmr_lookup() -> RgResult<f64> {
    let url = "https://min-api.cryptocompare.com/data/price?fsym=XMR&tsyms=USD";

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    // println!("Response text: {}", text.clone());
    let res = text.json_from::<XMRResponse>()?;
    Ok(res.USD)

}

pub async fn min_api_crypto_compare_lookup(symbol: String, quote: String, limit: i64, ts: i64) -> RgResult<CcDailyMinApiResponse> {
    let url = format!("https://min-api.cryptocompare.com/data/v2/histominute?fsym={symbol}&tsym={quote}&limit={limit}&toTs={ts}");

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    // println!("Response text: {}", text.clone());
    let res = text.json_from::<CcDailyMinApiResponse>()?;
    Ok(res)
}
pub async fn min_api_crypto_compare_daily_historical_candles(symbol: String, quote: String, limit: i64, ts: i64) -> RgResult<CcDailyMinApiResponse> {
    let url = format!("https://min-api.cryptocompare.com/data/v2/histoday?fsym={symbol}&tsym={quote}&limit={limit}&toTs={ts}");

    use reqwest::ClientBuilder;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    // println!("Response text: {}", text.clone());
    let res = text.json_from::<CcDailyMinApiResponse>()?;
    Ok(res)
}

pub async fn crypto_compare_daily_query(currency: SupportedCurrency, millis: i64, limit: Option<i64>) -> RgResult<Vec<CcDailyMinDataItem>> {
    let cur = translate_currency_short(currency);
    let data = min_api_crypto_compare_daily_historical_candles(
        cur?, "USD".to_string(),
        limit.unwrap_or(365), millis/1000
    ).await?;
    let close = data.Data.Data;
    Ok(close)
}
pub async fn daily_one_year() -> RgResult<HashMap<SupportedCurrency, Vec<(i64, f64)>>> {
    let mut map = HashMap::new();

    for cur in vec![SupportedCurrency::Bitcoin, SupportedCurrency::Ethereum] {
        let data = crypto_compare_daily_query(
            cur,
            util::current_time_millis_i64(),
            None
        ).await?
            .into_iter()
            .map(|x| (x.time * 1000, x.close))
            .collect_vec();
        map.insert(cur, data);
    }

    Ok(map)
}

#[ignore]
#[tokio::test]
async fn test_crypto_compare() {
    let data = crypto_compare_daily_query(
        SupportedCurrency::Bitcoin,
        util::current_time_millis_i64(),
        None
    ).await.unwrap();
    println!("Data: {:?}", data);
}

pub async fn crypto_compare_point_query(currency: SupportedCurrency, millis: i64) -> RgResult<f64> {
    let cur = translate_currency_short(currency);
    let data = min_api_crypto_compare_lookup(cur?, "USD".to_string(), 1, millis/1000).await?;
    let close = data.Data.Data.last().ok_msg("Failed to get data")?.close;
    Ok(close)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CcDailyMinDataItem {
    time: i64,
    close: f64,
    high: f64,
    low: f64,
    open: f64,
    volumefrom: f64,
    volumeto: f64,
    conversionType: String,
    conversionSymbol: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CcDailyMinData {
    Aggregated: bool,
    TimeFrom: i64,
    TimeTo: i64,
    Data: Vec<CcDailyMinDataItem>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CcDailyMinApiResponse {
    Response: String,
    Message: String,
    HasWarning: bool,
    Type: i64,
    RateLimit: RateLimit,
    Data: CcDailyMinData
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RateLimit {}

pub async fn xmr_lookup_dailies_delta() -> RgResult<f64> {
    let url = "https://min-api.cryptocompare.com/data/v2/histoday?fsym=XMR&tsym=USD&limit=1";

    use reqwest::ClientBuilder;
    use redgold_schema::error_info;
    let client = ClientBuilder::new().timeout(Duration::from_secs(10)).build().unwrap();
    let sent = client
        .get(url)
        .send();
    let response = sent.await.error_info("Failed to get response")?;
    let text = response.text().await.error_info("Failed to get response text")?;
    // println!("Response text: {}", text.clone());
    let res = text.json_from::<CcDailyMinApiResponse>()?;
    let yesterday = res.Data.Data.get(0).ok_or(error_info("Failed to get data"))?;
    let today = res.Data.Data.get(1).ok_or(error_info("Failed to get data"))?;
    Ok((today.close - yesterday.close) / yesterday.close)

}