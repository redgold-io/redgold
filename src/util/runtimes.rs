use std::sync::Arc;
use std::thread;
use tokio::runtime::{Builder, Runtime};

pub fn build_runtime(threads: usize, name: impl Into<String>) -> Arc<Runtime> {
    Arc::new(
        build_simple_runtime(threads, name),
    )
}

pub fn build_simple_runtime(threads: usize, name: impl Into<String> + Sized) -> Runtime {
    Builder::new_multi_thread()
        .worker_threads(threads)
        .thread_name(name)
        .thread_stack_size(512 * 1024 * 1024)
        .enable_all()
        .enable_time()
        .build()
        .unwrap()
}

pub fn big_thread() -> std::thread::Builder {
    thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
}
