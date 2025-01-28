use crate::structs::{InitiateMultipartyKeygenRequest, InitiateMultipartySigningRequest, MultipartyIdentifier};
use crate::{structs, HashClear};


// TODO: Eliminate these and make a separate trait for proto_serialize
impl HashClear for InitiateMultipartySigningRequest {
    fn hash_clear(&mut self) {}
}

impl HashClear for InitiateMultipartyKeygenRequest {
    fn hash_clear(&mut self) {}
}

impl MultipartyIdentifier {
    pub fn party_index(&self, pk: &structs::PublicKey) -> Option<usize> {
        self.party_keys.iter().enumerate().find(|(_, k)| k == &pk).map(|(i, _)| i + 1)
    }
}

impl InitiateMultipartySigningRequest {
    pub fn party_index(&self, pk: &structs::PublicKey) -> Option<usize> {
        self.signing_party_keys.iter().enumerate().find(|(_, k)| k == &pk).map(|(i, _)| i + 1)
    }
}
