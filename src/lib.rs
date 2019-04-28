// #![feature(impl_trait_in_bindings)]

#![allow(dead_code, unused)]

use wasm_bindgen::prelude::*;
use web_sys::console;
use wasm_bindgen::JsValue;

pub mod core;
pub mod view_macro;
pub mod css;
pub mod cssom;
pub mod html;
pub mod sync;

#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    core::test();
    Ok(())
}




