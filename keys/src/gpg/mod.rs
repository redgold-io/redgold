
use sequoia_openpgp::{
    cert::CertBuilder,
    packet::prelude::*,
    policy::StandardPolicy,
    types::*,
};
use chrono::{Duration, Utc};
use sequoia_openpgp::crypto::KeyPair;
use redgold_schema::RgResult;

pub fn gpg_from_entropy(entropy: &[u8; 32], user_name: String, email: Option<String>) -> RgResult<Vec<u8>> {
    // Generate one key pair to be used for everything
    let key_pair = KeyPair::generate_ecc(
        SignatureType::Binary,
        Curve::Ed25519,  // Ed25519 works well for all purposes
        entropy
    ).error_info("keypair generate")?;

    // Create certificate with user info, using the same key for all purposes
    let mut uid = user_name.clone();
    let cert = CertBuilder::new()
        .set_primary_key(key_pair.clone())
        .add_userid(&*uid)
        .set_creation_time(Utc::now())
        .set_validity_period(Some(Duration::days(365 * 2)))
        // Add same key as subkey with all capabilities
        .add_subkey(key_pair.clone(),
                    KeyFlags::empty()
                        .set_encrypt_communications()
                        .set_encrypt_storage()
                        .set_sign_data()
                        .set_authenticate(),
                    None)
        .generate().error_info("certificate generate")?;

    // Export for GPG
    let mut out = Vec::new();
    cert.serialize(&mut out)?;
    Ok(out)
}