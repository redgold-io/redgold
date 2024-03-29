use crate::response_metadata;
use crate::structs::{ExecutionResult, ResponseMetadata};

impl ExecutionResult {
    pub fn from_error(error: crate::structs::ErrorInfo) -> Self {
        let mut er = ExecutionResult::default();
        er.valid = false;
        er.result_metadata = Some(ResponseMetadata::from_error(error.clone()));
        er
    }
}