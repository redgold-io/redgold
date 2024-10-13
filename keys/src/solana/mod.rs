use bdk::bitcoin::util::base58;
use ed25519_dalek::{SigningKey, VerifyingKey};
use crate::TestConstants;
// We'll use this for generating random bytes

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

pub fn get_solana_address(public_key: &[u8; 32]) -> String {
    base58::encode_slice(public_key)
}

#[test]
fn debug_kg() {
    let tc = TestConstants::new();
    let wp = tc.words_pass;
    // wp.xpub()
    // // In a real scenario, you'd derive this seed from your mnemonic and path
    // let mut seed = [0u8; 32];
    // OsRng.fill_bytes(&mut seed);
    //
    // let (signing_key, verifying_key) = generate_solana_keypair(&seed);
    //
    // println!("Private key: {:?}", signing_key.to_bytes());
    // println!("Public key: {:?}", verifying_key.to_bytes());
}