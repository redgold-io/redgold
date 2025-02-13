use crate::errors::into_error::ToErrorInfo;
use crate::observability::errors::EnhanceErrorInfo;
use crate::structs::{CurrencyId, ErrorInfo, SupportedCurrency};
use strum::IntoEnumIterator;


impl TryFrom<String> for SupportedCurrency {
    type Error = ErrorInfo;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        for x in Self::iter() {
            if format!("{:?}", x).to_lowercase() == value.to_lowercase() {
                return Ok(x);
            }
            if x.abbreviated().to_lowercase() == value.to_lowercase() {
                return Ok(x);
            }
        }
        "Failed to parse currency".to_error()
            .with_detail("input_string", value.to_string())
    }
}

impl SupportedCurrency {

    pub fn multisig_party_currencies() -> Vec<SupportedCurrency> {
        vec![
            SupportedCurrency::Redgold,
            SupportedCurrency::Bitcoin,
            SupportedCurrency::Ethereum,
            // SupportedCurrency::Solana,
            // SupportedCurrency::Monero,
        ]
    }

    pub fn multisig_contract_fees() -> Vec<SupportedCurrency> {
        vec![
            SupportedCurrency::Ethereum,
            SupportedCurrency::Solana,
        ]
    }

    pub fn valid_swap_input(&self) -> bool {
        vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold, SupportedCurrency::Ethereum]
            .iter().any(|x| x == self)
    }

    pub fn valid_swap_output(&self) -> bool {
        Self::supported_swap_currencies()
            .iter().any(|x| x == self)
    }

    pub fn supported_swap_currencies() -> Vec<SupportedCurrency> {
        vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold, SupportedCurrency::Ethereum]
    }

    pub fn supported_external_swap_currencies() -> Vec<SupportedCurrency> {
        vec![SupportedCurrency::Bitcoin, SupportedCurrency::Ethereum]
    }

    pub fn abbreviated(&self) -> String {
        match self {
            SupportedCurrency::Redgold => "RDG".to_string(),
            SupportedCurrency::Bitcoin => "BTC".to_string(),
            SupportedCurrency::Ethereum => "ETH".to_string(),
            SupportedCurrency::Usd => { "USD".to_string() }
            SupportedCurrency::UsdcEth => { "USDC".to_string() }
            SupportedCurrency::UsdtEth => { "USDT".to_string() }
            SupportedCurrency::Solana => { "SOL".to_string() }
            SupportedCurrency::Monero => { "XMR".to_string() }
            SupportedCurrency::Cardano => { "ADA".to_string() }
            SupportedCurrency::WrappedRedgoldEthereum => {"wRDGeth".to_string()}
            SupportedCurrency::WrappedRedgoldSolana => {"wRDGsol".to_string()}
        }
    }
    pub fn to_display_string(&self) -> String {
        match self {
            SupportedCurrency::UsdtEth => return "USDT".to_string(),
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
            SupportedCurrency::UsdcEth => 1.0,
            SupportedCurrency::UsdtEth => 1.0,
            SupportedCurrency::Solana => 150.0,
            SupportedCurrency::Monero => 150.0,
            SupportedCurrency::Cardano => 100.0,
            SupportedCurrency::WrappedRedgoldEthereum => 100.0,
            SupportedCurrency::WrappedRedgoldSolana => 100.0,
        }
    }

    pub fn to_currency_id(&self) -> CurrencyId {
        let mut cid = CurrencyId::default();
        cid.set_supported_currency(self.clone());
        cid
    }

    pub fn only_one_destination_per_tx(&self) -> bool {
        match self {
            SupportedCurrency::Ethereum => true,
            SupportedCurrency::UsdcEth => true,
            SupportedCurrency::UsdtEth => true,
            SupportedCurrency::WrappedRedgoldEthereum => true,
            // double check these?
            SupportedCurrency::Solana => true,
            SupportedCurrency::Monero => true,
            SupportedCurrency::Cardano => true,
            SupportedCurrency::WrappedRedgoldSolana => true,
            _ => { false}
        }
    }

}