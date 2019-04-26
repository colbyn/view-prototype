use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use serde::{self, Serialize, Deserialize};
use std::collections::HashMap;
use std::cell::{self, Cell, RefCell};
use either::Either;

///////////////////////////////////////////////////////////////////////////////
// BASICS
///////////////////////////////////////////////////////////////////////////////

type GUID = usize;
type HashId = usize;


///////////////////////////////////////////////////////////////////////////////
// DOM-TREE REPRESENTATION
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Hash)]
pub enum Attribute {
    Pair {
        key: String,
        value: String,
    },
    Toggle {
        key: String,
        value: bool,
    }
}

impl Attribute {
    pub fn is_pair(&self) -> bool {
        match &self {
            Attribute::Pair{..} => true,
            _ => false,
        }
    }
    pub fn key(&self) -> String {
        match &self {
            Attribute::Pair{key, ..} => key.clone(),
            Attribute::Toggle{key, ..} => key.clone(),
        }
    }
    pub fn value(&self) -> Option<String> {
        match &self {
            Attribute::Pair{value, ..} => Some(value.clone()),
            Attribute::Toggle{..} => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Hash)]
pub enum Style {
    Style {
        property: String,
        value: CssValue,
    },
    PseudoClass(String, Vec<Style>),
}

impl Style {
    pub fn render_decls(selector: &String, styles: &Vec<Style>) -> String {
        let mut inner: Vec<String> = Vec::new();
        for style in styles {
            match style.render_decl() {
                Some(decl) => inner.push(decl),
                _ => {}
            }
        }
        format!(
            "{selector} {{{body}}}",
            selector=selector,
            body=inner.join(" "),
        )
    }
    pub fn render_decl(&self) -> Option<String> {
        match &self {
            Style::Style{property, value: CssValue(value)} => {
                let property = property.replace("_", "-");
                Some(format!(
                    "{prop}: {value};",
                    prop=property,
                    value=value,
                ))
            },
            Style::PseudoClass(name, body) => None,
        }
    }
    pub fn render_pseudo_selector(&self, css_hash: u64) -> Option<String> {
        match &self {
            Style::Style{property, value: CssValue(value)} => None,
            Style::PseudoClass(pseudo_name, body) => {
                let selector = format!(
                    "._{css_hash}:{pseudo_name}",
                    css_hash=css_hash,
                    pseudo_name=pseudo_name,
                );
                Some(Style::render_decls(&selector, body))
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Hash)]
pub struct CssValue(String);

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Hash)]
pub enum Node {
    Node {
        tag: String,
        attributes: Vec<Attribute>,
        styling: Vec<(Style)>,
        children: Vec<Node>,
    },
    Text {
        value: String,
    }
}

impl Node {
    ///////////////////////////////////////////////////////////////////////////
    // INTERNAL HELPERS
    ///////////////////////////////////////////////////////////////////////////
    fn stringify_attributes(&self) -> Option<String> {
        fn set_hash_class(hash: u64, attributes: &mut Vec<Attribute>) {
            let mut class_set = false;
            for attr in attributes.iter_mut() {
                match attr {
                    Attribute::Pair{key, value} if key == "class" => {
                        class_set = true;
                        value.push_str(format!("_{hash}", hash=hash).as_str());
                    },
                    _ => ()
                }
            }
            if !class_set {
                attributes.push(
                    Attribute::Pair{
                        key: String::from("class"),
                        value: format!("_{hash}", hash=hash),
                    }
                );
            }
        }
        match (self.get_css_hash(), &self) {
            (Some(hash), Node::Node{attributes,..}) => {
                let mut attributes = attributes.clone();
                set_hash_class(hash, &mut attributes);
                let attributes: String = attributes
                    .iter()
                    .map(|atr| {
                        if atr.is_pair() {
                            format!(
                                "{k}=\"{v}\"",
                                k=atr.key(),
                                v=atr.value().unwrap(),
                            )
                        } else {
                            atr.key()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
                Some(attributes)
            },
            _ => None
        }
    }
    
    
    ///////////////////////////////////////////////////////////////////////////
    // EXTERNAL API
    ///////////////////////////////////////////////////////////////////////////
    pub fn stringify_html(&self) -> String {
        match &self {
            Node::Node{tag, attributes, children,..} => {
                let attributes: Option<String> = self.stringify_attributes();
                let children: String = children
                    .iter()
                    .map(|c| c.stringify_html())
                    .collect::<Vec<String>>()
                    .join("\n");
                
                if attributes.is_none() {
                    format!(
                        "<{tag}>{children}</{tag}>",
                        tag=tag,
                        children=children,
                    )
                } else {
                    format!(
                        "<{tag} {attributes}>{children}</{tag}>",
                        tag=tag,
                        attributes=attributes.unwrap(),
                        children=children,
                    )
                }
            }
            Node::Text{value} => {value.clone()}
        }
    }
    
    pub fn get_css_hash(&self) -> Option<u64> {
        match &self {
            Node::Node{styling, ..} => Some(calculate_hash(&styling)),
            Node::Text{..} => None,
        }
    }
    
    pub fn stringify_css(&self) -> Option<(u64, String)> {
        let hash = self.get_css_hash();
        match (hash, &self) {
            (Some(hash), Node::Node{styling, ..}) => {
                let class_selector = format!("._{hash}", hash=hash);
                let class_decl = Style::render_decls(&class_selector, styling);
                let pseudo_decls = {
                    let mut contents: Vec<String> = Vec::new();
                    for style in styling {
                        match style.render_pseudo_selector(hash) {
                            None => (),
                            Some(rendered) => contents.push(rendered),
                        }
                    }
                    contents.join(" ")
                };
                let result = format!("{}\n{}", class_decl, pseudo_decls);
                Some((hash, result))
            },
            _ => None
        }
    }
    pub fn add_attribute(&mut self, attribute: Attribute) {
        match self {
            Node::Node{ref mut attributes, ..} => {
                attributes.push(attribute);
            }
            Node::Text{..} => {panic!()}
        }
    }
    pub fn add_style(&mut self, style: Style) {
        match self {
            Node::Node{ref mut styling, ..} => {
                styling.push(style);
            }
            Node::Text{..} => {panic!()}
        }
    }
    pub fn add_child(&mut self, child: Node) {
        match self {
            Node::Node{ref mut children, ..} => {
                children.push(child);
            }
            Node::Text{..} => {panic!()}
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// CSS VALUES
///////////////////////////////////////////////////////////////////////////////
pub mod css {
    use super::*;
    
    pub mod value {
        use super::*;
        
        ///////////////////////////////////////////////////////////////////////////
        // COLORS
        ///////////////////////////////////////////////////////////////////////////
        pub fn rgb(r: u32, g: u32, b: u32) -> CssValue {
            CssValue(format!(
                "rgb({r},{g},{b})",
                r=r,
                g=g,
                b=b,
            ))
        }
        pub fn hex(x: &str) -> CssValue {
            CssValue(x.to_owned())
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// VIEW MACRO
///////////////////////////////////////////////////////////////////////////////

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
    // CHILDREN
    ///////////////////////////////////////////////////////////////////////////
    // TEXT NODE
    ($node:expr, text $value:expr) => {
        $node.add_child(
            Node::Text{
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
        let inner: Node = view!($key| $($body)*);
        $node.add_child(inner);
    }};
}

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


#[macro_export]
macro_rules! view {
    ($tag:ident| $($x:tt)*) => {{
        let mut node = Node::Node {
            tag: String::from(stringify!($tag)),
            attributes: Vec::new(),
            styling: Vec::new(),
            children: Vec::new(),
        };
        view_arguments!(node, $($x)*);
        node
    }};
    ($($x:tt)*) => {{
        let mut node = Node::Node {
            tag: String::from("div"),
            attributes: Vec::new(),
            styling: Vec::new(),
            children: Vec::new(),
        };
        view_arguments!(node, $($x)*);
        node
    }};
}



///////////////////////////////////////////////////////////////////////////////
// REACTOR
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Reactor {
    active_vnode: Node,
    style_mount: web_sys::Element,
    view_mount: web_sys::Element,
}


impl Reactor {
    ///////////////////////////////////////////////////////////////////////////
    // INTERNAL HELPERS
    ///////////////////////////////////////////////////////////////////////////
    fn init_mounts() -> (web_sys::Element, web_sys::Element) {
        let window: web_sys::Window = web_sys::window()
            .expect("window not available");
        let document = window
            .document()
            .expect("document not available");
        let body: web_sys::Node = std::convert::From::from(
            document.body().expect("document.body not available")
        );
        let style_mount: web_sys::Node = std::convert::From::from(
            document.create_element("style").unwrap()
        );
        let view_mount: web_sys::Node = std::convert::From::from(
            document.create_element("div").unwrap()
        );
        body.append_child(&style_mount);
        body.append_child(&view_mount);
        let style_mount = {
            let style_mount: wasm_bindgen::JsValue = std::convert::From::from(style_mount);
            let style_mount: web_sys::Element = std::convert::From::from(style_mount);
            style_mount
        };
        let view_mount = {
            let view_mount: wasm_bindgen::JsValue = std::convert::From::from(view_mount);
            let view_mount: web_sys::Element = std::convert::From::from(view_mount);
            view_mount
        };
        (style_mount, view_mount)
    }
    
    fn init_view(node: &Node, style_mount: &web_sys::Element, view_mount: &web_sys::Element) {
        let markup = node.stringify_html();
        let (hash, styles) = node.stringify_css().expect("initial css");
        style_mount.set_inner_html(styles.as_str());
        view_mount.set_inner_html(markup.as_str());
    }
    
    ///////////////////////////////////////////////////////////////////////////
    // EXTERNAL API
    ///////////////////////////////////////////////////////////////////////////
    pub fn new(initial: Node) -> Self {
        let (style_mount, view_mount) = Reactor::init_mounts();
        Reactor::init_view(&initial, &style_mount, &view_mount);
        Reactor {
            active_vnode: initial,
            style_mount: style_mount,
            view_mount: view_mount,
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// INTERNAL UTILS
///////////////////////////////////////////////////////////////////////////////

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////


pub fn test() {
    use web_sys::console;
    use wasm_bindgen::JsValue;
    use css::value::*;
    
    let node = view!(h1|
        :hover (
            color: hex("#999")
        ),
        color: hex("#000"),
        display::flex,
        justify_content::center,
        text("Hello World")
    );
    let mut reactor = Reactor::new(node);
}


