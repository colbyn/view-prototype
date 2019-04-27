use std::fmt;
use std::fmt::Debug;
use std::convert::From;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::collections::BTreeMap;
use std::cell::{self, Cell, RefCell};
use std::rc::Rc;
use either::Either;
use serde::{self, Serialize, Deserialize};
use web_sys::console;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure;
use wasm_bindgen::closure::Closure;



pub trait CssValue {
    fn stringify(&self) -> String;
}

impl CssValue for String {
    fn stringify(&self) -> String {
        self.clone()
    }
}

impl CssValue for &str {
    fn stringify(&self) -> String {
        self.clone().to_owned()
    }
}



///////////////////////////////////////////////////////////////////////////
// COLORS
///////////////////////////////////////////////////////////////////////////
pub fn rgb(r: u32, g: u32, b: u32) -> impl CssValue {
    format!(
        "rgb({r},{g},{b})",
        r=r,
        g=g,
        b=b,
    )
}

pub fn hex(x: &str) -> impl CssValue {
    x.to_owned()
}


