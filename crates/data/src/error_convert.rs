use redgold_schema::{error_code, RgResult};
use redgold_schema::structs::ErrorInfo;
use sqlx::Error as SqlxError;

/// A trait for converting errors into ErrorInfo
pub trait IntoErrorInfo {
    fn into_error_info(self) -> ErrorInfo;
}

/// Extension trait for Result types to convert errors into ErrorInfo
pub trait ResultErrorInfoExt<T> {
    fn map_err_to_info(self) -> RgResult<T>;
}

impl<T, E: IntoErrorInfo> ResultErrorInfoExt<T> for Result<T, E> {
    fn map_err_to_info(self) -> RgResult<T> {
        self.map_err(|e| e.into_error_info())
    }
}

impl IntoErrorInfo for SqlxError {
    fn into_error_info(self) -> ErrorInfo {
        ErrorInfo {
            code: error_code::SQL_ERROR,
            description: "SQL error occurred".to_string(),
            description_extended: format!("SQL error details: {}", self),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Error;

    #[test]
    fn test_sqlx_error_conversion() {
        let err = Error::RowNotFound;
        let error_info = err.into_error_info();
        assert_eq!(error_info.code, error_code::SQL_ERROR);
        assert!(error_info.description.contains("SQL error"));
        assert!(error_info.description_extended.contains("RowNotFound"));
    }

    #[test]
    fn test_result_conversion() {
        let result: Result<i32, SqlxError> = Err(Error::RowNotFound);
        let converted: RgResult<i32> = result.map_err_to_info();
        assert!(converted.is_err());
        let err = converted.unwrap_err();
        assert_eq!(err.code, error_code::SQL_ERROR);
    }
}
