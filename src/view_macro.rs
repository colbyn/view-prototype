use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use serde::{self, Serialize, Deserialize};
use std::collections::HashMap;
use std::cell::{self, Cell, RefCell};
use std::rc::Rc;
use either::Either;
use web_sys::console;
use wasm_bindgen::JsValue;


///////////////////////////////////////////////////////////////////////////////
// INTERNAL
///////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! attribute_value {
    ($key:ident, true) => {
        Attribute::Toggle{
            key: String::from(stringify!($key)),
            value: true,
        }
    };
    ($key:ident, false) => {
        Attribute::Toggle{
            key: String::from(stringify!($key)),
            value: false,
        }
    };
    ($key:ident, $val:expr) => {
        Attribute::Pair{
            key: String::from(stringify!($key)),
            value: $val.to_owned(),
        }
    };
}


#[macro_export]
macro_rules! view_argument {
    ///////////////////////////////////////////////////////////////////////////
    // ATTRIBUTE
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, $key:ident = $val:tt) => {
        $node.add_attribute(attribute_value!($key, $val));
    };

    ///////////////////////////////////////////////////////////////////////////
    // STYLE
    ///////////////////////////////////////////////////////////////////////////
    // CSS DECLARATION
    ($node:expr, $key:ident : $val:expr) => {
        $node.add_style(Style::Style{
            property: String::from(stringify!($key)),
            value: $val,
        });
    };
    ($node:expr, $key:ident :: $val:tt) => {
        $node.add_style(Style::Style{
            property: String::from(stringify!($key)),
            value: CssValue(String::from(
                stringify!($val)
            )),
        });
    };
    // EMPTY PSEUDO-CLASS
    ($node:expr, : $key:ident ()) => {
        $node.add_style(Style::PseudoClass(
            String::from(stringify!($key)),
            Vec::new()
        ));
    };
    // PSEUDO-CLASS
    ($node:expr, : $key:ident $val:tt) => {{
        let mut body: Vec<Style> = Vec::new();
        style_properties_only_arguments!(body, $val);
        $node.add_style(Style::PseudoClass(
            String::from(stringify!($key)),
            body
        ));
    }};
    
    ///////////////////////////////////////////////////////////////////////////
    // EVENT HANDLER
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, . $key:ident ($value:expr)) => {
        $node.add_event_handler(
            String::from(stringify!($key)),
            Handler(Rc::new($value)),
        );
    };
    
    ///////////////////////////////////////////////////////////////////////////
    // CHILDREN
    ///////////////////////////////////////////////////////////////////////////
    // TEXT NODE
    ($node:expr, text $value:expr) => {
        $node.add_child(
            Html::Text{
                value: $value.to_owned(),
            }
        );
    };
    // EMPTY NODE
    ($node:expr, $key:ident ()) => {
        $node.add_child(
            Node::Node {
                tag: String::from(stringify!($key)),
                attributes: Vec::new(),
                styling: Vec::new(),
                children: Vec::new(),
            }
        );
    };
    // NODE
    ($node:expr, $key:ident ($($body:tt)*)) => {{
        let inner: Html = view!($key| $($body)*);
        $node.add_child(inner);
    }};
}

#[macro_export]
macro_rules! style_properties_only_arguments {
    ///////////////////////////////////////////////////////////////////////////
    // MANY
    ///////////////////////////////////////////////////////////////////////////
    ($list:expr, $key:ident : $val:expr, $($rest:tt)*) => {
        $list.push(
            Style::Style {
                property: String::from(stringify!($key)),
                value: $val,
            }
        );
        style_properties_only_arguments!(
            $list,
            $($rest)*
        );
    };
    ($list:expr, $key:ident :: $val:tt, $($rest:tt)*) => {
        $list.push(
            Style::Style {
                property: String::from(stringify!($key)),
                value: CssValue(String::from(
                    stringify!($val)
                )),
            }
        );
        style_properties_only_arguments!(
            $list,
            $($rest)*
        );
    };
    
    ///////////////////////////////////////////////////////////////////////////
    // SINGLE
    ///////////////////////////////////////////////////////////////////////////
    ($list:expr, $key:ident :: $val:tt) => {
        $list.push(Style::Style {
            property: String::from(stringify!($key)),
            value: CssValue(String::from(
                stringify!($val)
            )),
        });
    };
    ($list:expr, $key:ident : $val:expr) => {
        $list.push(Style::Style {
            property: String::from(stringify!($key)),
            value: $val,
        });
    };
    
    ///////////////////////////////////////////////////////////////////////////
    // INTERNAL - UNWRAP NESTED PARENS
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, ($($x:tt)*)) => {
        style_properties_only_arguments!(
            $node,
            $($x)*
        );
    };
}

#[macro_export]
macro_rules! view_arguments {
    ///////////////////////////////////////////////////////////////////////////
    // MANY - ATTRIBUTE
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, $key:ident = $val:tt, $($rest:tt)*) => {
        view_argument!($node, $key = $val);
        view_arguments!(
            $node,
            $($rest)*
        );
    };
    ///////////////////////////////////////////////////////////////////////////
    // MANY - CSS
    ///////////////////////////////////////////////////////////////////////////
    // CSS DECLARATION
    ($node:expr, $key:ident : $val:expr, $($rest:tt)*) => {
        view_argument!($node, $key : $val);
        view_arguments!(
            $node,
            $($rest)*
        );
    };
    ($node:expr, $key:ident :: $val:tt, $($rest:tt)*) => {
        view_argument!($node, $key :: $val);
        view_arguments!(
            $node,
            $($rest)*
        );
    };
    ($node:expr, : $key:ident $val:tt, $($rest:tt)*) => {
        view_argument!($node, : $key $val);
        view_arguments!(
            $node,
            $($rest)*
        );
    };
    
    ///////////////////////////////////////////////////////////////////////////
    // MANY - EVENT HANDLER
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, . $key:ident $value:tt, $($rest:tt)*) => {
        view_argument!($node, . $key $value);
        view_arguments!(
            $node,
            $($rest)*
        );
    };
    
    ///////////////////////////////////////////////////////////////////////////
    // MANY - CHILDREN
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, $key:ident $val:tt, $($rest:tt)*) => {
        view_argument!($node, $key $val);
        view_arguments!(
            $node,
            $($rest)*
        );
    };
    
    ///////////////////////////////////////////////////////////////////////////
    // SINGLE
    ///////////////////////////////////////////////////////////////////////////
    ($node:expr, $($rest:tt)*) => {
        view_argument!(
            $node,
            $($rest)*
        );
    };
}


///////////////////////////////////////////////////////////////////////////////
// EXTERNAL
///////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! view {
    ($tag:ident| $($x:tt)*) => {{
        let mut node = Html::new_node(String::from(
            stringify!($tag)
        ));
        view_arguments!(node, $($x)*);
        node
    }};
    ($($x:tt)*) => {{
        let mut node = Html::new_node(String::from("div"));
        view_arguments!(node, $($x)*);
        node
    }};
}
