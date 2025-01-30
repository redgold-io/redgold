use ethers::addressbook::Address;
use redgold_schema::{structs, ErrorInfoContext, RgResult};

pub fn parse_address(value: &String) -> RgResult<structs::Address> {
    let addr: Address = value.parse().error_info("address parse failure")?;
    Ok(structs::Address::from_eth_direct(value))
}