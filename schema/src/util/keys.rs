use bitcoin::secp256k1::PublicKey;
use crate::structs;

pub trait ToPublicKeyFromLib {
    fn to_struct_public_key(&self) -> structs::PublicKey;
}


impl ToPublicKeyFromLib for PublicKey {
    fn to_struct_public_key(&self) -> structs::PublicKey {
        structs::PublicKey::from_bytes(self.serialize().to_vec())
    }
}
