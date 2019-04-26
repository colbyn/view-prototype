#![allow(dead_code, unused)]

use wasm_bindgen::prelude::*;
use web_sys::console;
use wasm_bindgen::JsValue;

pub mod core;


#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    core::test();
    Ok(())
}




