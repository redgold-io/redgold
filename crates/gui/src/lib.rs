#[allow(unused_imports)]
#[allow(dead_code)]
#[allow(unused_variables)]

#[cfg(target_os = "linux")]
pub mod image_capture;
#[cfg(target_os = "linux")]
pub mod image_capture_openpnp;
pub mod components;
pub mod common;
pub mod dependencies;

pub mod state;


#[cfg(not(target_os = "linux"))]
pub mod image_capture_stub;
pub mod airgap;
pub mod data_query;
pub mod tab;
pub mod functionality;
pub mod wasm_app;

#[cfg(not(target_os = "linux"))]
pub use image_capture_stub as image_capture;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
