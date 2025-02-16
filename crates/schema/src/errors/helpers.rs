use metrics::counter;
use crate::RgResult;

pub trait WithMetrics {
    fn with_err_count(self, counter: impl Into<String>) -> Self;
}

impl<T> WithMetrics for RgResult<T> {
    fn with_err_count(self, counter: impl Into<String>) -> Self {
        counter!(counter.into()).increment(1);
        self
    }
}