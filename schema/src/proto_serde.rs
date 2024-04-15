use std::fmt::{Display, Formatter};
use prost::Message;
use crate::{ErrorInfoContext, HashClear, RgResult};
use crate::structs::{ErrorCode, ErrorInfo, Hash, PublicKey};

pub trait ProtoSerde
    where Self: Message + Default,
{
    fn proto_serialize(&self) -> Vec<u8>;
    fn vec(&self) -> Vec<u8>;

    fn proto_serialize_hex(&self) -> String;
    fn hex(&self) -> String;
    fn proto_deserialize(bytes: Vec<u8>) -> RgResult<Self>;
    fn from_bytes(bytes: Vec<u8>) -> RgResult<Self>;
    fn from_bytes_ref(bytes: &Vec<u8>) -> RgResult<Self>;

    fn proto_deserialize_hex(s: impl Into<String>) -> RgResult<Self>;
    fn from_hex(s: impl Into<String>) -> RgResult<Self>;
    fn proto_deserialize_ref(bytes: &Vec<u8>) -> RgResult<Self>;
}

impl<T> ProtoSerde for T
where T: Message + Default {
    fn proto_serialize(&self) -> Vec<u8> {
        self.encode_to_vec()
    }

    fn vec(&self) -> Vec<u8> {
        self.proto_serialize()
    }

    fn proto_serialize_hex(&self) -> String {
        hex::encode(self.proto_serialize())
    }

    fn hex(&self) -> String {
        self.proto_serialize_hex()
    }

    fn proto_deserialize(bytes: Vec<u8>) -> RgResult<Self> {
        T::decode(&*bytes)
            .map_err(|e|
                crate::error_message(ErrorCode::ProtoDecoderFailure, e.to_string()))
    }

    fn from_bytes(bytes: Vec<u8>) -> RgResult<Self> {
        Self::proto_deserialize(bytes)
    }

    fn from_bytes_ref(bytes: &Vec<u8>) -> RgResult<Self> {
        Self::proto_deserialize_ref(bytes)
    }

    fn proto_deserialize_hex(s: impl Into<String>) -> RgResult<Self> {
        hex::decode(s.into())
            .error_info("hex decode")
            .and_then(|v| T::proto_deserialize(v))
    }

    fn from_hex(s: impl Into<String>) -> RgResult<Self> {
        Self::proto_deserialize_hex(s)
    }

    fn proto_deserialize_ref(bytes: &Vec<u8>) -> RgResult<Self> {
        T::decode(&**bytes)
            .map_err(|e|
                crate::error_message(ErrorCode::ProtoDecoderFailure, e.to_string()))
    }

}


pub trait ProtoHashable
where
    Self: HashClear + Clone + Message + Default,
{
    // fn proto_serialize(&self) -> Vec<u8>;
    // fn proto_deserialize(bytes: Vec<u8>) -> Result<Self, ErrorInfo>;
    fn calculate_hash(&self) -> Hash;
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

