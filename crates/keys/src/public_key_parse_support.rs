use redgold_schema::{error_info, RgResult};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::PublicKey;
use crate::proof_support::PublicKeySupport;

//
// if let Some(pk) = PublicKey::from_hex(hash_input.clone()).ok().or(
// PublicKey::from_hex_direct(hash_input.clone()).and_then(|a| a.validate().map(|_| a.clone())).ok()
// ) {
// if pk.validate().is_ok() {
//
//
// }
pub trait PublicKeyParseSupport {
    fn parse_public_key(&self) -> RgResult<PublicKey>;
}

impl<T: Into<String> + Clone> PublicKeyParseSupport for T {
    fn parse_public_key(&self) -> RgResult<PublicKey> {
        let str_rep: String = self.clone().into();

        let result = if let Ok(pk) = PublicKey::from_hex(&str_rep) {
            pk
        } else if let Ok(pk) = PublicKey::from_hex_direct(&str_rep).and_then(|a| a.validate().map(|_| a.clone())) {
            pk
        } else {
            return Err(error_info("Unable to parse public key: ".to_string())).add(str_rep.clone());
        };

        if result.validate().is_ok() {
            Ok(result)
        } else {
            Err(error_info("Invalid public key: ".to_string())).add(str_rep)
        }
    }
}

#[test]
pub fn public_key_parse_test() {
    // todo fill out
    // let pk = "6abbf9e602ea5230180e73088eec9ba27e1b11e41a51e560c7c57e3155e42a87".parse_public_key().unwrap();
    // println!("{}", pk.render_string().unwrap());
}