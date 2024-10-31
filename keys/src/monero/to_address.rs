use redgold_schema::{RgResult, structs};
use redgold_schema::errors::into_error::ToErrorInfo;

pub trait ToMoneroAddress {
    fn to_monero_address(&self) -> RgResult<structs::Address>;
}

impl ToMoneroAddress for structs::PublicKey {
    fn to_monero_address(&self) -> RgResult<structs::Address> {
        "monero address".to_error()
    }
}