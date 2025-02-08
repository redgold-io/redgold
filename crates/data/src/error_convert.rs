use redgold_schema::{error_message, structs::ErrorInfo, structs::ErrorCode};

/// Trait for converting various error types to ErrorInfo
pub trait ResultErrorInfoExt<T> {
    /// Convert the error type to ErrorInfo
    fn map_err_to_info(self) -> Result<T, ErrorInfo>;
}

impl<T> ResultErrorInfoExt<T> for Result<T, sqlx::Error> {
    fn map_err_to_info(self) -> Result<T, ErrorInfo> {
        self.map_err(|e| error_message(ErrorCode::InternalDatabaseError, e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Error as SqlxError;

    #[test]
    fn test_sqlx_error_conversion() {
        let err = SqlxError::RowNotFound;
        let result: Result<(), SqlxError> = Err(err);
        let converted = result.map_err_to_info();
        assert!(converted.is_err());
        let error_info = converted.unwrap_err();
        assert_eq!(error_info.code, ErrorCode::InternalDatabaseError as i32);
        assert!(error_info.message.contains("no rows returned"));
    }
}