use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use serde::{self, Serialize, Deserialize};
use std::collections::HashMap;
use std::cell::{self, Cell, RefCell};
use std::rc::Rc;
use either::Either;
use web_sys::console;
use wasm_bindgen::JsValue;
use uuid::Uuid;


///////////////////////////////////////////////////////////////////////////////
// PATH MISC.
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone, Hash)]
pub struct UrlPath(Vec<PathSegment>);

impl UrlPath {
    pub fn is_index(&self) -> bool {
        self.0.is_empty()
    }
    pub fn unpack(&self) -> Vec<PathSegment> {
        self.0.clone()
    }
    pub fn parse(path: String) -> Self {
        let path: &str = path.split("?").collect::<Vec<&str>>()[0];
        let path_segs: Vec<PathSegment> = path.split("/")
            .filter(|x| !x.is_empty())
            .map(|x| PathSegment::Static(x.to_owned()))
            .collect();
        UrlPath(path_segs)
    }
    pub fn from_segs(segs: Vec<PathSegment>) -> Self {
        UrlPath(segs)
    }
    pub fn static_matches(r1: &UrlPath, r2: &UrlPath) -> bool {
        let r1 = &r1.0;
        let r2 = &r2.0;
        if r1.len() == r2.len() {
            let result = r1.iter().zip(r2.iter()).all(|(x1, x2)| {
                match (&x1, &x2) {
                    (PathSegment::Static(s1), PathSegment::Static(s2)) => {s1 == s2}
                    _ => {true}
                }
            });
            result
        } else {
            false
        }
    }
}

#[derive(Debug, PartialEq, Clone, Hash)]
pub enum PathSegment {
    Static(String),
    Wildcard,
    Binder
}

impl PathSegment {
    pub fn unpack_string(&self) -> Option<String> {
        match self {
            PathSegment::Static(x) => Some(x.clone()),
            _ => None
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// MACRO - INTERNAL
///////////////////////////////////////////////////////////////////////////////


#[macro_export]
macro_rules! build_patterns {
    // MANY - STATIC PATH
    ($xs:expr; $path:expr, $($rest:tt)*) => {
        $xs.push(PathSegment::Static($path.to_owned()));
        build_patterns!($xs; $($rest)*);
    };
    // MANY - PARAMETERIZED PATH
    ($xs:expr; $name:ident : $ty:ty, $($rest:tt)*) => {
        $xs.push(PathSegment::Binder);
        build_patterns!($xs; $($rest)*);
    };
    // SINGLE - EMPTY
    ($xs:expr;) => {};
    // SINGLE - PARAMETERIZED PATH
    ($xs:expr; $name:ident : $ty:ty) => {
        $xs.push(PathSegment::Binder);
    };
    // SINGLE - STATIC PATH
    ($xs:expr; $path:expr) => {
        $xs.push(PathSegment::Static($path.to_owned()));
    };
}


#[macro_export]
macro_rules! init_binders {
    // EMPTY - DONE
    ($xs:expr; $return_value:expr; $body:expr;) => {
        if $return_value.is_none() {
            $return_value = Some($body);
        }
    };
    // MANY
    ($xs:expr; $return_value:expr; $body:expr; $name:ident : $ty:tt, $($rest:tt)*) => {
        if $return_value.is_none() && ($xs.len() >= 1) {
            let current_segment = $xs.remove(0).unpack_string().expect("should be a string");
            let result: Option<$ty> = std::str::FromStr::from_str(current_segment.as_str()).ok();
            if let Some($name) = result {
                init_binders!($xs; $return_value; $body; $($rest)*);
            }
        }
    };
    // MANY - SKIP STATIC
    ($xs:expr; $return_value:expr; $body:expr; $other:expr, $($rest:tt)*) => {
        if $xs.len() >= 1 {
            $xs.remove(0);
            init_binders!($xs; $return_value; $body; $($rest)*);
        }
    };
    // DONE
    ($xs:expr; $return_value:expr; $body:expr; $name:tt : $ty:tt) => {
        if $return_value.is_none() && ($xs.len() >= 1) {
            let current_segment = $xs.remove(0).unpack_string().expect("should be a string");
            let result: Option<$ty> = std::str::FromStr::from_str(current_segment.as_str()).ok();
            if let Some($name) = result {
                if $xs.is_empty() {
                    $return_value = Some($body);
                }
            }
        }
    };
    // DONE - STATIC
    ($xs:expr; $return_value:expr; $body:expr; $other:expr) => {
        if $return_value.is_none() {
            $return_value = Some($body);
        }
    };
}


#[macro_export]
macro_rules! path_entry {
    // INDEX - EMPTY
    ($raw_input:expr; $return_value:expr; [] => $body:expr) => {
        if $return_value.is_none() {
            let given_route = UrlPath::parse($raw_input.clone());
            if given_route.is_index() {
                $return_value = Some($body);
            }
        }
    };
    // PATH SEGMENTS
    ($raw_input:expr; $return_value:expr; [$($xs:tt)*] => $body:expr) => {
        if $return_value.is_none() {
            let given_route = UrlPath::parse($raw_input.clone());
            let route_pattern: UrlPath = {
                let mut xs: Vec<PathSegment> = Vec::new();
                build_patterns!(xs; $($xs)*);
                UrlPath::from_segs(xs)
            };
            if UrlPath::static_matches(&given_route, &route_pattern) {
                let mut route: Vec<PathSegment> = given_route.unpack();
                init_binders!(route; $return_value; $body; $($xs)*);
            }
        }
    };
    // WILDCARD
    ($input:expr; $return_value:expr; _ => $ex:tt) => {
        if $return_value.is_none() {
            $return_value = Some($ex);
        }
    };
}


///////////////////////////////////////////////////////////////////////////////
// MACRO - EXTERNAL
///////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! match_path {
    ($($ps:tt => $ex:tt)*) => {Rc::new(
        move |raw_input: String| {
            let mut result = None;
            {$(
                path_entry!(raw_input; result; $ps => $ex);
            )*}
            result
        }
    )};
}




///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////

pub mod app {
    use super::*;
    
    #[derive(Debug, PartialEq, Clone, Hash)]
    pub enum Route {
        RootIndex,
        ContentIndex,
        ContentItem {
            uid: Uuid
        },
        AccountIndex,
        AccountUser {
            user_name: String
        },
        NotFound
    }
}

pub fn test() {
    use app::Route;
    
    let matcher: Rc<Fn(String)->Option<app::Route>> = match_path!(
        [] => {
            Route::RootIndex
        }
        ["content"] => {
            Route::ContentIndex
        }
        ["content", uid: Uuid] => {
            Route::ContentItem {uid: uid}
        }
        ["account"] => {
            Route::AccountIndex
        }
        ["account", user_name: String] => {
            Route::AccountUser {user_name: user_name}
        }
        _ => {
            Route::NotFound
        }
    );
    console::log_1(&JsValue::from(
        format!("Result: {:#?}", matcher.as_ref()(String::from("/content")))
    ));
}

