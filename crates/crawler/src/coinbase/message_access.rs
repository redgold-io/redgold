
use crate::coinbase::status_schema::{Message, Status, Currency};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::structs::SupportedCurrency;

pub trait MessageAccess {
    fn get_currency_info(&self, currency: SupportedCurrency) -> Option<Currency>;
}

impl MessageAccess for Status {
    fn get_currency_info(&self, currency: SupportedCurrency) -> Option<Currency> {
        self.currencies.iter()
            .find(|c| c.id == currency.abbreviated())
            .cloned()
    }
}

impl MessageAccess for Message {
    fn get_currency_info(&self, currency: SupportedCurrency) -> Option<Currency> {
        match self {
            Message::Status(status) => status.get_currency_info(currency),
            _ => None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_parsing() {
        assert_eq!(
            SupportedCurrency::try_from("BTC".to_string()).unwrap(),
            SupportedCurrency::Bitcoin
        );
        assert_eq!(
            SupportedCurrency::try_from("ETH".to_string()).unwrap(),
            SupportedCurrency::Ethereum
        );
        assert!(SupportedCurrency::try_from("INVALID".to_string()).is_err());
    }
}
