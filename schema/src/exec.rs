use crate::response_metadata;
use crate::structs::{ExecutionResult, ResponseMetadata};

impl ExecutionResult {
    pub fn from_error(error: crate::structs::ErrorInfo) -> Self {
        ExecutionResult {
            valid: false,
            result_metadata: Some(ResponseMetadata::from_error(error.clone())),
            data: None,
        }
    }
}