use crate::btc::btc_wallet::SingleKeyBitcoinWallet;
use bdk::database::MemoryDatabase;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, SupportedCurrency};
use redgold_schema::{error_info, RgResult};

pub trait AddressSupport {
    fn parse_address(&self) -> RgResult<Address>;
    fn parse_address_incl_raw(&self) -> RgResult<Address>;
    fn parse_ethereum_address(&self) -> RgResult<Address>;
    fn parse_ethereum_address_external(&self) -> RgResult<Address>;
}

impl<T : Into<String> + Clone> AddressSupport for T {
    fn parse_address(&self) -> RgResult<Address> {

        let str_rep: String = self.clone().into();

        let result = if let Ok(_a) = SingleKeyBitcoinWallet::<MemoryDatabase>::parse_address(&str_rep) {
            Address::from_bitcoin(&str_rep)
        } else if let Ok(a) = crate::eth::address_parser::parse_address(&str_rep) {
            a
        } else if let Ok(a) = Address::from_hex(&str_rep).and_then(|a| a.validated()) {
            a
        } else {
            return Err(error_info("Unable to parse address: ".to_string())).add(str_rep.clone());
        };
        Ok(result)
    }

    fn parse_address_incl_raw(&self) -> RgResult<Address> {
        let hex = self.clone().into();
        self.parse_address().or(Address::raw_from_hex(hex))
    }

    fn parse_ethereum_address(&self) -> RgResult<Address> {
        let str_rep: String = self.clone().into();
        let result = if let Ok(a) = crate::eth::address_parser::parse_address(&str_rep) {
            a
        } else {
            return Err(error_info("Unable to parse address: ".to_string())).add(str_rep.clone());
        };
        Ok(result)
    }

    fn parse_ethereum_address_external(&self) -> RgResult<Address> {
        self.parse_ethereum_address().map(|mut a| {
            a.set_currency(SupportedCurrency::Ethereum);
            a
        })
    }
}

#[test]
pub fn address_parse_test(){
    let a = "6abbf9e602ea5230180e73088eec9ba27e1b11e41a51e560c7c57e3155e42a87".parse_address().unwrap();
    println!("{}", a.render_string().unwrap());
}