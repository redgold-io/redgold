use crate::structs::{Address, Error as RGError, ErrorInfo, Hash, Proof};

use crate::{error_message, HashClear, signature_data, structs};

impl HashClear for Proof {
    // TODO: Separate the hashclear method for those that don't require clears
    fn hash_clear(&mut self) {}
}

impl Proof {
    pub fn public_key_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
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

    pub fn proofs_to_address(proofs: &Vec<Proof>) -> Result<Address, ErrorInfo> {
        let mut addresses = Vec::new();
        for proof in proofs {
            addresses.extend(proof.public_key_bytes()?);
        }
        let vec = Address::hash(&addresses);
        let addr = Address::from_bytes(vec)?;
        return Ok(addr);
    }


    pub fn from(public_key: structs::PublicKey, signature: structs::Signature) -> Self {
        Self {
            signature: Some(signature),
            public_key: Some(public_key),
        }
    }
}
