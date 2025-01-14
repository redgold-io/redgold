use bdk::bitcoin::secp256k1::PublicKey;
use redgold_schema::structs;

pub trait ToPublicKeyFromLib {
    fn to_struct_public_key(&self) -> structs::PublicKey;
}


impl ToPublicKeyFromLib for PublicKey {
    fn to_struct_public_key(&self) -> structs::PublicKey {
        structs::PublicKey::from_bytes_direct_ecdsa(self.serialize().to_vec())
    }
}
