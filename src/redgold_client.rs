// #![forbid(unsafe_code)]
// #![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
// #![warn(clippy::all, rust_2018_idioms)]
// use crate::{gui, Image};
//
// // https://github.com/emilk/egui_template/
// // https://emilk.github.io/egui/index.html
//
// // When compiling natively:
// #[cfg(not(target_arch = "wasm32"))]
// fn main() {
//     let bytes = include_bytes!("logo.jpg");
//     let image = Image::decode(bytes).unwrap();
//     let app = gui::ClientApp::from_logo(image);
//     let native_options = eframe::NativeOptions::default();
//     eframe::run_native(Box::new(app), native_options);
// }
