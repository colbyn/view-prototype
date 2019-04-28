use std::fmt;
use std::fmt::Debug;
use std::convert::From;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::collections::HashMap;
use std::collections::BTreeSet;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::cell::{self, Cell, RefCell};
use std::sync::Once;
use std::sync::RwLock;
use std::rc::Rc;
use either::Either;
use serde::{self, Serialize, Deserialize};
use web_sys::console;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure;
use wasm_bindgen::closure::Closure;

use crate::html;

#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Patch<Msg> {
    SetChildText {parent_id: String, value: String},
    SetNode {id: String, value: html::Html<Msg>},
    SetChildren {id: String, value: Vec<html::Html<Msg>>},
}

impl<Msg> Patch<Msg> {
    pub fn id(&self) -> Option<String> {
        match &self {
            Patch::SetChildText{parent_id, ..} => Some(parent_id.clone()),
            Patch::SetNode{id, ..} => Some(id.clone()),
            Patch::SetChildren{id, ..} => Some(id.clone()),
        }
    }
}

pub fn get_patches_with_id<Msg: Clone>(patches: &Vec<Patch<Msg>>, id: String) -> Vec<Patch<Msg>> {
    let mut results: Vec<Patch<Msg>> = Vec::new();
    for patch in patches {
        match patch.id() {
            Some(pid) => {
                if pid == id {
                    results.push(patch.clone());
                }
            },
            _ => ()
        }
    }
    results
}



