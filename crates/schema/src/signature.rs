use crate::structs::{Signature, SignatureType};
use crate::{bytes_data, RgResult, SafeOption};

impl Signature {
    pub fn ecdsa(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            signature_type: SignatureType::Ecdsa as i32,
            rsv: None
        }
    }
    pub fn hardware(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            signature_type: SignatureType::EcdsaBitcoinSignMessageHardware as i32,
            rsv: None
        }
    }
    pub fn raw_bytes(&self) -> RgResult<Vec<u8>> {
        Ok(self.bytes.safe_get()?.value.clone())
    }
}