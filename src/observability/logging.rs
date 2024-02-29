use futures::TryFutureExt;
use log::error;
use redgold_schema::structs::ErrorInfo;
use redgold_schema::{json_or, RgResult};

pub trait Loggable<T> {
    fn log_error(&self) -> RgResult<T>;
}

impl<T> Loggable<T> for RgResult<T> where T: Clone {
    // TODO: Better here to do a map_err and then return self to avoid the clone?
    fn log_error(&self) -> RgResult<T> {
        self.as_ref()
            .map_err(|e| {
            let e2 = e.clone();
            error!("{}", json_or(&e2));
            e.clone()
        }).map(|t| t.clone())
    }
}