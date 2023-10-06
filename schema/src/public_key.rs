use crate::{bytes_data, from_hex, SafeBytesAccess, ShortString, structs};
use crate::structs::{Address, ErrorInfo, PublicKeyType};

impl structs::PublicKey {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        structs::PublicKey {
            bytes: bytes_data(bytes),
            key_type: PublicKeyType::Secp256k1 as i32
        }
    }

    pub fn hex(&self) -> Result<String, ErrorInfo> {
        let b = self.bytes.safe_bytes()?;
        Ok(hex::encode(b))
    }

    pub fn hex_or(&self) -> String {
        self.hex().unwrap_or("hex error".to_string())
    }

    pub fn bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        self.bytes.safe_bytes()
    }

    pub fn short_id(&self) -> String {
        self.hex().expect("hex").short_string().expect("worked")
    }

    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Address::from_struct_public(self)
    }

    pub fn from_hex(hex: impl Into<String>) -> Result<Self, ErrorInfo> {
        let bytes = from_hex(hex.into())?;
        let key = Self::from_bytes(bytes);
        Ok(key)
    }

    pub fn vec(&self) -> Vec<u8> {
        self.bytes().expect("")
    }
}
