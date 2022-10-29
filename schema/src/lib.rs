use backtrace::Backtrace;
use itertools::Itertools;
use multihash::{Multihash, MultihashDigest};
use prost::{DecodeError, Message};

use structs::{
    Address, AddressType, BytesData, Error, ErrorInfo, Hash, HashFormatType, ResponseMetadata,
    StructMetadata, Transaction,
};

use crate::structs::{AboutNodeRequest, BytesDecoder, ErrorDetails, NetworkEnvironment, NodeMetadata, PeerData, Proof, PublicRequest, PublicResponse, Request, Response, SignatureType};
use crate::util::{dhash_str, dhash_vec};
use crate::util::wallet::{generate_key, generate_key_i};

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


use std::str::FromStr;
use serde::Serialize;

pub fn bytes_data(data: Vec<u8>) -> Option<BytesData> {
    Some(BytesData {
        bytes_value: data,
        bytes_decoder: BytesDecoder::Standard as i32,
        version: constants::STANDARD_VERSION,
    })
}

pub const VERSION: u32 = 0;

pub fn i64_from_string(value: String) -> Result<i64, ErrorInfo> {
    value.parse::<i64>().map_err(|e| {
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

impl Into<Hash> for Multihash {
    fn into(self) -> Hash {
        Hash {
            bytes: bytes_data(self.to_bytes()),
            hash_format_type: HashFormatType::Multihash as i32,
        }
    }
}

impl Into<Hash> for Vec<u8> {
    fn into(self) -> Hash {
        Hash {
            bytes: bytes_data(self),
            hash_format_type: HashFormatType::Legacy as i32,
        }
    }
}

impl Into<Address> for Vec<u8> {
    fn into(self) -> Address {
        address::address_data(self).expect("some")
    }
}

pub fn struct_metadata(time: i64) -> Option<StructMetadata> {
    Some(StructMetadata {
        time,
        version: VERSION as i32,
    })
}

pub fn struct_metadata_new() -> Option<StructMetadata> {
    Some(StructMetadata {
        time: util::current_time_millis(),
        version: constants::VERSION as i32,
    })
}

pub trait SafeBytesAccess {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo>;
}

impl SafeBytesAccess for Option<BytesData> {
    fn safe_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self
            .as_ref() // TODO: parent Field message? necessary or not?
            .ok_or(error_message(Error::MissingField, "bytes data"))?
            .bytes_value
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
pub trait ProtoHashable
where
    Self: HashClear + Clone + Message + Default,
{
    fn proto_serialize(&self) -> Vec<u8>;
    fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo>;
    fn calculate_hash(&self) -> Hash;
    fn from_hex(hex_value: String) -> Result<Self, ErrorInfo>;
}

impl<T> ProtoHashable for T
where
    T: HashClear + Clone + Message + Default,
{
    fn proto_serialize(&self) -> Vec<u8> {
        self.encode_to_vec()
    }

    fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo> {
        // TODO: Automap this error with a generic _.to_string() trait implicit?
        return T::decode(&*bytes)
            .map_err(|e| error_message(Error::ProtoDecoderFailure, e.to_string()));
    }

    fn from_hex(hex_value: String) -> Result<Self, ErrorInfo> {
        Self::proto_deserialize(from_hex(hex_value)?)
    }

    fn calculate_hash(&self) -> Hash {
        let mut clone = self.clone();
        clone.hash_clear();
        let input = self.proto_serialize();
        let multihash = constants::HASHER.digest(&input);
        return multihash.into();
    }
}

pub trait WithMetadataHashableFields {
    fn set_hash(&mut self, hash: Hash);
    fn stored_hash_opt(&self) -> Option<Hash>;
    fn struct_metadata_opt(&self) -> Option<StructMetadata>;
}

pub trait WithMetadataHashable {
    fn struct_metadata(&self) -> Result<StructMetadata, ErrorInfo>;
    fn version(&self) -> Result<i32, ErrorInfo>;
    fn time(&self) -> Result<i64, ErrorInfo>;
    fn hash(&self) -> Hash;
    fn hash_bytes(&self) -> Result<Vec<u8>, ErrorInfo>;
    fn hash_vec(&self) -> Vec<u8>;
    fn hash_hex(&self) -> Result<String, ErrorInfo>;
    fn hash_hex_or_missing(&self) -> String;
    fn with_hash(&mut self) -> &mut Self;
}

impl<T> WithMetadataHashable for T
where
    Self: WithMetadataHashableFields + HashClear + Clone + Message + std::default::Default,
{
    fn struct_metadata(&self) -> Result<StructMetadata, ErrorInfo> {
        Ok(self
            .struct_metadata_opt()
            .ok_or(error_message(Error::MissingField, "struct_metadata"))?
            .clone())
    }

    fn version(&self) -> Result<i32, ErrorInfo> {
        Ok(self.struct_metadata()?.version)
    }

    fn time(&self) -> Result<i64, ErrorInfo> {
        Ok(self.struct_metadata()?.time)
    }

    fn hash(&self) -> Hash {
        self.stored_hash_opt().unwrap_or(self.calculate_hash())
    }

    fn hash_bytes(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.hash().bytes.safe_bytes()?)
    }

    fn hash_vec(&self) -> Vec<u8> {
        self.hash_bytes().expect("hash bytes missing")
    }

    fn hash_hex(&self) -> Result<String, ErrorInfo> {
        Ok(hex::encode(self.hash().bytes.safe_bytes()?))
    }

    fn hash_hex_or_missing(&self) -> String {
        self.hash_hex().unwrap_or("missing hash".to_string())
    }

    fn with_hash(&mut self) -> &mut T {
        let hash = self.calculate_hash();
        self.set_hash(hash);
        self
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
}

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
    Some(ResponseMetadata {
        success: true,
        error_info: None,
    })
}

pub fn error_info<S: Into<String>>(message: S) -> ErrorInfo {
    ErrorInfo {
        code: crate::structs::Error::UnknownError as i32,
        description: message.into(),
        description_extended: "".to_string(),
        message: "".to_string(),
        details: vec![],
        retriable: false,
    }
}

pub fn error_from_code(error_code: structs::Error) -> ErrorInfo {
    ErrorInfo {
        code: error_code as i32,
        // TODO: From error code map
        description: "".to_string(),
        description_extended: "".to_string(),
        message: "".to_string(),
        details: vec![],
        retriable: false,
    }
}

pub fn slice_vec_eager<T>(vec: Vec<T>, start: usize, end: usize) -> Vec<T>
where T : Clone {
    let mut ret = vec![];
    let l = vec.len();
    for (i, v) in vec.iter().enumerate() {
        if i >= start && i <= end {
            ret.push(v.clone());
        }
    }
    ret
}

pub fn split_to_str(vec: String, splitter: &str) -> Vec<String> {
    let mut ret = vec![];
    for seg in vec.split("\n") {
        ret.push(seg.to_string());
    }
    ret
}

pub fn error_message<S: Into<String>>(error_code: structs::Error, message: S) -> ErrorInfo {
    let stacktrace = format!("{:?}", Backtrace::new());
    let stacktrace_abridged: Vec<String> = split_to_str(stacktrace, "\n");
    let stack = slice_vec_eager(stacktrace_abridged, 0, 30).join("\n").to_string();
    ErrorInfo {
        code: error_code as i32,
        // TODO: From error code map
        description: "".to_string(),
        description_extended: "".to_string(),
        message: message.into(),
        details: vec![ErrorDetails{ detail_name: "stacktrace".into(), detail: stack }],
        retriable: false,
    }
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
        let (secret, public) = crate::util::wallet::generate_key();
        let (secret2, public2) = generate_key_i(1);
        let hash = crate::util::dhash_str("asdf");
        let hash_vec = hash.to_vec();
        let addr = crate::address::address(&public);
        let addr2 = crate::address::address(&public2);
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
            rhash_1: Hash::from_string("asdf"),
            rhash_2: Hash::from_string("asdf2"),
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
        crate::address::address(&self.public_key)
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
        Self{
            gossip_transaction_request: None,
            gossip_observation_request: None,
            resolve_hash_request: None,
            download_request: None,
            about_node_request: None,
            proof: None,
            node_metadata: None,
            get_peers_info_request: None
        }
    }

    pub fn about(&mut self) -> Self {
        self.about_node_request = Some(AboutNodeRequest{verbose: true});
        self.clone()
    }

    pub fn with_auth(&mut self, key_pair: &KeyPair) -> &mut Request {
        let hash = self.calculate_hash();
        let proof = Proof::from_keypair_hash(&hash, &key_pair);
        self.proof = Some(proof);
        self
    }

    pub fn with_metadata(&mut self, node_metadata: NodeMetadata) -> &mut Request {
        self.node_metadata = Some(node_metadata);
        self
    }

}

impl HashClear for Response {
    fn hash_clear(&mut self) {
        self.proof = None;
    }
}

impl Response {
    pub fn serialize(&self) -> Vec<u8> {
        return self.encode_to_vec();
    }

    pub fn deserialize(bytes: Vec<u8>) -> Result<Self, DecodeError> {
        return Response::decode(&*bytes);
    }

    pub fn empty_success() -> Response {
        Response {
            response_metadata: response_metadata(),
            resolve_hash_response: None,
            download_response: None,
            about_node_response: None,
            get_peers_info_response: None,
            node_metadata: None,
            proof: None
        }
    }
    pub fn from_error_info(error_info: ErrorInfo) -> Response {
        Response {
            response_metadata: Some(ResponseMetadata {
                success: false,
                error_info: Some(error_info),
            }),
            resolve_hash_response: None,
            download_response: None,
            about_node_response: None,
            get_peers_info_response: None,
            node_metadata: None,
            proof: None
        }
    }

    pub fn with_auth(&mut self, key_pair: &KeyPair) -> &mut Response {
        let hash = self.calculate_hash();
        let proof = Proof::from_keypair_hash(&hash, &key_pair);
        self.proof = Some(proof);
        self
    }

    pub fn with_metadata(&mut self, node_metadata: NodeMetadata) -> &mut Response {
        self.node_metadata = Some(node_metadata);
        self
    }

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
    pub fn default_port_offset(&self) -> u16 {
        let port = match self {
            NetworkEnvironment::Main => {16380}
            NetworkEnvironment::Test => {16280}
            NetworkEnvironment::Dev => {16180}
            NetworkEnvironment::Staging => {16280}
            NetworkEnvironment::Perf => {16180}
            NetworkEnvironment::Integration => {15980}
            NetworkEnvironment::Local => {16180}
            NetworkEnvironment::Debug => {16180}
            NetworkEnvironment::All => {16180}
            NetworkEnvironment::Predev => {16080}
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

pub fn json<T: Serialize>(t: &T) -> Result<String, ErrorInfo> {
    serde_json::to_string(&t).map_err(|e| ErrorInfo::error_info(format!("serde json ser error: {}", e)))
}

impl NodeMetadata {
    pub fn port_or(&self, network: NetworkEnvironment) -> u16 {
        self.port_offset.unwrap_or(network.default_port_offset() as i64) as u16
    }
}