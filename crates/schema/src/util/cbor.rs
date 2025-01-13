use serde::{Deserialize, Serialize};
use crate::{ErrorInfoContext, RgResult};

pub trait SerdeCborConverters {
    fn to_cbor(&self) -> RgResult<Vec<u8>>;
    fn from_cbor(payload: Vec<u8>) -> RgResult<Self> where Self: Sized;
}

impl<T> SerdeCborConverters for T where T: Serialize + for<'a> Deserialize<'a> {
    fn to_cbor(&self) -> RgResult<Vec<u8>> {
        let cbor = serde_cbor::to_vec(&self).error_info("cbor serialization failed")?;
        Ok(cbor)
    }

    fn from_cbor(payload: Vec<u8>) -> RgResult<Self> {
        let tx = serde_cbor::from_slice(&payload).error_info("cbor deserialization failed")?;
        Ok(tx)
    }
}