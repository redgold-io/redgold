use crate::structs::{Address, ErrorCode as RGError, ErrorInfo, Hash, Proof};

use crate::{error_message, HashClear, RgResult, SafeOption, signature_data, structs};
use crate::proto_serde::ProtoSerde;

impl HashClear for Proof {
    // TODO: Separate the hashclear method for those that don't require clears
    fn hash_clear(&mut self) {}
}

impl Proof {

    pub fn signature_hex(&self) -> RgResult<String> {
        Ok(hex::encode(self.signature_bytes()?))
    }
    // pub fn public_key_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
    //     // Ok(self.public_key.safe_get_msg("Missing public key")?.bytes()?)
    //     Ok(self
    //         .public_key
    //         .as_ref()
    //         .ok_or(error_message(RGError::MissingField, "public_key"))?
    //         .clone()
    //         .bytes
    //         .as_ref()
    //         .ok_or(error_message(
    //             RGError::MissingField,
    //             "public_key bytes data",
    //         ))?
    //         .value
    //         .clone())
    // }
    pub fn public_key_direct_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        // Ok(self.public_key.safe_get_msg("Missing public key")?.bytes()?)
        Ok(self
            .public_key
            .as_ref()
            .ok_or(error_message(RGError::MissingField, "public_key"))?
            .clone()
            .bytes
            .as_ref()
            .ok_or(error_message(
                RGError::MissingField,
                "public_key bytes data",
            ))?
            .value
            .clone())
    }
    pub fn public_key_proto_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.public_key.safe_get_msg("Missing public key")?.vec())
    }

    pub fn signature_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self
            .signature
            .as_ref()
            .ok_or(error_message(RGError::MissingField, "signature"))?
            .clone()
            .bytes
            .as_ref()
            .ok_or(error_message(RGError::MissingField, "signature bytes data"))?
            .value
            .clone())
    }

    pub fn multi_proofs_to_address(proofs: &Vec<Proof>) -> Result<Address, ErrorInfo> {
        let mut addresses = Vec::new();
        for proof in proofs {
            addresses.extend(proof.public_key_proto_bytes()?);
        }
        let addr = Address::from_byte_calculate(&addresses)?;
        return Ok(addr);
    }


    pub fn from(public_key: structs::PublicKey, signature: structs::Signature) -> Self {
        Self {
            signature: Some(signature),
            public_key: Some(public_key),
        }
    }
}
