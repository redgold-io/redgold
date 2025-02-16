use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Message {
    Subscription(SubscriptionMessage),
    Ticker(TickerMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubscriptionMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub channels: Vec<Channel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Channel {
    pub name: String,
    pub product_ids: Vec<String>,
    pub account_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickerMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub sequence: u64,
    pub product_id: String,
    pub price: String,
    pub open_24h: String,
    pub volume_24h: String,
    pub low_24h: String,
    pub high_24h: String,
    pub volume_30d: String,
    pub best_bid: String,
    pub best_bid_size: String,
    pub best_ask: String,
    pub best_ask_size: String,
    pub side: String,
    pub time: String,
    pub trade_id: u64,
    pub last_size: String,
}