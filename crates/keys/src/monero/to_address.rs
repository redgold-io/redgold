use crate::TestConstants;
use monero::{Address, Network, PublicKey};
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};


pub trait ToMoneroAddress {
    fn to_monero_address_from_monero_public_format(&self, net: &NetworkEnvironment) -> RgResult<structs::Address>;
}

impl ToMoneroAddress for structs::PublicKey {
    fn to_monero_address_from_monero_public_format(&self, net: &NetworkEnvironment) -> RgResult<structs::Address> {
        let spend = self.raw_bytes()?;
        let aux = self.aux_data.clone().ok_msg("Missing aux bytes")?.value;
        let public_spend = PublicKey::from_slice(&spend).error_info("Invalid spend key")?;
        let public_view = PublicKey::from_slice(&aux).error_info("Invalid view key")?;

        let net = if net == &NetworkEnvironment::Main {
            Network::Mainnet
        } else {
            Network::Testnet
        };
        // Create the address (mainnet)
        let address = Address::standard(net, public_spend, public_view);
        Ok(structs::Address::from_monero(&address.to_string()))
    }
}

#[test]
fn debug_keygen() {
    let tc = TestConstants::new();
    let wp = tc.words_pass;
    // let mkp = wp.derive_monero_keys().unwrap();
}