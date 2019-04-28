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

use crate::css;
use crate::css::CssValue;
use crate::cssom::*;
use crate::html::*;



///////////////////////////////////////////////////////////////////////////////
// INTERNAL UTILS
///////////////////////////////////////////////////////////////////////////////

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
// FRAMEWORK
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Component<Model, Msg>
where
    Model: Debug + PartialEq + Clone + Hash,
    Msg: Debug + PartialEq + Clone + Hash
{
    model: RefCell<Model>,
    update: Rc<Fn(&mut Model, Msg)>,
    view: Rc<Fn(&Model)->Html<Msg>>,
}


#[derive(Clone)]
pub struct Process<Model, Msg>
where
    Model: Debug + PartialEq + Clone + Hash,
    Msg: Debug + PartialEq + Clone + Hash
{
    spec: Rc<Component<Model, Msg>>,
    active_vnode: Rc<RefCell<Html<Msg>>>,
    style_mount: StyleMount,
    view_mount: web_sys::Element,
}


impl<Model, Msg> Process<Model, Msg>
where
    Model: Debug + PartialEq + Clone + Hash + 'static,
    Msg: Debug + PartialEq + Clone + Hash + 'static
{
    pub fn new(spec: Component<Model, Msg>) -> Self {
        let style_mount = StyleMount::new();
        let view_mount = mk_raw_view_mount();
        let active_vnode = spec.view.as_ref()(&spec.model.clone().into_inner());
        view_mount.set_inner_html(
            active_vnode.render(&style_mount).as_str()
        );
        active_vnode.attach_event_listeners();
        Process {
            spec: Rc::new(spec),
            active_vnode: Rc::new(RefCell::new(
                active_vnode
            )),
            style_mount: style_mount,
            view_mount: view_mount,
        }
    }
    pub fn sync(&self, new: Html<Msg>) {
        let root_id = self.active_vnode.borrow().id().expect("missing id on root node");
        self.active_vnode.borrow_mut().sync(
            &mut new.clone(),
            root_id,
            &self.style_mount,
        );
    }
    pub fn tick(&self) {
        // UPDATE MODEL
        let update_model = |msg| {
            let new_model = {
                let mut model = self.spec.model.clone().into_inner();
                self.spec.update.as_ref()(&mut model, msg);
                model
            };
            self.spec.model.replace(new_model);
        };
        for msg in self.active_vnode.borrow().tick() {
            update_model(msg);
        }
        // INIT & SYNC VIEW
        let new_view = self.spec.view.as_ref()(&self.spec.model.clone().into_inner());
        self.sync(new_view);
    }
    pub fn start_loop(self) {
        use wasm_bindgen::JsCast;
        let process = self.clone();
        
        let process_callback: Rc<RwLock<Closure<Fn()>>> = Rc::new(
            RwLock::new(Closure::wrap(Box::new(|| unimplemented!())))
        );
        
        *process_callback.write().unwrap() = Closure::wrap(Box::new({
            let process = process.clone();
            let process_callback = process_callback.clone();
            move || {
                process.tick();
                web_sys::window()
                    .expect("missing window")
                    .request_animation_frame(
                        ((process_callback.clone().read().unwrap())).as_ref().unchecked_ref()
                    )
                    .expect("request_animation_frame failed");
            }
        }));
        {
            web_sys::window()
                .expect("missing window")
                .request_animation_frame(
                    ((process_callback.clone().read().unwrap())).as_ref().unchecked_ref()
                )
                .expect("request_animation_frame failed");
        }
    }
}




///////////////////////////////////////////////////////////////////////////////
// APPLICATION ENTRYPOINT
///////////////////////////////////////////////////////////////////////////////




///////////////////////////////////////////////////////////////////////////////
// DEV - IMPLEMENTATION
///////////////////////////////////////////////////////////////////////////////

pub mod app {
    use super::*;
    #[macro_use]
    use super::super::*;
    
    #[derive(Debug, PartialEq, Clone, Hash)]
    pub enum CounterMsg {
        Increment,
        Decrement,
    }
    
    #[derive(Debug, PartialEq, Clone, Hash)]
    pub struct Counter {
        value: i32,
    }
    
    pub fn init() -> Counter {
        Counter {value: 0}
    }
    
    pub fn update(counter: &mut Counter, msg: CounterMsg) {
        match msg {
            CounterMsg::Increment => {
                counter.value = counter.value + 1;
            }
            CounterMsg::Decrement => {
                counter.value = counter.value - 1;
            }
        }
    }
    
    
    pub fn view(counter: &Counter) -> Html<CounterMsg> {view!(
        display: "flex",
        flex_direction: "column",
        width: "60%",
        margin: "0 auto",
        h1(
            :hover (
                font_size: "8em",
                color: "#848484"
            ),
            display: "flex",
            justify_content: "center",
            font_family: "monospace",
            font_size: "5em",
            padding: "0",
            margin: "0",
            color: "#444",
            transition_duration: "0.5s",
            transition_timing_function: "ease",
            text(format!("{}", counter.value))
        ),
        button(
            padding: "14px",
            margin: "12px",
            border_radius: "12px",
            font_weight: "bolder",
            font_size: "2em",
            text_transform: "uppercase",
            outline: "none",
            user_select: "none",
            border: "1px solid #676767",
            background_color: "#676767",
            color: "#fff",
            .click(|event| CounterMsg::Increment),
            text("Increment")
        ),
        button(
            padding: "14px",
            margin: "12px",
            border_radius: "12px",
            font_weight: "bolder",
            font_size: "2em",
            text_transform: "uppercase",
            outline: "none",
            user_select: "none",
            border: "1px solid #676767",
            background_color: "#676767",
            color: "#fff",
            .click(|event| CounterMsg::Decrement),
            text("Decrement")
        )
    )}
}




///////////////////////////////////////////////////////////////////////////////
// DEV
///////////////////////////////////////////////////////////////////////////////


pub fn test() {
    let spec = Component {
        model: RefCell::new(
            app::init()
        ),
        update: Rc::new(app::update),
        view: Rc::new(app::view),
    };
    let process = Process::new(spec);
    process.start_loop();
}


