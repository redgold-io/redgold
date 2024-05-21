use bdk::database::MemoryDatabase;
use redgold_schema::{error_info, RgResult};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::Address;
use crate::eth::historical_client::EthHistoricalClient;
use crate::util::btc_wallet::SingleKeyBitcoinWallet;

pub trait AddressSupport {
    fn parse_address(&self) -> RgResult<Address>;
}

impl<T : Into<String> + Clone> AddressSupport for T {
    fn parse_address(&self) -> RgResult<Address> {

        let str_rep: String = self.clone().into();

        let result = if let Ok(_a) = SingleKeyBitcoinWallet::<MemoryDatabase>::parse_address(&str_rep) {
            Address::from_bitcoin(&str_rep)
        } else if let Ok(a) = EthHistoricalClient::parse_address(&str_rep) {
            a
        } else if let Ok(a) = Address::from_hex(&str_rep) {
            a
        } else {
            return Err(error_info("Unable to parse address: ".to_string())).add(str_rep.clone());
        };
        Ok(result)
    }
}

#[test]
pub fn address_parse_test(){
    let a = "6abbf9e602ea5230180e73088eec9ba27e1b11e41a51e560c7c57e3155e42a87".parse_address().unwrap();
    println!("{}", a.render_string().unwrap());
}