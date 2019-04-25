#![allow(dead_code, unused)]

use wasm_bindgen::prelude::*;
use web_sys::console;
use wasm_bindgen::JsValue;

pub mod core;


#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    fn set_panic_hook() {
        console_error_panic_hook::set_once();
    }
    set_panic_hook();
    // console::log_1(&JsValue::from("Lorem Ipsum..."));
    core::test();
    Ok(())
}




