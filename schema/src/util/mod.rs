use std::io::Cursor;

use bitcoin::hashes::{Hash, hash160};
use bitcoin::hashes::hex::ToHex;
use bitcoin::secp256k1::{Error, Message, PublicKey, Secp256k1, SecretKey, Signature};
use bitcoin::util::base58;
use bitcoin::util::bip158::{BitStreamReader, BitStreamWriter};
use crypto::digest::Digest;
use crypto::sha2::{Sha256, Sha512};

use crate::{bytes_data, error_message, ErrorInfo, ErrorInfoContext, SafeBytesAccess, structs, TestConstants};
use crate::structs::{Error as RGError, PublicKeyType};

// TODO: Should we just switch to truncated SHA512?

// https://bitcoin.stackexchange.com/questions/8443/where-is-double-hashing-performed-in-bitcoin
/*
To avoid this property, Ferguson and Schneier suggested using SHA256d = SHA256(SHA256(x))
 which avoids length-extension attacks. This construction has some minor weaknesses
  (not relevant to bitcoin), so I wouldn't recommend it for new protocols,
  and would use HMAC with constant key, or truncated SHA512 instead.
 */

// TODO: Replace with our own signature type
pub fn sign(hash: &Vec<u8>, key: &SecretKey) -> Result<Vec<u8>, ErrorInfo> {
    let mut ret = [0u8; 32];
    ret[..].copy_from_slice(&hash[0..32]);
    let message = Message::from_slice(&ret).map_err(|e| {
        error_message(
            structs::Error::IncorrectSignature,
            format!("Signature message construction failure {}", e.to_string()),
        )
    })?;
    let signature: Signature = Secp256k1::new().sign(&message, &key);
    let sig_ser = signature.serialize_compact().to_vec();
    return Ok(sig_ser);
}

// TODO: Change to our own signature type
pub fn sign_hash(hash: &structs::Hash, key: &SecretKey) -> Result<Vec<u8>, ErrorInfo> {
    sign(&hash.safe_bytes().expect("bytes"), &key)
}

#[test]
fn test_sign() {
    let tc: TestConstants = TestConstants::new();
    let sig = sign(&tc.hash_vec, &tc.secret).expect("worked");
    println!("{}", hex::encode(sig.clone()));
    assert_eq!(
        hex::decode("de287f019fbab3621d6604d800d3ed102afc5c49ac2be25f8eb677987072109f232508b061942cfbd1fd2c7e18a172a33ca8b6ad3739b410b01d18ed85bc25bb").unwrap(),
        sig
    );
}

pub fn verify(hash: &Vec<u8>, signature: &Vec<u8>, public_key: &Vec<u8>) -> Result<(), ErrorInfo> {
    let mut ret = [0u8; 32];

    ret[..].copy_from_slice(&hash[0..32]);
    let message = Message::from_slice(&ret).error_msg(
            structs::Error::IncorrectSignature,
            "Signature message construction failure",
    )?;
    let decoded_signature = Signature::from_compact(signature).error_msg(
            structs::Error::IncorrectSignature,
            "Decoded signature construction failure",
        )?;
    let result = PublicKey::from_slice(public_key).error_msg(
            structs::Error::IncorrectSignature,
            "Public key construction failure",
    )?;
    Secp256k1::new()
        .verify(&message, &decoded_signature, &result)
        .error_msg(
                structs::Error::IncorrectSignature,
                "Signature verification failure"
        )?;
    return Ok(());
}

#[test]
fn test_verify() {
    let sig = "de287f019fbab3621d6604d800d3ed102afc5c49ac2be25f8eb677987072109f232508b061942cfbd1fd2c7e18a172a33ca8b6ad3739b410b01d18ed85bc25bb";
    let tc: TestConstants = TestConstants::new();
    assert!(verify(
        &tc.hash_vec,
        &hex::decode(sig).unwrap(),
        &tc.public.serialize().to_vec()
    )
    .is_ok());
}

#[test]
fn sign_and_verify() {
    let tc: TestConstants = TestConstants::new();
    let sig = sign(&tc.hash_vec, &tc.secret);
    assert!(verify(
        &tc.hash_vec,
        &sig.expect("a"),
        &tc.public.serialize().to_vec()
    )
    .is_ok());
}

pub fn public_key_ser(public_key: &PublicKey) -> Option<crate::structs::PublicKey> {
    Some(structs::PublicKey {
        bytes: bytes_data(public_key.serialize().to_vec()),
        key_type: crate::structs::PublicKeyType::Secp256k1 as i32,
    })
}

pub mod mnemonic_builder;
pub mod mnemonic_words;
pub mod merkle;
pub mod bitcoin_message_signer;
pub mod btc_wallet;
pub mod keys;
pub mod mnemonic_support;
pub mod xor_distance;

pub fn sha256(s: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    let mut sha2 = Sha256::new();
    sha2.input(s);
    sha2.result(&mut hash);
    return hash;
}

pub fn sha512(s: &[u8]) -> [u8; 64] {
    let mut hash = [0u8; 64];
    let mut sha2 = Sha512::new();
    sha2.input(s);
    sha2.result(&mut hash);
    return hash;
}

pub fn sha256_str(s: &str) -> [u8; 32] {
    return sha256(s.as_bytes());
}

pub fn sha256_vec(s: &Vec<u8>) -> [u8; 32] {
    return sha256(s);
}

pub fn dhash(input: &[u8]) -> [u8; 32] {
    let s = sha256(input);
    return sha256(&s);
}

pub fn dhash_str(input: &str) -> [u8; 32] {
    return dhash(input.as_bytes());
}

pub fn dhash_vec(input: &Vec<u8>) -> [u8; 32] {
    return dhash(input);
}

#[test]
fn test_dhash() {
    let expected = "fe9a32f5b565da46af951e4aab23c24b8c1565eb0b6603a03118b7d225a21e8c";
    assert_eq!(expected, hex::encode(dhash_str("asdf")));
    assert_eq!(expected, hex::encode(dhash("asdf".as_bytes())));
    assert_eq!(
        expected,
        hex::encode(dhash_vec(&"asdf".as_bytes().to_vec()))
    );
}

pub fn merge_hash(left: [u8; 32], r: [u8; 32]) -> [u8; 32] {
    let mut merged = left.to_vec();
    merged.extend(r.to_vec());
    let parent_hash = dhash_vec(&merged);
    return parent_hash;
}

pub fn checksum(data: &[u8]) -> Vec<u8> {
    let mut hash = [0u8; 32];
    let mut checksum = Vec::new();
    let mut writer = BitStreamWriter::new(&mut checksum);

    let mut sha2 = Sha256::new();
    sha2.input(data);
    sha2.result(&mut hash);
    let mut check_cursor = Cursor::new(&hash);
    let mut check_reader = BitStreamReader::new(&mut check_cursor);
    for _ in 0..data.len() / 4 {
        writer.write(check_reader.read(1).unwrap(), 1).unwrap();
    }
    writer.flush().unwrap();
    checksum
}

pub fn current_time_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub fn make_ascii_titlecase(s: &mut str) -> String {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    return s.to_string();
}
