use crate::structs::{Address, Error as RGError, ErrorInfo, Hash, Proof, Signature, SignatureType};
use crate::util::public_key_ser;
#[cfg(test)]
use crate::TestConstants;
use crate::{error_message, signature_data, util, HashClear, KeyPair, structs, SafeBytesAccess, SafeOption, error_info};
use bitcoin::secp256k1::{PublicKey, SecretKey};

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
    //
    // pub fn verify(&self, hash: &Vec<u8>) -> Result<(), ErrorInfo> {
    //     // TODO: Flatten missing options
    //     return util::verify(hash, &self.signature_bytes()?, &self.public_key_bytes()?);
    // }

    pub fn verify(&self, hash: &Hash) -> Result<(), ErrorInfo> {
        let sig = self.signature.safe_get()?;
        let verify_hash = match sig.signature_type {
            // SignatureType::Ecdsa
            0 => {
                hash.safe_bytes()?
            }
            // SignatureType::EcdsaBitcoinSignMessageHardware
            1 => {
                util::bitcoin_message_signer::prepare_message_sign(hash.hex())
            }
            _ => {
                return Err(error_info(
                    "Invalid signature type",
                ));
            }
        };
        return util::verify(&verify_hash, &self.signature_bytes()?, &self.public_key_bytes()?);
    }

    pub fn new(hash: &Hash, secret: &SecretKey, public: &PublicKey) -> Proof {
        let signature = util::sign_hash(&hash, &secret).expect("signature works");
        return Proof {
            signature: signature_data(signature),
            public_key: public_key_ser(public),
        };
    }

    pub fn from_keypair(hash: &Vec<u8>, keypair: KeyPair) -> Proof {
        return Proof::new(
            &hash.clone().into(),
            &keypair.secret_key,
            &keypair.public_key,
        );
    }

    pub fn from_keypair_hash(hash: &Hash, keypair: &KeyPair) -> Proof {
        return Proof::new(
            &hash,
            &keypair.secret_key,
            &keypair.public_key,
        );
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

    pub fn verify_proofs(
        proofs: &Vec<Proof>,
        hash: &Hash,
        address: &Address,
    ) -> Result<(), ErrorInfo> {
        let addr = Self::proofs_to_address(proofs)?;
        if *address != addr {
            return Err(error_message(
                RGError::AddressPublicKeyProofMismatch,
                "address mismatch in Proof::verify_proofs",
            ));
        }
        for proof in proofs {
            proof.verify(hash)?
        }
        return Ok(());
    }

    pub fn from(public_key: structs::PublicKey, signature: structs::Signature) -> Self {
        Self {
            signature: Some(signature),
            public_key: Some(public_key),
        }
    }
    // pub fn public_key(&self) -> Result<structs::PublicKey, ErrorInfo> {
    //     self.public_key.safe_get()
    // }
}

#[test]
fn verify_single_signature_proof() {
    let tc = TestConstants::new();
    let proof = Proof::new(&tc.hash_vec.clone().into(), &tc.secret, &tc.public);
    assert!(proof.verify(&Hash::from_bytes(tc.hash_vec)).is_ok());
}

#[test]
fn verify_invalid_single_signature_proof() {
    let tc = TestConstants::new();
    let mut proof = Proof::new(&tc.hash_vec.clone().into(), &tc.secret, &tc.public);
    proof.signature = signature_data(tc.hash_vec.clone());
    assert!(proof.verify( &Hash::from_bytes(tc.hash_vec)).is_err());
}

#[test]
fn verify_invalid_key_single_signature_proof() {
    let tc = TestConstants::new();
    let mut proof = Proof::new(&tc.hash_vec.clone().into(), &tc.secret, &tc.public);
    proof.public_key = public_key_ser(&tc.public2);
    assert!(proof.verify(&Hash::from_bytes(tc.hash_vec)).is_err());
}
