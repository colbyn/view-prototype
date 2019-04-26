#![allow(dead_code, unused)]

use wasm_bindgen::prelude::*;
use web_sys::console;
use wasm_bindgen::JsValue;

pub mod core;
pub mod view_macro;


#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    core::test();
    Ok(())
}




