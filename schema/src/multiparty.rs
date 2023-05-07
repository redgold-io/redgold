use crate::HashClear;
use crate::structs::{InitiateMultipartyKeygenRequest, InitiateMultipartySigningRequest, MultipartyThresholdRequest};


// TODO: Eliminate these and make a separate trait for proto_serialize
impl HashClear for InitiateMultipartySigningRequest {
    fn hash_clear(&mut self) {}
}

impl HashClear for InitiateMultipartyKeygenRequest {
    fn hash_clear(&mut self) {}
}

impl MultipartyThresholdRequest {
    pub fn empty() -> Self {
        Self {
            multiparty_broadcast: None,
            multiparty_issue_unique_index: None,
            multiparty_subscribe: None,
            multiparty_subscribe_events: vec![],
            initiate_keygen: None,
            initiate_signing: None,
        }
    }
}