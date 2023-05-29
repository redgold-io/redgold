use bitcoin::secp256k1::PublicKey;
use crate::{bytes_data, error_info, from_hex, SafeBytesAccess, ShortString, structs};
use crate::structs::{ErrorInfo, PublicKeyType};

pub trait ToPublicKey {
    fn to_lib_public_key(&self) -> Result<PublicKey, ErrorInfo>;
}

impl ToPublicKey for structs::PublicKey {
    fn to_lib_public_key(&self) -> Result<PublicKey, ErrorInfo> {
        let b = self.bytes.safe_bytes()?;
        return PublicKey::from_slice(&b).map_err(|e| error_info(e.to_string()));
    }
}

impl structs::PublicKey {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        structs::PublicKey {
            bytes: bytes_data(bytes),
            key_type: PublicKeyType::Secp256k1 as i32
        }
    }

    pub fn from_hex<S: Into<String>>(hex: S) -> Result<structs::PublicKey, ErrorInfo> {
        let bytes = from_hex(hex.into())?;
        let key = Self::from_bytes(bytes);
        key.validate()?;
        Ok(key)
    }

    pub fn validate(&self) -> Result<&Self, ErrorInfo> {
        let _ = self.to_lib_public_key()?;
        Ok(self)
    }

    pub fn hex(&self) -> Result<String, ErrorInfo> {
        let b = self.bytes.safe_bytes()?;
        Ok(hex::encode(b))
    }
    pub fn bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        self.bytes.safe_bytes()
    }

    pub fn short_id(&self) -> String {
        self.hex().expect("hex").short_string().expect("worked")
    }
}
