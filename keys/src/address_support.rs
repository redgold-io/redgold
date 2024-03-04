use redgold_schema::{error_info, RgResult};
use redgold_schema::structs::Address;
use crate::eth::example::EthHistoricalClient;
use crate::util::btc_wallet::SingleKeyBitcoinWallet;

pub trait AddressSupport {
    fn parse_address(&self) -> RgResult<Address>;
}

impl AddressSupport for String {
    fn parse_address(&self) -> RgResult<Address> {
        let result = if let Ok(_a) = SingleKeyBitcoinWallet::parse_address(&self) {
            Address::from_bitcoin(&self)
        } else if let Ok(a) = EthHistoricalClient::parse_address(&self) {
            a
        } else if let Ok(a) = Address::parse(self.clone()) {
            a
        } else {
            return Err(error_info("Unable to parse address".to_string()));
        };
        Ok(result)
    }
}