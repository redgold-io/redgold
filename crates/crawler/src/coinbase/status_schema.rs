use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub currencies: Vec<Currency>,
    #[serde(default)]
    pub products: Option<Vec<Product>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Message {
    Subscriptions(Subscriptions),
    Status(Status),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Subscriptions {
    #[serde(rename = "type")]
    pub message_type: String,
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub product_ids: Vec<String>,
    pub account_ids: Option<serde_json::Value>, // Can be null
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub min_size: String,
    pub status: String,
    pub funding_account_id: String,
    pub status_message: String,
    pub max_precision: String,
    pub convertible_to: Vec<String>,
    pub details: CurrencyDetails,
    pub default_network: String,
    pub supported_networks: Vec<SupportedNetwork>,
    pub network_map: Option<serde_json::Value>, // Can be null
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurrencyDetails {
    #[serde(rename = "type")]
    pub detail_type: String,
    pub symbol: String,
    pub network_confirmations: u32,
    pub sort_order: u32,
    pub crypto_address_link: String,
    pub crypto_transaction_link: String,
    pub push_payment_methods: Option<Vec<String>>, // Can be null
    #[serde(default, alias = "min_withdrawal_amount")] // use alias and default
    pub min_withdrawal_amount: Option<f64>,

    #[serde(default, alias = "max_withdrawal_amount")] // use alias and default
    pub max_withdrawal_amount: Option<f64>,
    #[serde(default)]
    pub group_types: Vec<String>, // added this, because sometimes BTC has group_types
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupportedNetwork {
    pub id: String,
    pub name: String,
    pub status: String,
    pub contract_address: String,
    pub crypto_address_link: String,
    pub crypto_transaction_link: String,

    #[serde(default, alias = "min_withdrawal_amount")] // use alias and default
    pub min_withdrawal_amount: Option<f64>,

    #[serde(default, alias = "max_withdrawal_amount")] // use alias and default
    pub max_withdrawal_amount: Option<f64>,
    pub network_confirmations: u32,
    pub processing_time_seconds: u32,
    pub destination_tag_regex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Product {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub base_increment: String,
    pub quote_increment: String,
    pub display_name: String,
    pub status: String,
    pub margin_enabled: bool,
    pub status_message: String,
    pub min_market_funds: String,
    pub post_only: bool,
    pub limit_only: bool,
    pub cancel_only: bool,
    pub auction_mode: bool,
    #[serde(rename = "type")]
    pub product_type: String,
    pub fx_stablecoin: bool,
    pub max_slippage_percentage: String,
}
