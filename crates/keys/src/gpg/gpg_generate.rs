use redgold_schema::RgResult;
use sequoia_openpgp::packet::key::Key4;
use sequoia_openpgp::packet::prelude::SignatureBuilder;
use sequoia_openpgp::packet::UserID;
use sequoia_openpgp::serialize::Marshal;
use sequoia_openpgp::types::{Features, HashAlgorithm, KeyFlags, SignatureType, SymmetricAlgorithm};
use sequoia_openpgp::{Cert, Packet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::TestConstants;

fn system_time_from_millis(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}

// #[cfg(feature = "gpg")]
pub fn gpg_from_entropy(entropy: &[u8; 32], user_name: String, email: Option<String>) -> RgResult<Vec<u8>> {

    let reproducible_time = 1731180346404u64;
    let creation_time = system_time_from_millis(reproducible_time);

    let secret_key = Key4::import_secret_ed25519(entropy, creation_time)
        .expect("import secret ed25519");

    let uid = if let Some(email) = email {
        format!("{} <{}>", user_name, email)
    } else {
        user_name
    };

    // Set flags for the primary key
    let flags = KeyFlags::empty()
        .set_certification()
        .set_signing();

    let sig = sig_builder(creation_time, &flags);

    let mut signer = secret_key.clone().into_keypair()
        .expect("key generated above has a secret");

    // Create the certificate directly from the secret key packet
    let cert = Cert::try_from(vec![
        Packet::SecretKey(secret_key.into())
    ]).expect("creating cert from secret key");
    
    let sig = sig.sign_direct_key(&mut signer, cert.primary_key().key()).expect("sign direct");

    let mut acc = vec![
        Packet::from(sig),
    ];

    // Create a new signature builder specifically for the UserID binding
    let uid_sig = SignatureBuilder::new(SignatureType::PositiveCertification)
        .set_hash_algo(HashAlgorithm::SHA512)
        .set_signature_creation_time(creation_time).expect("signature creation time")
        .set_features(Features::sequoia()).expect("features")
        .set_key_flags(flags.clone()).expect("key flags")
        .set_primary_userid(true).expect("primary userid");

    let user_id: UserID = uid.clone().into();
    let signature = user_id.bind(&mut signer, &cert, uid_sig).expect("bind");
    acc.push(user_id.into());
    acc.push(signature.into());

    // Now add the new components and canonicalize once.
    let cert = cert.insert_packets(acc).expect("insert packets");

    // Export for GPG
    let mut out = Vec::new();
    cert.serialize(&mut out).expect("serialize");
    Ok(out)
}

// #[cfg(feature = "gpg")]
fn sig_builder(creation_time: SystemTime, flags: &KeyFlags) -> SignatureBuilder {
    SignatureBuilder::new(SignatureType::DirectKey)
        // GnuPG wants at least a 512-bit hash for P521 keys.
        .set_hash_algo(HashAlgorithm::SHA512)
        .set_signature_creation_time(creation_time).expect("signature creation time")
        .set_features(Features::sequoia()).expect("signature creation time")
        .set_key_flags(flags.clone()).expect("signature set_key_flags time")
        .set_key_validity_period(None).expect("signature set_key_validity_period time")
        .set_preferred_hash_algorithms(vec![
            HashAlgorithm::SHA512,
            HashAlgorithm::SHA256,
        ]).expect("signature set_preferred_hash_algorithms time")
        .set_preferred_symmetric_algorithms(vec![
            SymmetricAlgorithm::AES256,
            SymmetricAlgorithm::AES128,
        ]).expect("signature set_preferred_symmetric_algorithms time")
}

use sequoia_openpgp::cert::CertBuilder;
use crate::solana::derive_solana::SolanaWordPassExt;

// #[cfg(feature = "gpg")]
#[test]
pub fn gpg_test() {
    let tc = TestConstants::new();
    let sk = tc.words_pass.derive_solana_keys().unwrap();
    let entropy = sk.0.to_scalar_bytes();
    // let entropy = tc.secret.secret_bytes();
    let user_name = "test".to_string();
    let email = Some("test@test.com".to_string());
    let out = gpg_from_entropy(&entropy, user_name, email).expect("gpg_from_entropy");
    println!("out: {:?}", out);
}
