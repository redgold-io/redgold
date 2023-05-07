use crate::structs::ControlRequest;

impl ControlRequest {
    pub fn empty() -> Self {
        Self {
            add_peer_full_request: None,
            initiate_multiparty_keygen_request: None,
            initiate_multiparty_signing_request: None,
        }
    }
}