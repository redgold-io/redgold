use crate::util::mnemonic_support::MnemonicSupport;
use crate::TestConstants;
use bdk::bitcoin::util::base58;
use ed25519_dalek::{SigningKey, VerifyingKey};
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::structs::{AddressType, ErrorInfo, PublicKey};
use redgold_schema::util::lang_util::AnyPrinter;
use redgold_schema::{structs, RgResult};
// We'll use this for generating random bytes

pub trait SolanaWordPassExt {
    fn solana_seed_bytes(&self) -> RgResult<[u8; 32]>;
    fn derive_solana_keys(&self) -> RgResult<(SigningKey, VerifyingKey)>;
    fn derive_solana_keys_at(&self, change: i64) -> RgResult<(SigningKey, VerifyingKey)>;
    fn derive_solana_public_key(&self) -> RgResult<structs::PublicKey>;
    fn derive_solana_public_key_at(&self, change: i64) -> RgResult<structs::PublicKey>;
    fn solana_address(&self) -> RgResult<structs::Address>;
    fn derive_seed_at_change(&self, change: &i64) -> Result<[u8; 32], ErrorInfo>;
}


pub trait ToSolanaAddress {
    fn to_solana_address(&self) -> RgResult<structs::Address>;
}

impl ToSolanaAddress for structs::PublicKey {
    fn to_solana_address(&self) -> RgResult<structs::Address> {
        let public_key = self.raw_bytes()?;
        let address = get_solana_address(public_key);
        Ok(structs::Address::from_type(&address, AddressType::SolanaExternalString))
    }
}

pub fn generate_solana_keypair(seed: [u8; 32]) -> (SigningKey, VerifyingKey) {
    // Use the seed to create a SecretKey
    let secret = seed;

    // Create a SigningKey from the SecretKey
    let signing_key = SigningKey::from_bytes(&secret);

    // Derive the VerifyingKey (public key) from the SigningKey
    let verifying_key = signing_key.verifying_key();

    (signing_key, verifying_key)
}

pub fn get_solana_public_key(verifying_key: &VerifyingKey) -> [u8; 32] {
    verifying_key.to_bytes()
}

pub fn get_solana_address(public_key: Vec<u8>) -> String {
    base58::encode_slice(&*public_key)
}
pub fn get_solana_address_from_verifying(verifying_key: &VerifyingKey) -> String {
    get_solana_address(verifying_key.to_bytes().to_vec())
}

impl SolanaWordPassExt for WordsPass {
    fn solana_seed_bytes(&self) -> RgResult<[u8; 32]> {
        self.derive_seed_at_path("m/44'/501'/0'/0'")
    }

    fn derive_solana_keys(&self) -> RgResult<(SigningKey, VerifyingKey)> {
        let seed = self.solana_seed_bytes()?;
        Ok(generate_solana_keypair(seed))
    }

    fn derive_solana_keys_at(&self, change: i64) -> RgResult<(SigningKey, VerifyingKey)> {
        let seed = self.derive_seed_at_change(&change)?;
        Ok(generate_solana_keypair(seed))
    }

    fn derive_solana_public_key(&self) -> RgResult<structs::PublicKey> {
        let (_, verifying) = self.derive_solana_keys()?;
        Ok(structs::PublicKey::from_bytes_direct_ed25519(verifying.to_bytes().to_vec()))
    }

    fn derive_solana_public_key_at(&self, change: i64) -> RgResult<PublicKey> {
        self.derive_seed_at_path(&format!("m/44'/501'/0'/0'/{}", change))
            .and_then(|seed| {
                let (_, verifying) = generate_solana_keypair(seed);
                Ok(PublicKey::from_bytes_direct_ed25519(verifying.to_bytes().to_vec()))
            })
    }

    fn solana_address(&self) -> RgResult<structs::Address> {
        self.derive_solana_public_key()?.to_solana_address()
    }

    fn derive_seed_at_change(&self, change: &i64) -> Result<[u8; 32], ErrorInfo> {
        self.derive_seed_at_path(&format!("m/44'/501'/0'/0'/{}", change))
    }
}


#[ignore]
#[test]
fn debug_kg() {
    let tc = TestConstants::new();
    let wp = tc.words_pass;

    wp.derive_solana_public_key().unwrap().to_solana_address().unwrap().render_string().unwrap().print();
    wp.derive_solana_public_key_at(0).unwrap().to_solana_address().unwrap().render_string().unwrap().print();
    wp.derive_solana_public_key_at(1).unwrap().to_solana_address().unwrap().render_string().unwrap().print();

}