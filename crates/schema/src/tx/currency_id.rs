use crate::structs::{CurrencyId, SupportedCurrency};


impl From<SupportedCurrency> for CurrencyId {
    fn from(currency: SupportedCurrency) -> Self {
        let mut s = CurrencyId::default();
        s.set_supported_currency(currency);
        s
    }
}

impl CurrencyId {
    pub fn bitcoin() -> Self {
        SupportedCurrency::Bitcoin.into()
    }
}