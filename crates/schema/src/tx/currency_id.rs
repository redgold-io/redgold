use crate::structs;
use crate::structs::{CurrencyDescriptor, CurrencyId, SupportedCurrency};


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

    pub fn from_erc20(contract_address: &structs::Address) -> Self {
        let mut s = CurrencyId::default();
        s.set_supported_currency(SupportedCurrency::Ethereum);
        let mut d = CurrencyDescriptor::default();
        d.contract = Some(contract_address.clone());
        s.descriptor = Some(d);
        s
    }
}