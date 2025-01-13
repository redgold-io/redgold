use crate::structs::SupportedCurrency;

impl SupportedCurrency {

    pub fn abbreviated(&self) -> String {
        match self {
            SupportedCurrency::Redgold => "RDG".to_string(),
            SupportedCurrency::Bitcoin => "BTC".to_string(),
            SupportedCurrency::Ethereum => "ETH".to_string(),
            SupportedCurrency::Usd => { "USD".to_string() }
            SupportedCurrency::Usdc => { "USDC".to_string() }
            SupportedCurrency::Usdt => { "USDT".to_string() }
            SupportedCurrency::Solana => { "SOL".to_string() }
            SupportedCurrency::Monero => { "XMR".to_string() }
            SupportedCurrency::Cardano => { "ADA".to_string() }
        }
    }
    pub fn to_display_string(&self) -> String {
        match self {
            SupportedCurrency::Usdt => return "USDT".to_string(),
            _ => {}
        }
        format!("{:?}", self)
    }

    pub fn price_default(&self) -> f64 {
        match self {
            SupportedCurrency::Redgold => 100.0,
            SupportedCurrency::Bitcoin => 60000.0,
            SupportedCurrency::Ethereum => 3000.0,
            SupportedCurrency::Usd => 1.0,
            SupportedCurrency::Usdc => 1.0,
            SupportedCurrency::Usdt => 1.0,
            SupportedCurrency::Solana => 150.0,
            SupportedCurrency::Monero => 150.0,
            SupportedCurrency::Cardano => 100.0,
        }
    }
}