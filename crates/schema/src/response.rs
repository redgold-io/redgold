use crate::helpers::easy_json::EasyJson;
use crate::structs::{AboutNodeResponse, ControlResponse, ErrorInfo, NodeMetadata, PublicKey, QueryTransactionResponse, State, SubmitTransactionResponse};
use crate::{error_info, response_metadata, HashClear, Response, ResponseMetadata, SafeOption};
use itertools::Itertools;
use prost::{DecodeError, Message};
use std::collections::{HashMap, HashSet};

impl AboutNodeResponse {
    pub fn empty() -> Self {
        AboutNodeResponse::default()
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
        let mut response = Response::default();
        response.response_metadata = response_metadata();
        response
    }

    pub fn from_error_info(error_info: ErrorInfo) -> Response {
        let mut r = Response::empty_success();
        let mut rm = response_metadata().expect("m");
        rm.success = false;
        rm.error_info = Some(error_info);
        r.response_metadata = Some(rm);
        return r.clone();
    }

    pub fn with_metadata(mut self, node_metadata: NodeMetadata) -> Response {
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

    pub fn with_error_info(self) -> Result<Self, ErrorInfo> {
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
            control_multiparty_keygen_response: None,
            control_multiparty_signing_response: None,
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


impl QueryTransactionResponse {

}

impl SubmitTransactionResponse {
    pub fn accepted(&self, expected_count: usize) -> Result<(), ErrorInfo> {
        self.check_by_state(expected_count, State::Accepted)
    }
    pub fn pending(&self, expected_count: usize) -> Result<(), ErrorInfo> {
        self.check_by_state(expected_count, State::Pending)
    }
    pub fn check_by_state(&self, expected_count: usize, state: State) -> Result<(), ErrorInfo> {
        let accepted_count = self.unique_by_state()?.iter().filter(|(_, s)| *s == state as i32).count();
        if accepted_count >= expected_count {
            Ok(())
        } else {
            Err(error_info(format!("not enough {} observations, expected {}, got {}",
                                   state.json_or(), expected_count, accepted_count)))
        }
    }
    pub fn unique_by_state(&self) -> Result<HashSet<(&PublicKey, i32)>, ErrorInfo> {
        let mut results = HashSet::new();
        for p in &self.query_transaction_response
            .safe_get()?
            .observation_proofs {
            let state = p.metadata.safe_get()?.state;
            let pk = p.proof.safe_get()?.public_key.safe_get()?;
            results.insert((pk, state));
        }
        Ok(results)
    }

    pub fn count_unique_by_state(&self) -> Result<HashMap<i32, usize>, ErrorInfo> {
        let map: HashMap<i32, usize> = self.unique_by_state()?.iter().map(|(_, y)| y.clone()).counts();
        Ok(map)
    }


    pub fn at_least_1(&self) -> Result<(), ErrorInfo> {
        self.at_least_n(1)
    }

    pub fn at_least_n(&self, n: usize) -> Result<(), ErrorInfo> {
        self.accepted(n)?;
        self.pending(n)
    }
}

impl ResponseMetadata {
    pub fn from_error(error: ErrorInfo) -> Self {
        let mut rm = response_metadata().expect("m");
        rm.success = false;
        rm.error_info = Some(error);
        rm
    }
}