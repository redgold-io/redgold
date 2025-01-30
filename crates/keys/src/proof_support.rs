use crate::address_external::{ToBitcoinAddress, ToEthereumAddress};
use crate::util::{public_key_ser, ToPublicKey};
use crate::{btc, util, KeyPair};
use bdk::bitcoin::secp256k1::{PublicKey, SecretKey};
use itertools::Itertools;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, ErrorInfo, Hash, NetworkEnvironment, Proof, SupportedCurrency};
use redgold_schema::{error_info, error_message, from_hex, signature_data, structs, RgResult, SafeOption};
use std::collections::HashMap;

pub trait ProofSupport {
    fn verify(&self, hash: &Hash) -> RgResult<()>;
    fn new(hash: &Hash, secret: &SecretKey, public: &PublicKey) -> Proof;
    fn from_keypair(hash: &Vec<u8>, keypair: KeyPair) -> Proof;
    fn from_keypair_hash(hash: &Hash, keypair: &KeyPair) -> Proof;
    fn verify_proofs(
        proofs: &Vec<Proof>,
        hash: &Hash,
        address: &Address,
    ) -> Result<(), ErrorInfo>;
    fn public_key(&self) -> RgResult<structs::PublicKey>;
    fn proofs_to_addresses(proofs: &Vec<Proof>) -> RgResult<Vec<Address>>;
    fn verify_inner(&self, hash: &Hash) -> RgResult<()>;
}

impl ProofSupport for Proof {
    fn verify(&self, hash: &Hash) -> RgResult<()> {
        self.verify_inner(&hash)
            .with_detail("hash", &hash.json_or())
            .with_detail("public_key", &self.public_key.json_or())
            .with_detail("signature", &self.signature.json_or())
    }
    fn new(hash: &Hash, secret: &SecretKey, public: &PublicKey) -> Proof {
        let signature = util::sign_hash(&hash, &secret).expect("signature works");
        return Proof {
            signature: signature_data(signature),
            public_key: public_key_ser(public),
        };
    }
    fn from_keypair(hash: &Vec<u8>, keypair: KeyPair) -> Proof {
        let secret = &keypair.secret_key;
        let public = &keypair.public_key;
        let signature = util::sign(&hash, &secret).expect("signature works");
        return Proof {
            signature: signature_data(signature),
            public_key: public_key_ser(public),
        };
    }

    fn from_keypair_hash(hash: &Hash, keypair: &KeyPair) -> Proof {
        return Proof::new(
            &hash,
            &keypair.secret_key,
            &keypair.public_key,
        );
    }

    fn verify_proofs(
        proofs: &Vec<Proof>,
        hash: &Hash,
        address: &Address,
    ) -> Result<(), ErrorInfo> {
        let all_addresses = Self::proofs_to_addresses(proofs)?;
        let proof_1 = proofs.get(0).expect("exists").clone();
        if !all_addresses.contains(address) {
            return Err(error_message(
                structs::ErrorCode::AddressPublicKeyProofMismatch,
                "address mismatch in Proof::verify_proofs",
            ))
                .with_detail("all_addresses", all_addresses.json_or())
                .with_detail("all_addresses_hex_lengths",
                             all_addresses
                                 .iter()
                                 .map(|a| a.render_string().map(|r| r.len()).unwrap_or(0))
                                 .collect_vec()
                                 .json_or()
                )
                .with_detail("address", &address.json_or())
                .with_detail("address_hex_len", address.hex().len().to_string())
                .with_detail("proof_1", &proof_1.json_or())
            ;
        }
        for proof in proofs {
            proof.verify(hash)?
        }
        return Ok(());
    }

    fn public_key(&self) -> RgResult<structs::PublicKey> {
        // TODO: RSV Signature public key reconstruction
        // self.signature.safe_get()?.;
        self.public_key.clone().ok_msg("Missing public key")
    }

    fn proofs_to_addresses(proofs: &Vec<Proof>) -> RgResult<Vec<Address>> {
        if proofs.len() > 1 {
            // Multisig support only for RDG address types, not external address types.
            Self::multi_proofs_to_address(proofs).map(|a| vec![a])
        } else if proofs.len() == 1 {
            let proof = proofs.get(0).expect("proof");
            proof.public_key.safe_get_msg("Missing public key")?
                .to_all_addresses()
        } else {
            return Err(error_info("No proofs to convert to addresses"));
        }
    }

    fn verify_inner(&self, hash: &Hash) -> RgResult<()> {
        let sig = self.signature.safe_get()?;
        let verify_hash = match sig.signature_type {
            // SignatureType::Ecdsa
            0 => {
                hash.raw_bytes()?
            }
            // SignatureType::EcdsaBitcoinSignMessageHardware
            1 => {
                btc::bitcoin_message_signer::prepare_message_sign_hash(&hash)
            }
            _ => {
                return Err(error_info(
                    "Invalid signature type",
                ));
            }
        };
        return util::verify(&verify_hash, &self.signature_bytes()?, &self.public_key_direct_bytes()?);
    }

}



#[cfg(test)]
mod test {
    use redgold_schema::signature_data;
    use redgold_schema::structs::Proof;
    use crate::proof_support::ProofSupport;
    use crate::TestConstants;
    use crate::util::public_key_ser;

    #[test]
fn verify_single_signature_proof() {
    let tc = TestConstants::new();

    let proof = Proof::new(&tc.rhash_1, &tc.secret, &tc.public);
    assert!(proof.verify(&tc.rhash_1).is_ok());
}

#[test]
fn verify_invalid_single_signature_proof() {
    let tc = TestConstants::new();
    let mut proof = Proof::new(&tc.rhash_1, &tc.secret, &tc.public);
    proof.signature = signature_data(tc.hash_vec.clone());
    assert!(proof.verify( &tc.rhash_1).is_err());
}

#[test]
fn verify_invalid_key_single_signature_proof() {
    let tc = TestConstants::new();
    let mut proof = Proof::new(&tc.rhash_1, &tc.secret, &tc.public);
    proof.public_key = public_key_ser(&tc.public2);
    assert!(proof.verify(&tc.rhash_1).is_err());
}

}


pub trait PublicKeySupport {
    fn validate(&self) -> Result<&Self, ErrorInfo>;
    fn from_direct_ecdsa_hex<S: Into<String>>(hex: S) -> Result<structs::PublicKey, ErrorInfo>;
    fn to_all_addresses(&self) -> RgResult<Vec<Address>>;
    fn to_all_addresses_for_network(&self, network: &NetworkEnvironment) -> RgResult<Vec<Address>>;
    fn to_all_addresses_for_network_by_currency(&self, network: &NetworkEnvironment) -> RgResult<HashMap<SupportedCurrency, Address>>;
}

impl PublicKeySupport for structs::PublicKey {

    fn validate(&self) -> Result<&Self, ErrorInfo> {
        let _ = self.to_lib_ecdsa_public_key()?;
        Ok(self)
    }

    fn from_direct_ecdsa_hex<S: Into<String>>(hex: S) -> Result<structs::PublicKey, ErrorInfo> {
        let bytes = from_hex(hex.into())?;
        let key = Self::from_bytes_direct_ecdsa(bytes);
        key.validate()?;
        Ok(key)
    }

    fn to_all_addresses(&self) -> RgResult<Vec<Address>> {
        let default = self.address()?;
        let eth = self.to_ethereum_address_typed()?;
        let btc_test = self.to_bitcoin_address_typed(&NetworkEnvironment::Dev)?;
        let btc_main = self.to_bitcoin_address_typed(&NetworkEnvironment::Main)?;
        Ok(vec![default, eth, btc_test, btc_main])
    }

    fn to_all_addresses_for_network(&self, network: &NetworkEnvironment) -> RgResult<Vec<Address>> {
        let default = self.address()?;
        let eth = self.to_ethereum_address_typed()?;
        let btc = self.to_bitcoin_address_typed(&network)?;
        Ok(vec![default, eth, btc])
    }

    fn to_all_addresses_for_network_by_currency(&self, network: &NetworkEnvironment) -> RgResult<HashMap<SupportedCurrency, Address>> {
        let default = self.address()?;
        let eth = self.to_ethereum_address_typed()?;
        let btc = self.to_bitcoin_address_typed(&network)?;
        let mut hm = HashMap::new();
        hm.insert(SupportedCurrency::Redgold, default);
        hm.insert(SupportedCurrency::Ethereum, eth);
        hm.insert(SupportedCurrency::Bitcoin, btc);
        Ok(hm)
    }

}

