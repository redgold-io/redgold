use crate::{ErrorInfo, HashClear, RgResult};
use crate::structs::{ErrorDetails, ResponseMetadata};

impl ErrorInfo {
    pub fn error_info<S: Into<String>>(message: S) -> ErrorInfo {
        crate::error_info(message)
    }
    pub fn new(message: impl Into<String>) -> ErrorInfo {
        crate::error_info(message.into())
    }

    pub fn response_metadata(self) -> ResponseMetadata {
        ResponseMetadata {
            success: false,
            error_info: Some(self),
            task_local_details: vec![],
            request_id: None,
            trace_id: None,
        }
    }
    pub fn enhance(self, message: impl Into<String>) -> ErrorInfo {
        let mut e = self;
        e.message = format!("{} {} ", e.message, message.into());
        e
    }
    pub fn with_detail(&mut self, k: impl Into<String>, v: impl Into<String>) {
        let mut ed = ErrorDetails::default();
        ed.detail = v.into();
        ed.detail_name = k.into();
        self.details.push(ed);
    }
}

impl HashClear for ErrorInfo {
    fn hash_clear(&mut self) {

    }
}

pub trait EnhanceErrorInfo<T> {
    fn add(self, message: impl Into<String>) -> RgResult<T>;
    fn mark_abort(self) -> RgResult<T>;
    fn bubble_abort(self) -> RgResult<RgResult<T>>;
    fn with_detail(self, k: impl Into<String>, v: impl Into<String>) -> RgResult<T>;
}

impl<T> EnhanceErrorInfo<T> for RgResult<T> {
    fn add(self, message: impl Into<String>) -> RgResult<T> {
        self.map_err(|e| e.enhance(message))
    }
    fn mark_abort(self) -> RgResult<T> {
        self.map_err(|mut e| {
            e.abort = true;
            e
        })
    }
    fn bubble_abort(self) -> RgResult<RgResult<T>> {
        match self {
            Ok(r) => {Ok(Ok(r))}
            Err(e) => {
                if !e.abort {
                    Ok(Err(e))
                } else {
                    Err(e)
                }
            }
        }
    }

    fn with_detail(self, k: impl Into<String>, v: impl Into<String>) -> RgResult<T> {
        self.map_err(|mut e| {
            e.with_detail(k, v);
            e
        })
    }

}