#![allow(dead_code)]

pub mod auto_update;
pub mod base26;
pub mod cmd;
pub mod lang_util;
#[cfg(not(target_arch = "wasm32"))]
pub mod metrics_registry;
pub mod rg_merkle;
pub mod runtimes;
pub mod sym_crypt;
pub mod ip_lookup;
pub mod cli;
pub mod hashviz;
pub mod merkle;
pub mod keys;
pub mod logging;

use std::io::{Cursor, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::node_config::NodeConfig;
use crate::schema::address::address;
use crate::schema::structs;
use crate::schema::structs::{
    BytesData, BytesDecoder, ErrorInfo, KeyType, PublicKeyType, PublicResponse, SignatureType,
};
use crate::schema::SafeBytesAccess;
use bitcoin::hashes::hex::ToHex;
use bitcoin::hashes::{Hash, hash160};
use bitcoin::secp256k1::{Error, Message, PublicKey, Secp256k1, SecretKey, Signature};
use bitcoin::util::base58;
use bitcoin::util::bip158::{BitStreamReader, BitStreamWriter};
use crypto::digest::Digest;
use crypto::sha2::{Sha256, Sha512};
use libp2p::core::identity::{Keypair, secp256k1};
use libp2p::core::PublicKey as LPublicKey;
use libp2p::PeerId;
use log::SetLoggerError;
use log4rs::config::Logger;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Handle;
use rand::rngs::OsRng;
use rand::RngCore;
use redgold_schema::TestConstants;
use redgold_schema::util::{dhash_str, sign};
use redgold_schema::util::wallet::{generate_key, generate_key_i};

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

#[test]
fn test_sha256() {
    let expected = "f0e4c2f76c58916ec258f246851bea091d14d4247a2fc3e18694461b1816e13b";
    assert_eq!(expected, hex::encode(sha256_str("asdf")));
    assert_eq!(expected, hex::encode(sha256("asdf".as_bytes())));
    assert_eq!(
        expected,
        hex::encode(sha256_vec(&"asdf".as_bytes().to_vec()))
    );
}

pub fn current_time_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

pub fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn current_time_millis_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub fn current_time_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn init_logger_with_config(node_config: NodeConfig) -> Result<Handle, SetLoggerError> {
    use log::LevelFilter;
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};

    let stdout = ConsoleAppender::builder().build();

    let log_path = format!("{}/{}", node_config.data_store_folder(), "log/redgold.log");
    println!("Starting log config with log path {}", log_path);

    let root_redgold = FileAppender::builder()
        // .encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
        .build(log_path)
        .expect("log path no bueno");

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("redgold", Box::new(root_redgold)))
        .logger(Logger::builder().build("sqlx", LevelFilter::Warn))
        // .logger(Logger::builder().build("app::backend::db", LevelFilter::Info))
        // .logger(
        //     Logger::builder()
        //         .appender("requests")
        //         .additive(false)
        //         .build("app::requests", LevelFilter::Info),
        // )
        .build(
            Root::builder()
                .appenders(vec!["stdout", "redgold"])
                .build(LevelFilter::Info), //.appender("redgold").build(LevelFilter::Warn)
        )
        .unwrap();

    log4rs::init_config(config)
}

pub fn init_logger() -> Result<Handle, SetLoggerError> {
    use log::LevelFilter;
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};

    let stdout = ConsoleAppender::builder().build();

    let root_redgold = FileAppender::builder()
        //.encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
        .build("../../log/redgold.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("redgold", Box::new(root_redgold)))
        .logger(Logger::builder().build("sqlx", LevelFilter::Warn))
        .logger(Logger::builder().build("redgold", LevelFilter::Debug))
        // .logger(Logger::builder().build("app::backend::db", LevelFilter::Info))
        // .logger(
        //     Logger::builder()
        //         .appender("requests")
        //         .additive(false)
        //         .build("app::requests", LevelFilter::Info),
        // )
        .build(
            Root::builder()
                .appenders(vec!["stdout", "redgold"])
                .build(LevelFilter::Info), //.appender("redgold").build(LevelFilter::Warn)
        )
        .unwrap();

    log4rs::init_config(config)
}

pub trait Short {
    fn short_id(&self) -> String;
}

impl Short for PeerId {
    fn short_id(&self) -> String {
        let string = self.to_base58();
        return string[(string.len() - 5)..string.len()].to_string();
    }
}

pub fn to_libp2p_kp(s: &SecretKey) -> Keypair {
    let hex_dec = hex::decode(s.to_hex()).unwrap();
    let s4 = secp256k1::SecretKey::from_bytes(hex_dec).unwrap();
    let kp1 = secp256k1::Keypair::from(s4);
    let kp2 = Keypair::Secp256k1(kp1);
    return kp2;
}

pub fn to_libp2p_pk(pk: &PublicKey) -> LPublicKey {
    let s4 = secp256k1::PublicKey::decode(&*pk.serialize().to_vec()).unwrap();
    let pkl = LPublicKey::Secp256k1(s4);
    return pkl;
}

pub fn to_libp2p_peer_id(pk: &PublicKey) -> PeerId {
    return PeerId::from(to_libp2p_pk(&pk));
}

pub fn to_libp2p_peer_id_ser(pk: &Vec<u8>) -> PeerId {
    return PeerId::from(to_libp2p_pk(&PublicKey::from_slice(pk).unwrap()));
}
//
// #[test]
// fn test_libp2p_conversion() {
//     let tc = TestConstants::new();
//     let pk = to_libp2p_pk(&tc.public);
//     //pk.
//     // test verification here.
// }

pub trait TimeConvert {
    fn millis(&self) -> u64;
    fn nanos(&self) -> u64;
}

pub fn nanos(time: &SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos() as u64
}

pub fn millis(time: &SystemTime) -> u64 {
    nanos(time) / 1_000_000
}

impl TimeConvert for SystemTime {
    fn millis(&self) -> u64 {
        millis(self)
    }
    fn nanos(&self) -> u64 {
        nanos(self)
    }
}

pub fn vec_to_fixed(v: &Vec<u8>) -> [u8; 32] {
    let mut x: [u8; 32] = [0 as u8; 32];
    x.copy_from_slice(v);
    x
    //v.try_into().unwrap()
    //.unwrap_or_else(|v: Vec<u8>| panic!("Vec to fixed problem"))
}

#[test]
fn vec_conversions() {
    let hash = dhash_str("a");
    assert_eq!(hash, vec_to_fixed(&hash.to_vec()));
}

pub fn random_port() -> u16 {
    (3030 + OsRng::default().next_u32() % 40000) as u16
}
//
// pub fn public_response(
//     construct: fn(crate::schema::structs::PublicResponse) -> crate::schema::structs::PublicResponse,
// ) -> PublicResponse {
//     let mut public_response = PublicResponse {
//         response_metadata: None,
//         submit_transaction_response: None,
//         query_transaction_response: None,
//         about_node_response: None,
//         query_addresses_response: None,
//     };
//     construct(public_response)
// }

pub fn local_debug_mode() -> bool {
    std::env::var("REDGOLD_LOCAL_DEBUG").is_ok()
}

pub fn not_local_debug_mode() -> bool {
    !local_debug_mode()
}

pub fn make_ascii_titlecase(s: &mut str) -> String {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    return s.to_string();
}

pub const STANDARD_VERSION: i64 = 0;
