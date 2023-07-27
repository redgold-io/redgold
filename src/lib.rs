// #![feature(result_flattening)]
#![allow(unused_imports)]
#![allow(dead_code)]
pub mod api;
pub mod async_functionality_loop;
pub mod debug_err_handle;
pub mod e2e;
pub mod core;
pub mod custodial;
pub mod data;
pub mod genesis;
pub mod gui;
pub mod infra;
pub mod node;
pub mod  node_config;
pub mod resources;
pub mod trust;
pub mod util;
pub mod multiparty;
pub mod hardware;
pub mod wallet;
pub mod observability;
pub mod integrations;

pub use redgold_schema as schema;
pub use redgold_data as datas;

#[cfg(test)]
pub mod tests {
    use super::*;
}

// #![forbid(unsafe_code)]
// #![cfg_attr(not(debug_assertions), allow(warnings))] // Forbid warnings in release builds
// #![warn(clippy::all, rust_2018_idioms)]
//
// pub mod constants;
// pub mod gui;
// pub mod mnemonic_builder;
// pub mod rg_merkle;
// pub mod sym_crypt;
// pub mod util;
// pub mod wallet;
//
// pub mod structs {
//     include!(concat!(env!("OUT_DIR"), "/structs.rs"));
// }
//
// pub use gui::image_load::Image;
// pub use gui::ClientApp;
//
// // ----------------------------------------------------------------------------
// // When compiling for web:
//
// #[cfg(target_arch = "wasm32")]
// use eframe::wasm_bindgen::{self, prelude::*};
//
// /// This is the entry-point for all the web-assembly.
// /// This is called once from the HTML.
// /// It loads the app, installs some callbacks, then returns.
// /// You can add more callbacks like this if you want to call in to your code.
// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen]
// pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
//     let app = ClientApp::default();
//     eframe::start_web(canvas_id, Box::new(app))
// }
