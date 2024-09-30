
#[cfg(not(any(target_arch = "wasm32", target_os = "wasi")))]
pub mod task_local_impl;

#[cfg(any(target_arch = "wasm32", target_os = "wasi"))]
pub mod task_local_stub;

#[cfg(any(target_arch = "wasm32", target_os = "wasi"))]
pub use self::task_local_stub as task_local_impl;

