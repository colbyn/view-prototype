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
use std::rc::Rc;
use either::Either;
use serde::{self, Serialize, Deserialize};
use web_sys::console;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure;
use wasm_bindgen::closure::Closure;

use crate::css;
use crate::css::CssValue;
use crate::cssom::*;



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
pub struct Handler<Msg> {
    pub fun: Rc<Fn(JsValue)->Msg>,
    pub js_ref: js_sys::Function,
}

impl<Msg> Handler<Msg> {
    pub fn eval(&self, arg: JsValue) -> Msg {
        self.fun.as_ref()(arg)
    }
}

impl<Msg> Debug for Handler<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "Handler")
    }
}

impl<Msg> PartialEq for Handler<Msg> {
    fn eq(&self, other: &Handler<Msg>) -> bool {true}
}

impl<Msg> Hash for Handler<Msg> {
    fn hash<H: Hasher>(&self, state: &mut H) {}
}


///////////////////////////////////////////////////////////////////////////////
// MAILBOX
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq)]
pub struct Mailbox(Rc<RefCell<VecDeque<(String, JsValue)>>>);

impl Mailbox {
    pub fn unpack(&self) -> Rc<RefCell<VecDeque<(String, JsValue)>>> {
        self.0.clone()
    }
    pub fn new() -> Self {
        Mailbox(Rc::new(RefCell::new(VecDeque::new())))
    }
    pub fn insert(&self, event_name: String, value: JsValue) {
        self.unpack().borrow_mut().push_back((event_name, value));
    }
    pub fn remove(&self) -> Option<(String, JsValue)> {
        self.unpack().borrow_mut().pop_front()
    }
}

impl Hash for Mailbox {
    fn hash<H: Hasher>(&self, state: &mut H) {}
}


///////////////////////////////////////////////////////////////////////////////
// VIRTUAL-DOM NODE
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Html<Msg> {
    Node {
        tag: String,
        id: String,
        attributes: Vec<Attribute>,
        styling: Vec<(Style)>,
        events: BTreeMap<String, Handler<Msg>>,
        mailbox: Mailbox,
        children: Vec<Html<Msg>>,
    },
    Text {
        value: String,
    }
}

impl<Msg> Html<Msg> {
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
    
    pub fn attach_event_listeners(&self) {
        match (self.get_live().as_ref(), &self) {
            (Some(live), Html::Node{id, children, events, ..}) => {
                for child in children {
                    child.attach_event_listeners();
                }
                for (event_name, event_handler) in events {
                    let result = live.add_event_listener_with_callback(
                        event_name,
                        &event_handler.js_ref,
                    );
                }
            },
            _ => ()
        }
    }
    pub fn delete_event_listeners(&self) {
        match &self {
            Html::Node{children, events, ..} => {
                // CHILDREN FIRST
                for child in children {
                    child.delete_event_listeners();
                }
                // CURRENT NODE
                let live = self.get_live().expect("failed to get live dom ref");
                let live: web_sys::EventTarget = From::from(live.clone());
                for (name, handler) in events {
                    live.remove_event_listener_with_callback(
                        name.as_str(),
                        &handler.js_ref
                    ).expect("unable to remove event handler");
                }
            },
            Html::Text{..} => (),
        }
    }
    
    ///////////////////////////////////////////////////////////////////////////
    // EVENT-SYSTEM
    ///////////////////////////////////////////////////////////////////////////
    pub fn tick(&self) -> Vec<Msg> {
        let mut messages: Vec<Msg> = Vec::new();
        // CHILDREN FIRST
        match &self {
            Html::Text{..} => {}
            Html::Node{children, ..} => {
                for child in children {
                    messages.append(&mut child.tick())
                }
            }
        }
        // CURRENT
        match &self {
            Html::Text{..} => (),
            Html::Node{mailbox, ..} => {
                match mailbox.remove() {
                    None => {},
                    Some((name, value)) => {
                        match self.lookup_handler(&name) {
                            None => (),
                            Some(handler) => messages.push(handler.eval(value)),
                        }
                    }
                }
            }
        }
        messages
    }
    
    
    ///////////////////////////////////////////////////////////////////////////
    // SYNC VIEW CHANGES
    ///////////////////////////////////////////////////////////////////////////
    pub fn sync(&mut self, new: &mut Html<Msg>, parent_ref: &web_sys::Element) {
        let live = self.get_live();
        match (self, new) {
            (Html::Node{children: cs1, ..}, Html::Node{children: cs2, ..}) => {
                let live = live.expect("failed to get live dom ref");
                if cs1.len() == cs2.len() {
                    for (c1, c2) in cs1.iter_mut().zip(cs2.iter_mut()) {
                        c1.sync(c2, &live);
                    }
                }
            },
            (Html::Text{value: v1}, Html::Text{value: v2}) => {
                if v1 != v2 {
                    parent_ref.set_text_content(Some(v2.as_str()));
                    *v1 = v2.clone();
                }
            },
            _ => ()
        }
    }
    
    
    ///////////////////////////////////////////////////////////////////////////
    // GETTER/SETTER UTILS
    ///////////////////////////////////////////////////////////////////////////
    pub fn id(&self) -> Option<String> {
        match &self {
            Html::Node{id, ..} => Some(id.clone()),
            Html::Text{..} => None,
        }
    }
    fn events(&self) -> Option<&BTreeMap<String, Handler<Msg>>> {
        match self {
            Html::Node{events, ..} => Some(events),
            Html::Text{..} => None
        }
    }
    fn get_mail(&self) -> Option<(String, JsValue)> {
        match self {
            Html::Node{mailbox: Mailbox(queue), ..} => {
                queue.borrow_mut().pop_front()
            },
            Html::Text{..} => None
        }
    }
    fn get_mailbox(&self) -> Option<Rc<Mailbox>> {
        match self {
            Html::Node{mailbox, ..} => Some(
                Rc::new(mailbox.clone())
            ),
            Html::Text{..} => None
        }
    }
    fn lookup_handler(&self, key: &String) -> Option<&Handler<Msg>> {
        match &self {
            Html::Node{events, ..} => {
                match events.get(key) {
                    Some(handler) => Some(handler),
                    None => None
                }
            },
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
    pub fn new_node(tag: String) -> Html<Msg> {
        Html::Node {
            tag: tag,
            id: format!("_{}", rand::random::<u16>()),
            attributes: Vec::new(),
            styling: Vec::new(),
            events: BTreeMap::new(),
            mailbox: Mailbox::new(),
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
    pub fn add_event_handler(&mut self, event_name: String, fun: Rc<Fn(JsValue)->Msg>) {
        let handler: Handler<Msg> = {
            use wasm_bindgen::JsCast;
            let mailbox: Rc<Mailbox> = self.get_mailbox().expect("missing mailbox");
            let closure: Closure<dyn FnMut(JsValue)> = Closure::wrap(Box::new(
                {
                    let mailbox = mailbox.clone();
                    let event_name = event_name.to_owned();
                    move |value: JsValue| {
                        mailbox.insert(event_name.clone(), value);
                    }
                }
            ));
            let function: &js_sys::Function = closure.as_ref().unchecked_ref();
            let function: js_sys::Function = function.clone();
            closure.forget();
            let handler =  Handler {
                fun: fun,
                js_ref: function
            };
            handler
        };
        match self {
            Html::Node{ref mut events, ..} => {
                events.insert(event_name, handler);
            }
            Html::Text{..} => {panic!()}
        }
    }
    pub fn add_child(&mut self, child: Html<Msg>) {
        match self {
            Html::Node{ref mut children, ..} => {
                children.push(child);
            }
            Html::Text{..} => {panic!()}
        }
    }
}

