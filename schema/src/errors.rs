use crate::ErrorInfo;

impl ErrorInfo {
    pub fn error_info<S: Into<String>>(message: S) -> ErrorInfo {
        crate::error_info(message)
    }
}