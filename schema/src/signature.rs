use crate::bytes_data;
use crate::structs::{Signature, SignatureType};

impl Signature {
    pub fn ecdsa(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            signature_type: SignatureType::Ecdsa as i32,
        }
    }
    pub fn hardware(bytes: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(bytes),
            signature_type: SignatureType::EcdsaBitcoinSignMessageHardware as i32,
        }
    }
}