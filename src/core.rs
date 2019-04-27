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

use crate::css;
use crate::css::CssValue;

///////////////////////////////////////////////////////////////////////////////
// HTML ATTRIBUTES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone, Hash)]
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


///////////////////////////////////////////////////////////////////////////////
// CSS STYLING
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Style {
    Style {
        property: String,
        value: String,
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
            Style::Style{property, value} => {
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
    pub fn render_pseudo_selector(&self, node_id: &String) -> Option<String> {
        match &self {
            Style::Style{..} => None,
            Style::PseudoClass(pseudo_name, body) => {
                let selector = format!(
                    "#{id}:{pseudo_name}",
                    id=node_id,
                    pseudo_name=pseudo_name,
                );
                Some(Style::render_decls(&selector, body))
            },
        }
    }
}

// #[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Hash)]
// pub struct CssValue(pub String);


///////////////////////////////////////////////////////////////////////////////
// EVENTS
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Handler (pub Rc<Fn(JsValue)>);

impl Handler {
    pub fn eval(&self, arg: JsValue) {
        match &self {
            Handler(fun) => {
                fun(arg);
            }
        }
    }
}

impl Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "Handler")
    }
}

impl PartialEq for Handler {
    fn eq(&self, other: &Handler) -> bool {true}
}

impl Hash for Handler {
    fn hash<H: Hasher>(&self, state: &mut H) {}
}




///////////////////////////////////////////////////////////////////////////////
// VIRTUAL-DOM NODE
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Html {
    Node {
        tag: String,
        id: String,
        attributes: Vec<Attribute>,
        styling: Vec<(Style)>,
        events: BTreeMap<String, Handler>,
        children: Vec<Html>,
    },
    Text {
        value: String,
    }
}

impl Html {
    ///////////////////////////////////////////////////////////////////////////
    // INTERNAL HELPERS
    ///////////////////////////////////////////////////////////////////////////
    fn render_attributes(&self) -> Option<String> {
        match &self {
            Html::Node{attributes,..} => {
                let attributes: String = attributes
                    .iter()
                    .filter(|atr| atr.key() != "id")
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
    fn render_css(&self, style_mount: &StyleMount) {
        pub fn default_selector(style_mount: &StyleMount, id: &String, styles: &Vec<Style>) {
            let class_selector = format!("#{id}", id=id);
            let rule = Style::render_decls(&class_selector, styles);
            style_mount.insert(&rule);
        }
        pub fn pseudo_selectors(style_mount: &StyleMount, id: &String, styles: &Vec<Style>) {
            let mut rules: Vec<String> = Vec::new();
            for style in styles {
                match style.render_pseudo_selector(id) {
                    None => (),
                    Some(rendered) => rules.push(rendered),
                }
            }
            for rule in rules {
                style_mount.insert(&rule);
            }
        }
        match &self {
            Html::Node{styling, id, ..} => {
                default_selector(style_mount, &id, styling);
                pseudo_selectors(style_mount, &id, styling);
            },
            _ => ()
        }
    }
    
    fn add_event_listeners(&self) {
        fn add_handler(element: &web_sys::Element, event_name: &str, handler: &Handler) {
            use wasm_bindgen::JsCast;
            
            let element: web_sys::EventTarget = From::from(element.clone());
            let closure: Closure<dyn FnMut(JsValue)> = Closure::wrap(Box::new({
                let handler = handler.clone();
                move |value: JsValue| {
                    handler.eval(value);
                }
            }));
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let result = element.add_event_listener_with_callback(
                event_name,
                function,
            );
            closure.forget();
            result.expect("unable to add event listener");
        }
        
        match (self.get_live().as_ref(), &self) {
            (Some(live), Html::Node{id, children, events, ..}) => {
                for child in children {
                    child.add_event_listeners();
                }
                for (event_name, event_handler) in events {
                    add_handler(live, event_name, event_handler);
                }
            },
            _ => ()
        }
    }
    
    
    ///////////////////////////////////////////////////////////////////////////
    // GETTER UTILS
    ///////////////////////////////////////////////////////////////////////////
    pub fn id(&self) -> Option<String> {
        match &self {
            Html::Node{id, ..} => Some(id.clone()),
            Html::Text{..} => None,
        }
    }
    fn events(&self) -> Option<BTreeMap<String, Handler>> {
        match self {
            Html::Node{events, ..} => Some(
                events.clone()
            ),
            Html::Text{..} => None
        }
    }
    
    fn get_live(&self) -> Option<web_sys::Element> {
        let window: web_sys::Window = web_sys::window()
            .expect("window not available");
        let document = window
            .document()
            .expect("document not available");
        match self.id() {
            Some(id) => {
                document.get_element_by_id(id.as_str())
            },
            None => None
        }
    }
    
    
    ///////////////////////////////////////////////////////////////////////////
    // CONSTRUCTION
    ///////////////////////////////////////////////////////////////////////////
    pub fn new_node(tag: String) -> Html {
        Html::Node {
            tag: tag,
            id: format!("_{}", rand::random::<u16>()),
            attributes: Vec::new(),
            styling: Vec::new(),
            events: BTreeMap::new(),
            children: Vec::new(),
        }
    }
    
    ///////////////////////////////////////////////////////////////////////////
    // EXTERNAL - API
    ///////////////////////////////////////////////////////////////////////////
    pub fn render(&self, style_mount: &StyleMount) -> String {
        match &self {
            Html::Node{tag, id, attributes, children,..} => {
                self.render_css(style_mount);
                let attributes: Option<String> = self.render_attributes();
                let children: String = children
                    .iter()
                    .map(|c| c.render(style_mount))
                    .collect::<Vec<String>>()
                    .join("");
                
                if attributes.is_none() {
                    format!(
                        "<{tag} id={id}>{children}</{tag}>",
                        id=id,
                        tag=tag,
                        children=children,
                    )
                } else {
                    format!(
                        "<{tag} id={id} {attributes}>{children}</{tag}>",
                        id=id,
                        tag=tag,
                        attributes=attributes.unwrap(),
                        children=children,
                    )
                }
            }
            Html::Text{value} => {value.clone()}
        }
    }
    pub fn add_attribute(&mut self, attribute: Attribute) {
        match self {
            Html::Node{ref mut attributes, ..} => {
                attributes.push(attribute);
            }
            Html::Text{..} => {panic!()}
        }
    }
    pub fn add_style(&mut self, style: Style) {
        match self {
            Html::Node{ref mut styling, ..} => {
                styling.push(style);
            }
            Html::Text{..} => {panic!()}
        }
    }
    pub fn add_event_handler(&mut self, event: String, handler: Handler) {
        match self {
            Html::Node{ref mut events, ..} => {
                events.insert(event, handler);
            }
            Html::Text{..} => {panic!()}
        }
    }
    pub fn add_child(&mut self, child: Html) {
        match self {
            Html::Node{ref mut children, ..} => {
                children.push(child);
            }
            Html::Text{..} => {panic!()}
        }
    }
}



///////////////////////////////////////////////////////////////////////////////
// INTERNAL - DOM - VIEW-MOUNT POINT
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ViewMount {
    mount: web_sys::Element,
}


///////////////////////////////////////////////////////////////////////////////
// INTERNAL - DOM - STYLE-MOUNT POINT
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct StyleMount {
    mount: web_sys::HtmlStyleElement,
}


impl StyleMount {
    pub fn new() -> Self {
        StyleMount {
            mount: mk_raw_style_mount(),
        }
    }
    pub fn log(&self) {
        let rules: web_sys::StyleSheet = self.mount.sheet().expect("missing sheet property");
        let rules: wasm_bindgen::JsValue = std::convert::From::from(
            self.mount.sheet().expect("missing sheet property")
        );
        let rules: web_sys::CssStyleSheet = std::convert::From::from(rules);
        let rule_list: web_sys::CssRuleList = rules.css_rules().expect("missing cssRules property");
        for ix in (1..rule_list.length()).map(|x| x - 1).rev() {
            let rule: web_sys::CssRule = rule_list.item(ix).expect("rule index error");
            let rule: wasm_bindgen::JsValue = std::convert::From::from(rule);
            let rule: web_sys::CssStyleRule = std::convert::From::from(rule);
            let selector = rule.selector_text();
            console::log_1(&JsValue::from_str(
                selector.as_str()
            ))
        }
    }
    pub fn delete(&self, node_id: String) {
        let rules: web_sys::StyleSheet = self.mount.sheet().expect("missing sheet property");
        let rules: wasm_bindgen::JsValue = std::convert::From::from(
            self.mount.sheet().expect("missing sheet property")
        );
        let rules: web_sys::CssStyleSheet = std::convert::From::from(rules);
        let rule_list: web_sys::CssRuleList = rules.css_rules().expect("missing cssRules property");
        for ix in (1..rule_list.length()).map(|x| x - 1).rev() {
            let rule: web_sys::CssRule = rule_list.item(ix).expect("rule index error");
            let rule: wasm_bindgen::JsValue = std::convert::From::from(rule);
            let rule: web_sys::CssStyleRule = std::convert::From::from(rule);
            let selector = rule.selector_text();
            if selector.contains(node_id.as_str()) {
                rules.delete_rule(ix).expect("unable to delete css rule");
            }
        }
    }
    pub fn insert(&self, contents: &String) {
        let rules: web_sys::StyleSheet = self.mount.sheet().expect("missing sheet property");
        let rules: wasm_bindgen::JsValue = std::convert::From::from(
            self.mount.sheet().expect("missing sheet property")
        );
        let rules: web_sys::CssStyleSheet = std::convert::From::from(rules);
        rules.insert_rule(contents.as_str()).expect("failed to insert rule");
    }
}


///////////////////////////////////////////////////////////////////////////////
// REACTOR
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Reactor {
    active_vnode: Html,
    style_mount: StyleMount,
    view_mount: web_sys::Element,
}


impl  Reactor {
    ///////////////////////////////////////////////////////////////////////////
    // INTERNAL HELPERS
    ///////////////////////////////////////////////////////////////////////////
    fn init_view(node: &Html, style_mount: &StyleMount, view_mount: &web_sys::Element) {
        let markup = node.render(style_mount);
        view_mount.set_inner_html(markup.as_str());
        node.add_event_listeners();
    }
    
    ///////////////////////////////////////////////////////////////////////////
    // EXTERNAL API
    ///////////////////////////////////////////////////////////////////////////
    pub fn new(initial: Html) -> Self {
        let style_mount = StyleMount::new();
        let view_mount = mk_raw_view_mount();
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

fn mk_raw_style_mount() -> web_sys::HtmlStyleElement {
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
    body.append_child(&style_mount);
    let style_mount = {
        let style_mount: wasm_bindgen::JsValue = std::convert::From::from(style_mount);
        let style_mount: web_sys::HtmlStyleElement = std::convert::From::from(style_mount);
        style_mount
    };
    style_mount
}

fn mk_raw_view_mount() -> web_sys::Element {
    let window: web_sys::Window = web_sys::window()
        .expect("window not available");
    let document = window
        .document()
        .expect("document not available");
    let body: web_sys::Node = std::convert::From::from(
        document.body().expect("document.body not available")
    );
    let view_mount: web_sys::Node = std::convert::From::from(
        document.create_element("div").unwrap()
    );
    body.append_child(&view_mount);
    let view_mount = {
        let view_mount: wasm_bindgen::JsValue = std::convert::From::from(view_mount);
        let view_mount: web_sys::Element = std::convert::From::from(view_mount);
        view_mount
    };
    view_mount
}


///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////


pub fn test() {
    #[macro_use]
    use super::*;
    
    let node = view!(
        h1(
            :hover (
                color: "#999"
            ),
            color: "#000",
            display: "flex",
            justify_content: "center",
            .click(move |event| {
                console::log_1(&JsValue::from_str("event handler...."));
                console::log_1(&event);
            }),
            .click(move |event| {
                console::log_1(&JsValue::from_str("event handler...."));
                console::log_1(&event);
            }),
            text("Hello World")
        )
    );
    console::log_1(&JsValue::from(format!(
        "{:#?}",
        node
    )));
    let mut reactor = Reactor::new(node);
}


