use futures::TryFutureExt;
use log::error;
use redgold_schema::structs::ErrorInfo;
use redgold_schema::json_or;

pub trait Loggable<T> {
    fn log_error(&self) -> Result<&T, &ErrorInfo>;
}

impl<T> Loggable<T> for Result<T, ErrorInfo> {
    fn log_error(&self) -> Result<&T, &ErrorInfo> {
        self.as_ref().map_err(|e| {
            let e2 = e.clone();
            error!("{}", json_or(&e2));
            e
        })
    }
}