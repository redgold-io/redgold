#![allow(unused_imports)]
#![allow(dead_code)]
use std::fmt::Display;
use std::future::Future;
use std::str::FromStr;

use anyhow::{Context, Result};
use backtrace::Backtrace;
use itertools::Itertools;
use prost::{DecodeError, Message};
use serde::Serialize;

use structs::{
    Address, BytesData, ErrorCode, ErrorInfo, Hash, HashFormatType, ResponseMetadata,
    StructMetadata, Transaction,
};
use observability::errors::EnhanceErrorInfo;
use proto_serde::{ProtoHashable, ProtoSerde};
use util::{lang_util, times};
use crate::structs::{AboutNodeRequest, BytesDecoder, ContentionKey, NetworkEnvironment, NodeMetadata, PeerId, PeerMetadata, PublicKey, PublicRequest, PublicResponse, Request, Response, SignatureType, StateSelector};
use crate::util::task_local::task_local_impl::task_local_error_details;

pub mod structs {
    include!(concat!(env!("OUT_DIR"), "/structs.rs"));
}

pub mod execution {
    include!(concat!(env!("OUT_DIR"), "/execution.rs"));
}

pub mod airgap {
    include!(concat!(env!("OUT_DIR"), "/airgap.rs"));
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
pub mod weighting;
pub mod pow;
pub mod tx_schema_validate;
pub mod fee_validator;
pub mod observability;
pub mod proto_serde;
pub mod helpers;
pub mod party;
pub mod tx;
pub mod config_data;
pub mod portfolio;
pub mod conf;
pub mod data_folder;
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
            ErrorCode::ParseFailure,
            "unable to parse i64 value from string amount",
        )
    })
}

pub fn from_hex(hex_value: String) -> Result<Vec<u8>, ErrorInfo> {
    hex::decode(hex_value.clone()).map_err(|e| {
        error_message(
            ErrorCode::HexDecodeFailure,
            format!("Error decoding hex string value to bytes: {} {}", hex_value, e.to_string()),
        )
    })
}

pub fn from_hex_ref(hex_value: &String) -> Result<Vec<u8>, ErrorInfo> {
    hex::decode(hex_value).map_err(|e| {
        error_message(
            ErrorCode::HexDecodeFailure,
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
    struct_metadata(times::current_time_millis())
}
//
// pub trait SafeBytesAccess {
//     fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo>;
// }
//
// impl SafeBytesAccess for Option<BytesData> {
//     fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
//         Ok(self
//             .as_ref() // TODO: parent Field message? necessary or not?
//             .ok_or(error_message(ErrorCode::MissingField, "bytes data"))?
//             .value
//             .clone())
//     }
// }
//
//
// //
// // impl SafeBytesAccess for Option<Hash> {
// //     fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
// //         Ok(self
// //             .as_ref() // TODO: parent Field message? necessary or not?
// //             .ok_or(error_message(Error::MissingField, "hash"))?
// //             .bytes
// //             .safe_bytes()?)
// //     }
// // }
// //
// // impl SafeBytesAccess for Hash {
// //     fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
// //         self.proto_serialize()
// //     }
// // }
//
// impl SafeBytesAccess for Option<Address> {
//     fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
//         Ok(self
//             .as_ref() // TODO: parent Field message? necessary or not?
//             .ok_or(error_message(ErrorCode::MissingField, "address"))?
//             .address
//             .safe_bytes()?)
//     }
// }
//
// impl SafeBytesAccess for Option<PublicKey> {
//     fn safe_bytes(&self) -> RgResult<Vec<u8>> {
//         Ok(self
//             .as_ref() // TODO: parent Field message? necessary or not?
//             .ok_or(error_message(ErrorCode::MissingField, "Missing public key"))?
//             .bytes
//             .safe_bytes()?
//         )
//     }
// }
//
// impl SafeBytesAccess for Option<PeerId> {
//     fn safe_bytes(&self) -> RgResult<Vec<u8>> {
//         Ok(self
//             .as_ref() // TODO: parent Field message? necessary or not?
//             .ok_or(error_message(ErrorCode::MissingField, "Missing peerid"))?
//             .peer_id
//             .safe_bytes()?
//         )
//     }
// }
//
// impl<T> SafeBytesAccess for Option<T>
// where T: SafeBytesAccess + Sized
// {
//     fn safe_bytes(&self) -> RgResult<Vec<u8>> {
//         Ok(self
//             .as_ref() // TODO: parent Field message? necessary or not?
//             .ok_or(error_message(ErrorCode::MissingField, "Missing safe bytes field"))?
//             .safe_bytes()?
//         )
//     }
// }

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.hex()
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
            ErrorCode::MissingField,
            "unspecified optional value",
        ))
    }
    fn safe_get_msg<S: Into<String>>(&self, msg: S) -> Result<&T, ErrorInfo> {
        self // TODO: parent Field message? necessary or not?
            .as_ref()
            .ok_or(error_message(
                ErrorCode::MissingField,
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
    fn error_msg<C: Into<String>>(self, code: ErrorCode, context: C) -> Result<T, ErrorInfo>
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
        self.map_err(|e| error_msg(ErrorCode::UnknownError, context.into(), e.to_string()))
    }

    fn error_msg<C: Into<String>>(self, code: ErrorCode, context: C) -> Result<T, ErrorInfo> where C: Display + Send + Sync + 'static {
        self.map_err(|e| error_msg(code, context.into(), e.to_string()))
    }
}


pub fn error_info<S: Into<String>>(message: S) -> ErrorInfo {
    error_message(crate::structs::ErrorCode::UnknownError, message.into())
}

pub fn error_code(code: ErrorCode) -> ErrorInfo {
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

pub fn error_msg<S: Into<String>, P: Into<String>>(code: ErrorCode, message: S, lib_message: P) -> ErrorInfo {
    let stacktrace = format!("{:?}", Backtrace::new());
    let stacktrace_abridged: Vec<String> = split_to_str(stacktrace, "\n");
    // 14 is number of lines of prelude, might need to be less here honestly due to invocation.
    let stack = slice_vec_eager(stacktrace_abridged, 0, 50).join("\n").to_string();

    let details = task_local_error_details();

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

pub fn error_message<S: Into<String>>(error_code: structs::ErrorCode, message: S) -> ErrorInfo {
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

    pub fn btc_explorer_link(&self) -> String {
        let mut net = "testnet/";
        if self.is_main() {
            net = "";
        }
        format!("https://blockstream.info/{net}")
    }

    pub fn btc_address_link(&self, address: String) -> String {
        format!("{}address/{}", self.btc_explorer_link(), address)
    }


    pub fn btc_tx_link(&self, address: String) -> String {
        format!("{}tx/{}", self.btc_explorer_link(), address)
    }

    pub fn eth_explorer_link(&self) -> String {
        let eth_url = if self.is_main() {
            "https://etherscan.io"
        } else {
            "https://sepolia.etherscan.io"
        };
        eth_url.to_string()
    }

    pub fn eth_address_link(&self, eth_address: String) -> String {
        format!("{}/address/{}", self.eth_explorer_link(), eth_address)
    }

    pub fn eth_tx_link(&self, txid: String) -> String {
        format!("{}/tx/{}", self.eth_explorer_link(), txid)
    }


    pub fn explorer_link(&self) -> String {
        let self_str = self.to_std_string();
        let pfx = if self.is_main() {
            "".to_string()
        } else {
            format!("{}.", self_str)
        };
      format!("https://{}explorer.redgold.io", pfx)
    }

    pub fn explorer_hash_link(&self, hash: String) -> String {
        format!("{}/hash/{}", self.explorer_link(), hash)
    }

    pub fn to_std_string(&self) -> String {
        format!("{:?}", &self).to_lowercase()
    }
    pub fn parse(from_str: String) -> Self {
        let mut n = from_str.clone();
        let string2 = lang_util::make_ascii_titlecase(&mut *n);
        NetworkEnvironment::from_str(&*string2).expect("error parsing network environment")
    }
    pub fn parse_safe(from_str: String) -> Result<Self, ErrorInfo> {
        let mut n = from_str.clone();
        let string2 = lang_util::make_ascii_titlecase(&mut *n);
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

impl PeerId {
    pub fn from_bytes_direct(bytes: Vec<u8>) -> Self {
        Self {
            peer_id: Some(PublicKey::from_bytes_direct_ecdsa(bytes)),
        }
    }

    pub fn from_pk(pk: PublicKey) -> Self {
        Self {
            peer_id: Some(pk),
        }
    }

    pub fn raw_hex_or_from_public_key(&self) -> String {
        self.peer_id.as_ref().map(|x| x.hex()).unwrap_or("missing peer id".to_string())
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
    fn last_n(&self, n: impl Into<i32>) -> Result<String, ErrorInfo>;
}

impl ShortString for String {
    fn short_string(&self) -> Result<String, ErrorInfo> {
        self.last_n(6)
    }

    fn last_n(&self, n: impl Into<i32>) -> Result<String, ErrorInfo> {
        let len = self.len();
        let start = (len as i32) - n.into();
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
