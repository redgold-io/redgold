
// pub fn add_one(x: i32) -> i32 {
//     return x + 1;
// }

// src/lib.rs


#[no_mangle]
pub extern "C" fn add_one(x: i32) -> i32 {
    x + 1
}

// use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
pub extern "C" fn add_one2(x: i32) -> i32 {
    x + 1
}

