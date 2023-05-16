use crate::structs::ControlRequest;

// Can we remove all references to this and just replace with Default?
impl ControlRequest {
    pub fn empty() -> Self {
        Self {
            initiate_multiparty_keygen_request: None,
            initiate_multiparty_signing_request: None,
        }
    }
}