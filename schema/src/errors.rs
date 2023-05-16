use crate::{ErrorInfo, HashClear};
use crate::structs::ResponseMetadata;

impl ErrorInfo {
    pub fn error_info<S: Into<String>>(message: S) -> ErrorInfo {
        crate::error_info(message)
    }
    pub fn response_metadata(self) -> ResponseMetadata {
        ResponseMetadata {
            success: false,
            error_info: Some(self),
        }
    }
}

impl HashClear for ErrorInfo {
    fn hash_clear(&mut self) {

    }
}