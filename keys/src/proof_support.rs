use bdk::bitcoin::secp256k1::{PublicKey, SecretKey};
use log::info;
use redgold_schema::{EasyJson, error_info, error_message, from_hex, RgResult, SafeBytesAccess, SafeOption, signature_data, structs};
use redgold_schema::structs::{Address, ErrorInfo, Hash, NetworkEnvironment, Proof};
use crate::{KeyPair, TestConstants, util};
use crate::address_external::{ToBitcoinAddress, ToEthereumAddress};
use crate::util::{public_key_ser, ToPublicKey};

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
}

impl ProofSupport for Proof {
    fn verify(&self, hash: &Hash) -> RgResult<()> {
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
    fn new(hash: &Hash, secret: &SecretKey, public: &PublicKey) -> Proof {
        let signature = util::sign_hash(&hash, &secret).expect("signature works");
        return Proof {
            signature: signature_data(signature),
            public_key: public_key_ser(public),
        };
    }

    fn from_keypair(hash: &Vec<u8>, keypair: KeyPair) -> Proof {
        return Proof::new(
            &hash.clone().into(),
            &keypair.secret_key,
            &keypair.public_key,
        );
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
        if !all_addresses.contains(address) {
            return Err(error_message(
                structs::Error::AddressPublicKeyProofMismatch,
                "address mismatch in Proof::verify_proofs",
            ));
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
            Self::proofs_to_address(proofs).map(|a| vec![a])
        } else if proofs.len() == 1 {
            let proof = proofs.get(0).expect("proof");
            proof.public_key.safe_get_msg("Missing public key")?
                .to_all_addresses()
        } else {
            return Err(error_info("No proofs to convert to addresses"));
        }
    }

}


#[test]
fn verify_single_signature_proof() {
    let tc = TestConstants::new();
    let proof = Proof::new(&tc.hash_vec.clone().into(), &tc.secret, &tc.public);
    assert!(proof.verify(&Hash::new(tc.hash_vec)).is_ok());
}

#[test]
fn verify_invalid_single_signature_proof() {
    let tc = TestConstants::new();
    let mut proof = Proof::new(&tc.hash_vec.clone().into(), &tc.secret, &tc.public);
    proof.signature = signature_data(tc.hash_vec.clone());
    assert!(proof.verify( &Hash::new(tc.hash_vec)).is_err());
}

#[test]
fn verify_invalid_key_single_signature_proof() {
    let tc = TestConstants::new();
    let mut proof = Proof::new(&tc.hash_vec.clone().into(), &tc.secret, &tc.public);
    proof.public_key = public_key_ser(&tc.public2);
    assert!(proof.verify(&Hash::new(tc.hash_vec)).is_err());
}


pub trait PublicKeySupport {
    fn validate(&self) -> Result<&Self, ErrorInfo>;
    fn from_hex<S: Into<String>>(hex: S) -> Result<structs::PublicKey, ErrorInfo>;
    fn to_all_addresses(&self) -> RgResult<Vec<Address>>;
}

impl PublicKeySupport for structs::PublicKey {

    fn validate(&self) -> Result<&Self, ErrorInfo> {
        let _ = self.to_lib_public_key()?;
        Ok(self)
    }

    fn from_hex<S: Into<String>>(hex: S) -> Result<structs::PublicKey, ErrorInfo> {
        let bytes = from_hex(hex.into())?;
        let key = Self::from_bytes(bytes);
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
}

