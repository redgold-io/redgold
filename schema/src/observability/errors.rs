use log::{error, Level};
use tracing::{event, Event, Metadata};
use tracing::field::FieldSet;
use crate::{HashClear, RgResult, structs};
use crate::helpers::easy_json::EasyJson;
use crate::structs::{ErrorDetails, ErrorInfo, ResponseMetadata};

pub fn convert_log_level(level: String) -> log::Level {
    match level.to_lowercase().as_str() {
        "trace" => log::Level::Trace,
        "debug" => log::Level::Debug,
        "info" => log::Level::Info,
        "warn" => log::Level::Warn,
        "error" => log::Level::Error,
        _ => log::Level::Error,
    }
}

pub fn convert_trace_log_level(level: String) -> tracing::Level {
    match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::ERROR,
    }
}

pub trait Loggable<T> {
    fn log_error(&self) -> RgResult<T>;
}

impl<T> Loggable<T> for RgResult<T> where T: Clone {
    // TODO: Better here to do a map_err and then return self to avoid the clone?
    fn log_error(&self) -> RgResult<T> {
        self.as_ref()
            .map_err(|e| {
                let e2 = e.clone();
                if !e2.skip_logging {
                    let ser = e2.json_or();
                    if let Some(l) = e2.internal_log_level.as_ref(){
                        let level = convert_trace_log_level(l.clone());
                        // TODO: Finish this
                        // let metadata = Metadata::new("log_error", "?", level, None, None, None, FieldSet::new(&[],), &[], &[]);
                        // let event = Event::new(&metadata, &ser);
                        // tracing::dispatcher::get_default(|dispatcher| {
                        //     dispatcher.event(&event);
                        // });
                        // event!(level, "{}", ser);
                    } else {
                        error!("{}", ser);
                    }
                }
                e.clone()
            }).map(|t| t.clone())
    }
}

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
    pub fn with_code(&mut self, v: structs::ErrorCode) {
        self.code = v as i32;
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
    fn with_code(self, v: structs::ErrorCode) -> RgResult<T>;
    fn with_detail(self, k: impl Into<String>, v: impl Into<String>) -> RgResult<T>;
    fn with_detail_fn<F>(self, k: impl Into<String>, v: impl Fn() -> F) -> RgResult<T>
    where F: Into<String> + Sized;
    fn mark_skip_log(self) -> RgResult<T>;
    fn level(self, l: Level) -> RgResult<T>;
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
    fn with_code(self, v: structs::ErrorCode) -> RgResult<T> {
        self.map_err(|mut e| {
            e.with_code(v);
            e
        })
    }
    fn with_detail(self, k: impl Into<String>, v: impl Into<String>) -> RgResult<T> {
        self.map_err(|mut e| {
            e.with_detail(k, v);
            e
        })
    }


    fn with_detail_fn<F>(self, k: impl Into<String>, v: impl Fn() -> F) -> RgResult<T>
    where F: Into<String> + Sized{
        self.map_err(|mut e| {
            let v = v().into();
            e.with_detail(k, v);
            e
        })
    }

    fn mark_skip_log(self) -> RgResult<T> {
        self.map_err(|mut e| {
            e.skip_logging = true;
            e
        })
    }

    fn level(self, l: Level) -> RgResult<T> {
        self.map_err(|mut e| {
            e.internal_log_level = Some(l.to_string());
            e
        })
    }

}
