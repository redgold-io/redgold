use crate::{error_info, RgResult};
use crate::structs::ErrorInfo;


pub trait ToErrorInfo {
    fn to_error<T>(&self) -> RgResult<T>;
}

impl ToErrorInfo for String {
    fn to_error<T>(&self) -> RgResult<T> {
        Err(error_info(self))
    }
}

impl ToErrorInfo for &str {
    fn to_error<T>(&self) -> RgResult<T> {
        Err::<T, ErrorInfo>(error_info(self.to_string()))
    }
}
