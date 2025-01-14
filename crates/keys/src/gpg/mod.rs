
#[cfg(target_os = "linux")]
pub mod gpg_generate;
#[cfg(not(target_os = "linux"))]
pub mod gpg_generate_stub;
#[cfg(not(target_os = "linux"))]
pub use gpg_generate_stub as gpg_generate;

