use bitcoin::secp256k1::{Error, PublicKey, Signature};
use bitcoin::hashes::hex::ToHex;
use redgold_schema::{error_info, SafeBytesAccess, structs, TestConstants};
use redgold_schema::structs::ErrorInfo;
use redgold_schema::util::sign;

pub trait ToPublicKey {
    fn to_public_key(&self) -> Result<PublicKey, ErrorInfo>;
}

impl ToPublicKey for structs::PublicKey {
    fn to_public_key(&self) -> Result<PublicKey, ErrorInfo> {
        let b = self.bytes.safe_bytes()?;
        return PublicKey::from_slice(&b).map_err(|e| error_info(e.to_string()));
    }
}

pub fn public_key_from_bytes(bytes: &Vec<u8>) -> Result<PublicKey, Error> {
    return PublicKey::from_slice(bytes);
}

pub fn public_key_to_bytes(bytes: &PublicKey) -> Vec<u8> {
    return bytes.serialize().to_vec();
}

#[test]
fn test_pk_hex_encode() {
    let tc = TestConstants::new();
    // println!("{:?}", tc.public.serialize());
    // println!("{:?}", tc.public.serialize_uncompressed());
    // println!("{:?}", tc.public.to_hex());
    // println!("{:?}", hex::decode(tc.public.to_hex()));
    assert_eq!(
        tc.public.serialize().to_vec(),
        hex::decode(tc.public.to_hex()).unwrap()
    )
}

// TODO: update to compact
#[test]
fn test_signature_hex_encode_matches_der() {
    // let tc: TestConstants = TestConstants::new();
    // let sig = sign(&tc.hash_vec, &tc.secret);
    // let siga = Signature::from_der(&*sig.expect("works")).unwrap();
    // assert_eq!(siga.to_string(), hex::encode(siga.serialize_der().to_vec()));
    // let sig2 = Signature::from_compact(&*siga.serialize_compact().to_vec()).unwrap();
    // let sig3 = Signature::from_der(&*siga.serialize_der().to_vec()).unwrap();
    // println!("{:?}", sig);
    // println!("{:?}", siga.serialize_compact());
    // println!("{:?}", hex::encode(siga.serialize_compact()));
    // println!("{:?}", hex::encode(siga.serialize_der().to_vec()));
    // println!("{:?}", sig2.to_string());
    // println!("{:?}", sig3.to_string());
}
