use redgold_schema::RgResult;
use sequoia_openpgp::packet::key::Key4;
use sequoia_openpgp::packet::prelude::SignatureBuilder;
use sequoia_openpgp::packet::UserID;
use sequoia_openpgp::serialize::Marshal;
use sequoia_openpgp::types::{Features, HashAlgorithm, KeyFlags, SignatureType, SymmetricAlgorithm};
use sequoia_openpgp::{Cert, Packet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn system_time_from_millis(millis: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(millis)
}
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
    // secret_key.parts_as_public()
    let sig = sig.sign_direct_key(&mut signer, cert.primary_key().key()).expect("sign direct");

    let mut acc = vec![
        Packet::from(sig),
    ];

    let mut sig = sig_builder(creation_time, &flags);

    let user_id: UserID = uid.clone().into();

    sig = sig.set_primary_userid(true).expect("primary userid");

    let signature = user_id.bind(&mut signer, &cert, sig).expect("bind");
    acc.push(user_id.into());
    acc.push(signature.into());
    // cert.with_policy()

    // Now add the new components and canonicalize once.
    let cert = cert.insert_packets(acc).expect("insert packets");


    // Create the certificate builder
    // let mut builder = CertBuilder::new()
    //     .add_userid(uid)
    //     .set_creation_time(creation_time)
    //     .set_validity_period(None)
    //     .set_primary_key_flags(flags)
    //     .generate();

    // Export for GPG
    let mut out = Vec::new();
    cert.serialize(&mut out).expect("serialize");
    Ok(out)
}

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

