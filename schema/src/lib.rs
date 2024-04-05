#![allow(unused_imports)]
#![allow(dead_code)]
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::str::FromStr;

use anyhow::{Context, Result};
use backtrace::Backtrace;
use itertools::Itertools;
use prost::{DecodeError, Message};
use serde::Serialize;
use tokio::task::futures::TaskLocalFuture;
use tokio::task_local;

use structs::{
    Address, BytesData, Error, ErrorInfo, Hash, HashFormatType, ResponseMetadata,
    StructMetadata, Transaction,
};
use observability::errors::EnhanceErrorInfo;

use crate::structs::{AboutNodeRequest, BytesDecoder, ContentionKey, ErrorDetails, NetworkEnvironment, NodeMetadata, PeerId, PeerMetadata, PublicKey, PublicRequest, PublicResponse, Request, Response, SignatureType, StateSelector, VersionInfo};

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
pub mod transaction_info;
pub mod exec;
pub mod contract;
pub mod local_stored_state;
mod weighting;
pub mod pow;
pub mod tx_schema_validate;
pub mod fee_validator;
pub mod observability;


impl BytesData {
    pub fn from(data: Vec<u8>) -> Self {
        BytesData {
            value: data,
            decoder: BytesDecoder::Standard as i32,
            version: constants::STANDARD_VERSION,
        }
    }
}

pub fn bytes_data(data: Vec<u8>) -> Option<BytesData> {
    Some(BytesData::from(data))
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

pub fn from_hex_ref(hex_value: &String) -> Result<Vec<u8>, ErrorInfo> {
    hex::decode(hex_value).map_err(|e| {
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
//
// impl SafeBytesAccess for Option<Hash> {
//     fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
//         Ok(self
//             .as_ref() // TODO: parent Field message? necessary or not?
//             .ok_or(error_message(Error::MissingField, "hash"))?
//             .bytes
//             .safe_bytes()?)
//     }
// }

impl SafeBytesAccess for Hash {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.bytes.safe_bytes()?)
    }
}

impl SafeBytesAccess for Option<Address> {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "address"))?
            .address
            .safe_bytes()?)
    }
}

impl SafeBytesAccess for Option<PublicKey> {
    fn safe_bytes(&self) -> RgResult<Vec<u8>> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "Missing public key"))?
            .bytes
            .safe_bytes()?
        )
    }
}

impl SafeBytesAccess for Option<PeerId> {
    fn safe_bytes(&self) -> RgResult<Vec<u8>> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "Missing peerid"))?
            .peer_id
            .safe_bytes()?
        )
    }
}

impl<T> SafeBytesAccess for Option<T>
where T: SafeBytesAccess + Sized
{
    fn safe_bytes(&self) -> RgResult<Vec<u8>> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "Missing safe bytes field"))?
            .safe_bytes()?
        )
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

    fn proto_deserialize_hex(s: impl Into<String>) -> Result<Self, ErrorInfo>;
    fn proto_deserialize_ref(bytes: &Vec<u8>) -> Result<Self, ErrorInfo>;
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

    fn proto_deserialize_hex(s: impl Into<String>) -> Result<Self, ErrorInfo> {
        hex::decode(s.into()).error_info("hex decode").and_then(|v| T::proto_deserialize(v))
    }

    fn proto_deserialize_ref(bytes: &Vec<u8>) -> Result<Self, ErrorInfo> {
        T::decode(&**bytes)
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
    fn div_mod(&self, bucket: usize) -> i64;
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

    fn div_mod(&self, bucket: usize) -> i64 {
        self.calculate_hash().div_mod(bucket)
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
    fn ok_msg(self, err: impl Into<String>) -> Result<T, ErrorInfo>;
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


// #[inline]
// #[stable(feature = "rust1", since = "1.0.0")]
// pub fn ok_or<E>(self, err: E) -> Result<T, E> {
//     match self {
//         Some(v) => Ok(v),
//         None => Err(err),
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
    fn ok_msg(self, err: impl Into<String>) -> RgResult<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(error_info(err.into())),
        }
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
        abort: false,
        skip_logging: false,
        internal_log_level: None
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

pub fn signature_data(data: Vec<u8>) -> Option<crate::structs::Signature> {
    Some(structs::Signature {
        bytes: bytes_data(data),
        signature_type: SignatureType::Ecdsa as i32,
        rsv: None
    })
}

pub fn decode_hex(h: String) -> Result<Vec<u8>, ErrorInfo> {
    from_hex(h)
}


impl PeerMetadata {
    pub fn proto_serialize(&self) -> Vec<u8> {
        return self.encode_to_vec();
    }

    pub fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        return PeerMetadata::decode(&*bytes);
    }

}

impl HashClear for Request {
    fn hash_clear(&mut self) {

        self.proof = None;
        self.origin = None;

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

    pub fn with_metadata(mut self, node_metadata: NodeMetadata) -> Request {
        self.node_metadata = Some(node_metadata);
        self
    }

    pub fn auth_required(&self) -> bool {
        self.initiate_keygen.is_some() || self.initiate_signing.is_some()
    }

}



#[cfg(test)]
mod tests {
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
            NetworkEnvironment::Staging,
            NetworkEnvironment::Dev,
            NetworkEnvironment::Predev,
        ]
    }

    pub fn gui_networks() -> Vec<NetworkEnvironment> {
        vec![
            NetworkEnvironment::Main,
            NetworkEnvironment::Test,
            NetworkEnvironment::Staging,
            NetworkEnvironment::Dev,
            NetworkEnvironment::Predev,
        ]
    }

    pub fn is_local_debug(&self) -> bool {
        vec![
            NetworkEnvironment::Debug, NetworkEnvironment::Local
        ].contains(self)
    }

    pub fn is_main_stage_network(&self) -> bool {
        Self::status_networks().contains(self)
    }

    pub fn is_all(&self) -> bool {
        self == &NetworkEnvironment::All
    }

    pub fn is_main(&self) -> bool {
        self == &NetworkEnvironment::Main
    }
    pub fn is_dev(&self) -> bool {
        self == &NetworkEnvironment::Dev
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

// #[async_trait]
pub trait EasyJson {
    fn json(&self) -> Result<String, ErrorInfo>;
    fn json_or(&self) -> String;
    fn json_pretty(&self) -> Result<String, ErrorInfo>;
    fn json_pretty_or(&self) -> String;
    fn write_json(&self, path: &str) -> RgResult<()>;
}

pub trait EasyJsonDeser {
    fn json_from<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, ErrorInfo>;
}

impl EasyJsonDeser for String {
    fn json_from<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, ErrorInfo> {
        json_from(self)
    }
}

// #[async_trait]
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

    fn write_json(&self, path: &str) -> RgResult<()> {
        let string = self.json_or();
        std::fs::write(path, string.clone()).error_info("error write json to path ").add(path.to_string()).add(" ").add(string)
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
            peer_id: Some(PublicKey::from_bytes(bytes)),
            known_proof: vec![],
        }
    }

    pub fn from_hex(hex: impl Into<String>) -> RgResult<Self> {
        Ok(Self::from_bytes(from_hex(hex.into())?))
    }

    pub fn from_pk(pk: PublicKey) -> Self {
        Self {
            peer_id: Some(pk),
            known_proof: vec![],
        }
    }

    pub fn hex_or(&self) -> String {
        self.peer_id.as_ref().map(|x| x.hex_or()).unwrap_or("missing peer id".to_string())
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


impl HashClear for BytesData {
    fn hash_clear(&mut self) {}
}

impl HashClear for structs::ContentionKey {
    fn hash_clear(&mut self) {}
}

impl ContentionKey {
    pub fn contract_request(address: &Address, selector: Option<&StateSelector>) -> ContentionKey {
        let mut s = Self::default();
        s.address = Some(address.clone());
        s.selector = selector.cloned();
        s
    }
}
