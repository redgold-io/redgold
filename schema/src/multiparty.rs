use crate::HashClear;
use crate::structs::{InitiateMultipartyKeygenRequest, InitiateMultipartySigningRequest};


// TODO: Eliminate these and make a separate trait for proto_serialize
impl HashClear for InitiateMultipartySigningRequest {
    fn hash_clear(&mut self) {}
}

impl HashClear for InitiateMultipartyKeygenRequest {
    fn hash_clear(&mut self) {}
}
