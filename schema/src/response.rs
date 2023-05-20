use prost::{DecodeError, Message};
use crate::{HashClear, KeyPair, ProtoHashable, Response, response_metadata, ResponseMetadata, SafeOption};
use crate::structs::{AboutNodeResponse, ControlResponse, ErrorInfo, MultipartyThresholdResponse, NodeMetadata, Proof};

impl AboutNodeResponse {
    pub fn empty() -> Self {
        Self {
            latest_metadata: None,
            latest_node_metadata: None,
            num_known_peers: 0,
            num_active_peers: 0,
            recent_transactions: vec![],
            pending_transactions: 0,
            total_accepted_transactions: 0,
            observation_height: 0,
        }
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
            response_metadata: crate::response_metadata(),
            resolve_hash_response: None,
            download_response: None,
            about_node_response: None,
            get_peers_info_response: None,
            node_metadata: None,
            proof: None,
            multiparty_threshold_response: None,
            submit_transaction_response: None,
        }
    }
    pub fn from_error_info(error_info: ErrorInfo) -> Response {
        let mut r = Response::empty_success();
        r.response_metadata = Some(ResponseMetadata {
            success: false,
            error_info: Some(error_info)
        });
        return r.clone();
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

    pub fn as_error_info(&self) -> Result<(), ErrorInfo> {
        let res = self.response_metadata.safe_get()?;
        if let Some(e) = &res.error_info {
            return Err(e.clone());
        }
        Ok(())
    }

    pub fn with_error_info(&self) -> Result<&Self, ErrorInfo> {
        let res = self.response_metadata.safe_get()?;
        if let Some(e) = &res.error_info {
            return Err(e.clone());
        }
        Ok(self)
    }


}


impl ControlResponse {
    pub fn empty() -> Self {
        Self {
            response_metadata: response_metadata(),
            initiate_multiparty_keygen_response: None,
            initiate_multiparty_signing_response: None,
        }
    }

    // TODO: Trait duplicate
    pub fn as_error_info(&self) -> Result<(), ErrorInfo> {
        let res = self.response_metadata.safe_get()?;
        if let Some(e) = &res.error_info {
            return Err(e.clone());
        }
        Ok(())
    }
}

impl MultipartyThresholdResponse {
    pub fn empty() -> Self {
        Self {
            multiparty_issue_unique_index_response: None,
            initiate_keygen_response: None,
            initiate_signing_response: None,
        }
    }
}