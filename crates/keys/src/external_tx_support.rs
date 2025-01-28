use crate::address_support::AddressSupport;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::{structs, RgResult};

pub trait ExternalTxSupport {

    fn other_address_typed(&self) -> RgResult<structs::Address>;

}
impl ExternalTxSupport for ExternalTimedTransaction {

    fn other_address_typed(&self) -> RgResult<structs::Address> {
        let mut addr = self.other_address.parse_address()?;
        Ok(addr)
    }

}