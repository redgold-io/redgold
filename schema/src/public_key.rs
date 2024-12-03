use std::fmt::{Display, Formatter};
use crate::{bytes_data, RgResult, SafeOption, ShortString, structs};
use crate::proto_serde::ProtoSerde;
use crate::structs::{Address, ErrorInfo, PublicKey, PublicKeyType};



impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.hex())
    }
}

impl PublicKey {
    pub fn from_bytes_direct_ecdsa(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            key_type: PublicKeyType::Secp256k1 as i32,
            aux_data: None
        }
    }
    pub fn from_bytes_direct_ed25519(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            key_type: PublicKeyType::Ed25519 as i32,
            aux_data: None
        }
    }
    pub fn from_bytes_direct_ed25519_aux(bytes: Vec<u8>, aux: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            key_type: PublicKeyType::Ed25519 as i32,
            aux_data: bytes_data(aux)
        }
    }

    pub fn raw_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.bytes.safe_get().cloned()?.value)
    }

    pub fn short_id(&self) -> String {
        self.hex().short_string().expect("worked")
    }

    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Address::from_struct_public(self)
    }

    pub fn from_hex_direct(hex: impl Into<String>) -> RgResult<Self> {
        let bytes = crate::from_hex(hex.into())?;
        let key = Self::from_bytes_direct_ecdsa(bytes);
        Ok(key)
    }

    pub fn to_hex_direct_ecdsa(&self) -> RgResult<String> {
        self.raw_bytes().map(|b| hex::encode(b))
    }

}
