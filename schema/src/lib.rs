#![allow(unused_imports)]
#![allow(dead_code)]
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use backtrace::Backtrace;
use bitcoin::util::psbt::serialize::Deserialize;
use itertools::Itertools;
use prost::{DecodeError, Message};
use serde::Serialize;
use tokio::task::futures::TaskLocalFuture;
use tokio::task_local;

use structs::{
    Address, BytesData, Error, ErrorInfo, Hash, HashFormatType, ResponseMetadata,
    StructMetadata, Transaction,
};

use crate::structs::{AboutNodeRequest, BytesDecoder, ErrorDetails, HashType, KeyType, NetworkEnvironment, NodeMetadata, PeerData, PeerId, Proof, PublicKey, PublicRequest, PublicResponse, Request, Response, SignatureType, VersionInfo};
use crate::util::{dhash_str, dhash_vec};
use crate::util::mnemonic_words::{generate_key, generate_key_i};

pub mod structs {
    include!(concat!(env!("OUT_DIR"), "/structs.rs"));
}
pub mod address;
pub mod hash;
pub mod block;
pub mod constants;
pub mod observation;
pub mod output;
pub mod proof;
pub mod transaction;
pub mod util;
pub mod utxo_entry;
pub mod utxo_id;
pub mod merkle_proof;
pub mod errors;
pub mod transaction_builder;
pub mod response;
pub mod servers;
pub mod peers;
pub mod multiparty;
pub mod signature;
pub mod udp;
pub mod control;
pub mod public_key;
pub mod seeds;
pub mod trust;
pub mod input;
pub mod debug_version;


pub fn bytes_data(data: Vec<u8>) -> Option<BytesData> {
    Some(BytesData {
        value: data,
        decoder: BytesDecoder::Standard as i32,
        version: constants::STANDARD_VERSION,
    })
}

pub const VERSION: u32 = 0;

pub fn i64_from_string(value: String) -> Result<i64, ErrorInfo> {
    value.parse::<i64>().map_err(|_| {
        error_message(
            Error::ParseFailure,
            "unable to parse i64 value from string amount",
        )
    })
}

pub fn from_hex(hex_value: String) -> Result<Vec<u8>, ErrorInfo> {
    hex::decode(hex_value.clone()).map_err(|e| {
        error_message(
            Error::HexDecodeFailure,
            format!("Error decoding hex string value to bytes: {} {}", hex_value, e.to_string()),
        )
    })
}

//
//
// impl<T> Into<Result<T, ErrorInfo>> for Option<T> {
//     fn into(self) -> T {
//
//     }
// }

pub fn struct_metadata(time: i64) -> Option<StructMetadata> {
    Some(StructMetadata {
        time: Some(time),
        version: VERSION as i32,
        hash: None,
        signable_hash: None,
        signed_hash: None,
        counter_party_hash: None,
        confirmation_hash: None,
    })
}

pub fn struct_metadata_new() -> Option<StructMetadata> {
    struct_metadata(util::current_time_millis())
}

pub trait SafeBytesAccess {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo>;
}

impl SafeBytesAccess for Option<BytesData> {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "bytes data"))?
            .value
            .clone())
    }
}

impl SafeBytesAccess for Option<Hash> {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "hash"))?
            .bytes
            .safe_bytes()?)
    }
}

impl SafeBytesAccess for Hash {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.bytes.safe_bytes()?)
    }
}

impl SafeBytesAccess for Option<Address> {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "hash"))?
            .address
            .safe_bytes()?)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.safe_bytes()
                .map(hex::encode)
                .unwrap_or("missing hash bytes field".to_string())
        )
    }
}

pub trait HashClear {
    fn hash_clear(&mut self);
}

/*

trait AddAny
where Self: Sized + Add<Self, Output=Self>,
for<'b> Self: Add<&'b Self, Output=Self>,
{}

impl<T> AddAny for T
where T: Add<T, Output=T>,
for<'b> T: Add<&'b T, Output=T>,
{}

fn add_val_val<T: AddAny>(x: T, y: T) -> T { x + y }
fn add_val_ref<T: AddAny>(x: T, y: &T) -> T { x + y }
 */
//

pub trait ProtoSerde
    where Self: Message + Default,
{
    fn proto_serialize(&self) -> Vec<u8>;
    fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo>;
}

impl<T> ProtoSerde for T
where T: Message + Default {
    fn proto_serialize(&self) -> Vec<u8> {
        self.encode_to_vec()
    }

    fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo> {
        T::decode(&*bytes)
            .map_err(|e|
                error_message(Error::ProtoDecoderFailure, e.to_string()))
    }
}


pub trait ProtoHashable
where
    Self: HashClear + Clone + Message + Default,
{
    // fn proto_serialize(&self) -> Vec<u8>;
    // fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo>;
    fn calculate_hash(&self) -> Hash;
    fn from_hex(hex_value: String) -> Result<Self, ErrorInfo>;
}

impl<T> ProtoHashable for T
where
    T: HashClear + Clone + Message + Default,
{
    // fn proto_serialize(&self) -> Vec<u8> {
    //     self.encode_to_vec()
    // }
    //
    // fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo> {
    //     // TODO: Automap this error with a generic _.to_string() trait implicit?
    //     return T::decode(&*bytes)
    //         .map_err(|e| error_message(Error::ProtoDecoderFailure, e.to_string()));
    // }

    fn from_hex(hex_value: String) -> Result<Self, ErrorInfo> {
        Self::proto_deserialize(from_hex(hex_value)?)
    }

    fn calculate_hash(&self) -> Hash {
        let mut clone = self.clone();
        clone.hash_clear();
        let input = clone.proto_serialize();
        Hash::digest(input)
    }
}

pub trait WithMetadataHashableFields {
    fn struct_metadata_opt(&mut self) -> Option<&mut StructMetadata>;
    // fn struct_metadata(&self) -> Option<&StructMetadata>;
    fn struct_metadata_opt_ref(&self) -> Option<&StructMetadata>;
}

pub trait WithMetadataHashable {
    fn struct_metadata(&mut self) -> Result<&mut StructMetadata, ErrorInfo>;
    fn struct_metadata_err(&self) -> Result<&StructMetadata, ErrorInfo>;
    fn version(&self) -> Result<i32, ErrorInfo>;
    fn time(&self) -> Result<&i64, ErrorInfo>;
    fn hash_or(&self) -> Hash;
    fn hash_bytes(&self) -> Result<Vec<u8>, ErrorInfo>;
    fn hash_vec(&self) -> Vec<u8>;
    fn hash_hex(&self) -> Result<String, ErrorInfo>;
    fn hash_hex_or_missing(&self) -> String;
    fn with_hash(&mut self) -> &mut Self;
    fn set_hash(&mut self, hash: Hash) -> Result<(), ErrorInfo>;
}

impl<T> WithMetadataHashable for T
where
    Self: WithMetadataHashableFields + HashClear + Clone + Message + std::default::Default,
{
    fn struct_metadata(&mut self) -> Result<&mut StructMetadata, ErrorInfo> {
        let option = self.struct_metadata_opt();
        option.ok_or(error_message(Error::MissingField, "struct_metadata"))
    }

    fn struct_metadata_err(&self) -> Result<&StructMetadata, ErrorInfo> {
        self.struct_metadata_opt_ref().ok_or(error_message(Error::MissingField, "struct_metadata"))
    }

    fn version(&self) -> Result<i32, ErrorInfo> {
        Ok(self.struct_metadata_err()?.version)
    }

    fn time(&self) -> Result<&i64, ErrorInfo> {
        Ok(self.struct_metadata_opt_ref().safe_get()?.time.safe_get()?)
    }

    fn hash_or(&self) -> Hash {
        self.struct_metadata_opt_ref()
            .and_then(|s| s.hash.clone()) // TODO: Change to as_ref() to prevent clone?
            .unwrap_or(self.calculate_hash())
    }

    fn hash_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.hash_or().bytes.safe_bytes()?)
    }

    fn hash_vec(&self) -> Vec<u8> {
        self.hash_bytes().expect("hash bytes missing")
    }

    fn hash_hex(&self) -> Result<String, ErrorInfo> {
        Ok(hex::encode(self.hash_or().bytes.safe_bytes()?))
    }

    fn hash_hex_or_missing(&self) -> String {
        self.hash_hex().unwrap_or("missing hash".to_string())
    }

    fn with_hash(&mut self) -> &mut T {
        let hash = self.calculate_hash();
        self.set_hash(hash).expect("set");
        self
    }

    fn set_hash(&mut self, hash: Hash) -> Result<(), ErrorInfo> {
        let met = self.struct_metadata()?;
        met.hash = Some(hash);
        Ok(())
    }
}

/*


   pub fn hash_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {

   }
*/
//
// struct CurrencyTransferTransaction {
//     transaction: Transaction
// }

pub trait SafeOption<T> {
    fn safe_get(&self) -> Result<&T, ErrorInfo>;
    fn safe_get_msg<S: Into<String>>(&self, msg: S) -> Result<&T, ErrorInfo>;
    // TODO: put in another trait with a clone bound
    // fn safe_get_clone(&self) -> Result<T, ErrorInfo>;
}

//
// pub trait SafeOptionJson<T> {
//     fn safe_get_or_json(&self) -> Result<&T, ErrorInfo>;
// }
//
//
// impl<T> SafeOptionJson<T> for Option<T> where T: Serialize {
//     fn safe_get_or_json(&self) -> Result<&T, ErrorInfo> {
//         self.as_ref().ok_or(error_message(Error::MissingField, serde_json::to_string(&self).unwrap_or("Json serialization error of missing field data".to_string())))
//     }
// }

impl<T> SafeOption<T> for Option<T> {
    fn safe_get(&self) -> Result<&T, ErrorInfo> {
        self.as_ref().ok_or(error_message(
            Error::MissingField,
            "unspecified optional value",
        ))
    }
    fn safe_get_msg<S: Into<String>>(&self, msg: S) -> Result<&T, ErrorInfo> {
        self // TODO: parent Field message? necessary or not?
            .as_ref()
            .ok_or(error_message(
                Error::MissingField,
                format!("{} option empty", msg.into()),
            ))
    }

}

pub fn response_metadata() -> Option<ResponseMetadata> {
    let mut response_metadata = ResponseMetadata::default();
    response_metadata.success = true;
    Some(response_metadata)
}

#[test]
fn bool_defaults() {
    let metadata = ResponseMetadata::default();
    println!("metadata: {:?}", metadata);
    assert_eq!(false, metadata.success);
}


pub trait ErrorInfoContext<T, E> {
    /// Wrap the error value with additional context.
    fn error_info<C: Into<String>>(self, context: C) -> Result<T, ErrorInfo>
        where
            C: Display + Send + Sync + 'static;
    fn error_msg<C: Into<String>>(self, code: Error, context: C) -> Result<T, ErrorInfo>
        where
            C: Display + Send + Sync + 'static;

}

impl<T, E> ErrorInfoContext<T, E> for Result<T, E>
    where
        E: std::error::Error + Send + Sync + 'static,
{
    fn error_info<C: Into<String>>(self, context: C) -> Result<T, ErrorInfo>
        where
            C: Display + Send + Sync + 'static {
        // Not using map_err to save 2 useless frames off the captured backtrace
        // in ext_context.
        // self.context(context)
        self.map_err(|e| error_msg(Error::UnknownError, context.into(), e.to_string()))
    }

    fn error_msg<C: Into<String>>(self, code: Error, context: C) -> Result<T, ErrorInfo> where C: Display + Send + Sync + 'static {
        self.map_err(|e| error_msg(code, context.into(), e.to_string()))
    }
}


pub fn error_info<S: Into<String>>(message: S) -> ErrorInfo {
    error_message(crate::structs::Error::UnknownError, message.into())
}

pub fn error_code(code: Error) -> ErrorInfo {
    error_message(code, "".to_string())
}

pub fn slice_vec_eager<T>(vec: Vec<T>, start: usize, end: usize) -> Vec<T>
where T : Clone {
    let mut ret = vec![];
    // let l = vec.len();
    for (i, v) in vec.iter().enumerate() {
        if i >= start && i <= end {
            ret.push(v.clone());
        }
    }
    ret
}

pub fn split_to_str(vec: String, splitter: &str) -> Vec<String> {
    let mut ret = vec![];
    for seg in vec.split(splitter) {
        ret.push(seg.to_string());
    }
    ret
}

// TODO: This feature is only available in tokio RT, need to substitute this for a
// standard local key implementation depending on the features available for WASM crate.
task_local! {

    pub static TASK_LOCAL: HashMap<String, String>;
    // pub static TASK_LOCAL: String;
    // pub static ONE: u32;
    //
    // #[allow(unused)]
    // static TWO: f32;
    //
    // static NUMBER: u32;
}

pub fn get_task_local() -> HashMap<String, String> {
    TASK_LOCAL.try_with(|local| { local.clone() })
        .unwrap_or(HashMap::new())
}

pub fn task_local<K: Into<String>, V: Into<String>, F>(k: K, v: V, f: F) -> TaskLocalFuture<HashMap<String, String>, F>
where F : Future{
    let mut current = get_task_local();
    current.insert(k.into(), v.into());
    TASK_LOCAL.scope(current, f)
}

pub fn task_local_map<F>(kv: HashMap<String, String>, f: F) -> TaskLocalFuture<HashMap<String, String>, F>
where F : Future{
    let mut current = get_task_local();
    for (k, v) in kv {
        current.insert(k, v);
    }
    TASK_LOCAL.scope(current, f)
}


pub fn error_msg<S: Into<String>, P: Into<String>>(code: Error, message: S, lib_message: P) -> ErrorInfo {
    let stacktrace = format!("{:?}", Backtrace::new());
    let stacktrace_abridged: Vec<String> = split_to_str(stacktrace, "\n");
    // 14 is number of lines of prelude, might need to be less here honestly due to invocation.
    let stack = slice_vec_eager(stacktrace_abridged, 0, 50).join("\n").to_string();

    let details = get_task_local().iter().map(|(k, v)| {
        ErrorDetails {
            detail_name: k.clone(),
            detail: v.clone(),
        }
    }).collect_vec();

    ErrorInfo {
        code: code as i32,
        // TODO: From error code map
        description: "".to_string(),
        description_extended: "".to_string(),
        message: message.into(),
        details,
        retriable: false,
        stacktrace: stack,
        lib_message: lib_message.into(),
    }
}

pub fn error_message<S: Into<String>>(error_code: structs::Error, message: S) -> ErrorInfo {
    error_msg(error_code, message, "".to_string())
}

pub fn empty_public_response() -> PublicResponse {
    PublicResponse {
        response_metadata: None,
        submit_transaction_response: None,
        query_transaction_response: None,
        about_node_response: None,
        query_addresses_response: None,
        faucet_response: None,
        recent_transactions_response: None,
        hash_search_response: None
    }
}


pub fn empty_public_request() -> PublicRequest {
    PublicRequest {
        submit_transaction_request: None,
        query_transaction_request: None,
        about_node_request: None,
        query_addresses_request: None,
        faucet_request: None,
        recent_transactions_request: None,
        hash_search_request: None
    }
}


pub struct TestConstants {
    pub secret: bitcoin::secp256k1::SecretKey,
    pub public: bitcoin::secp256k1::PublicKey,
    pub public_peer_id: Vec<u8>,
    pub secret2: bitcoin::secp256k1::SecretKey,
    pub public2: bitcoin::secp256k1::PublicKey,
    pub hash: [u8; 32],
    pub hash_vec: Vec<u8>,
    pub addr: Vec<u8>,
    pub addr2: Vec<u8>,
    pub peer_ids: Vec<Vec<u8>>,
    pub peer_trusts: Vec<f64>,
    pub address_1: Address,
    pub rhash_1: Hash,
    pub rhash_2: Hash,
}
impl TestConstants {
    pub fn key_pair(&self) -> KeyPair {
        KeyPair {
            secret_key: self.secret,
            public_key: self.public,
        }
    }
    pub fn new() -> TestConstants {
        let (secret, public) = crate::util::mnemonic_words::generate_key();
        let (secret2, public2) = generate_key_i(1);
        let hash = crate::util::dhash_str("asdf");
        let hash_vec = hash.to_vec();
        let addr = Address::address(&public);
        let addr2 = Address::address(&public2);
        let mut peer_ids: Vec<Vec<u8>> = Vec::new();
        let mut peer_trusts: Vec<f64> = Vec::new();

        for i in 0..10 {
            peer_ids.push(dhash_str(&i.to_string()).to_vec());
            peer_trusts.push((i as f64) / 10f64);
        }

        let public_peer_id = dhash_vec(&dhash_vec(&public.serialize().to_vec()).to_vec()).to_vec();

        return TestConstants {
            secret,
            public,
            public_peer_id,
            secret2,
            public2,
            hash,
            hash_vec,
            addr: addr.clone(),
            addr2,
            peer_ids,
            peer_trusts,
            address_1: addr.into(),
            rhash_1: Hash::from_string_calculate("asdf"),
            rhash_2: Hash::from_string_calculate("asdf2"),
        };
    }
}

pub fn signature_data(data: Vec<u8>) -> Option<crate::structs::Signature> {
    Some(structs::Signature {
        bytes: bytes_data(data),
        signature_type: SignatureType::Ecdsa as i32,
    })
}

pub fn decode_hex(h: String) -> Result<Vec<u8>, ErrorInfo> {
    from_hex(h)
}

#[derive(Clone, Copy)]
pub struct KeyPair {
    pub secret_key: bitcoin::secp256k1::SecretKey,
    pub public_key: bitcoin::secp256k1::PublicKey,
}

impl KeyPair {
    pub fn new(
        secret_key: &bitcoin::secp256k1::SecretKey,
        public_key: &bitcoin::secp256k1::PublicKey,
    ) -> Self {
        return Self {
            secret_key: *secret_key,
            public_key: *public_key,
        };
    }

    pub fn address(&self) -> Vec<u8> {
        Address::address(&self.public_key)
    }

    pub fn address_typed(&self) -> Address {
        Address::from_public(&self.public_key).expect("address")
    }

    pub fn public_key_vec(&self) -> Vec<u8> {
        self.public_key.serialize().to_vec()
    }
}


impl PeerData {
    pub fn proto_serialize(&self) -> Vec<u8> {
        return self.encode_to_vec();
    }

    pub fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        return PeerData::decode(&*bytes);
    }

}

impl HashClear for Request {
    fn hash_clear(&mut self) {
        self.proof = None;
    }
}

impl Request {
    pub fn serialize(&self) -> Vec<u8> {
        return self.encode_to_vec();
    }

    pub fn deserialize(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        return Request::decode(&*bytes);
    }

    pub fn empty() -> Self {
        Request::default()
    }

    pub fn about(&mut self) -> Self {
        self.about_node_request = Some(AboutNodeRequest{verbose: true});
        self.clone()
    }

    pub fn with_auth(&mut self, key_pair: &KeyPair) -> &mut Request {
        let hash = self.calculate_hash();
        // println!("with_auth hash: {:?}", hash.hex());
        let proof = Proof::from_keypair_hash(&hash, &key_pair);
        proof.verify(&hash).expect("immediate verify");
        self.proof = Some(proof);
        self
    }

    pub fn with_metadata(&mut self, node_metadata: NodeMetadata) -> &mut Request {
        self.node_metadata = Some(node_metadata);
        self
    }

    pub fn verify_auth(&self) -> Result<PublicKey, ErrorInfo> {
        let hash = self.calculate_hash();
        // println!("verify_auth hash: {:?}", hash.hex());
        self.proof.safe_get()?.verify(&hash)?;
        let proof = self.proof.safe_get()?;
        let pk = proof.public_key.safe_get()?;
        Ok(pk.clone())
    }

}

#[test]
fn verify_request_auth() {
    let tc = TestConstants::new();
    let mut req = Request::empty();
    req.about();
    req.with_auth(&tc.key_pair());
    // println!("after with auth assign proof {}", req.calculate_hash().hex());
    req.verify_auth().unwrap();

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let result = add(2, 2);
        // assert_eq!(result, 4);
    }
}

impl NetworkEnvironment {
    pub fn to_std_string(&self) -> String {
        format!("{:?}", &self).to_lowercase()
    }
    pub fn parse(from_str: String) -> Self {
        let mut n = from_str.clone();
        let string2 = util::make_ascii_titlecase(&mut *n);
        NetworkEnvironment::from_str(&*string2).expect("error parsing network environment")
    }
    pub fn parse_safe(from_str: String) -> Result<Self, ErrorInfo> {
        let mut n = from_str.clone();
        let string2 = util::make_ascii_titlecase(&mut *n);
        NetworkEnvironment::from_str(&*string2).error_info("error parsing network environment")
    }

    pub fn status_networks() -> Vec<NetworkEnvironment> {
        vec![
            NetworkEnvironment::Main,
            NetworkEnvironment::Test,
            NetworkEnvironment::Dev,
            NetworkEnvironment::Predev,
        ]
    }

    pub fn default_port_offset(&self) -> u16 {
        let port = match self {
            NetworkEnvironment::Main => {16180}
            NetworkEnvironment::Test => {16280}
            NetworkEnvironment::Staging => {16380}
            NetworkEnvironment::Dev => {16480}
            NetworkEnvironment::Predev => {16580}
            NetworkEnvironment::Perf => {16680}
            NetworkEnvironment::Integration => {16780}
            NetworkEnvironment::Local => {16880}
            NetworkEnvironment::Debug => {16980}
            NetworkEnvironment::All => {17080}
        };
        port as u16
    }
}

#[test]
fn network_environment_ser() {
    use std::str::FromStr;
    use strum_macros::EnumString;

    println!("{}", format!("{:?}", NetworkEnvironment::Local).to_lowercase());
    assert_eq!(NetworkEnvironment::Local.to_std_string(), "local");
}

impl PublicResponse {
    pub fn accepted(&self) -> bool {
        self.response_metadata
            .as_ref()
            .map(|x| x.success)
            .unwrap_or(false)
    }
    pub fn error_code(&self) -> Option<i32> {
        self.response_metadata
            .clone()
            .and_then(|r| r.error_info.map(|e| e.code))
    }
    pub fn error_info(&self) -> Option<ErrorInfo> {
        self.response_metadata
            .clone()
            .and_then(|r| r.error_info)
    }
    pub fn as_error(&self) -> Result<Self, ErrorInfo> {
        self.error_info().map(|o| Err(o)).unwrap_or(Ok(self.clone()))
    }
}

pub trait EasyJson {
    fn json(&self) -> Result<String, ErrorInfo>;
    fn json_or(&self) -> String;
    fn json_pretty(&self) -> Result<String, ErrorInfo>;
    fn json_pretty_or(&self) -> String;
}

pub trait EasyJsonDeser {
    fn json_from<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, ErrorInfo>;
}

impl EasyJsonDeser for String {
    fn json_from<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, ErrorInfo> {
        json_from(self)
    }
}

impl<T> EasyJson for T
where T: Serialize {
    fn json(&self) -> Result<String, ErrorInfo> {
        json(&self)
    }

    fn json_or(&self) -> String {
        json_or(&self)
    }

    fn json_pretty(&self) -> Result<String, ErrorInfo> {
        json_pretty(&self)
    }
    fn json_pretty_or(&self) -> String {
        json_pretty(&self).unwrap_or("json pretty failure".to_string())
    }
}

#[test]
pub fn json_trait_ser_test() {
    let mut vers = VersionInfo::default();
    vers.executable_checksum = "asdf".to_string();
    println!("{}", vers.json_or());
}

pub fn json<T: Serialize>(t: &T) -> Result<String, ErrorInfo> {
    serde_json::to_string(&t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {:?}", e)))
}

pub fn json_result<T: Serialize, E: Serialize>(t: &Result<T, E>) -> String {
    match t {
        Ok(t) => json_or(t),
        Err(e) => json_or(e),
    }
}

pub fn json_or<T: Serialize>(t: &T) -> String {
    json(t).unwrap_or("json ser failure of error".to_string())
}

pub fn json_pretty<T: Serialize>(t: &T) -> Result<String, ErrorInfo> {
    serde_json::to_string_pretty(&t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {:?}", e)))
}

pub fn json_from<'a, T: serde::Deserialize<'a>>(t: &'a str) -> Result<T, ErrorInfo> {
    serde_json::from_str(t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {:?}", e)))
}

impl PeerId {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            peer_id: bytes_data(bytes),
            known_proof: vec![],
        }
    }
}

impl HashClear for StructMetadata {
    fn hash_clear(&mut self) {
        self.hash = None;
        self.signable_hash = None;
        self.signed_hash = None;
        self.counter_party_hash = None;
        self.confirmation_hash = None;
    }
}

impl StructMetadata {

}

pub trait ShortString {
    fn short_string(&self) -> Result<String, ErrorInfo>;
}

impl ShortString for String {
    fn short_string(&self) -> Result<String, ErrorInfo> {
        let len = self.len();
        let start = (len as i32) - 6;
        if start < 0 {
            return Err(error_info("string too short to short_string"));
        }
        let start = start as usize;
        let x = &self[start..len];
        Ok(x.to_string())
    }
}

pub type RgResult<T> = Result<T, ErrorInfo>;