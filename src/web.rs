//! Browser entry points, called from index.html.
//!
//! `start_desktop` runs the full outliner desktop on a canvas (needs WebGL).
//! `eval_self` and friends drive a text-only shell on the same Self engine,
//! used when the browser cannot draw the desktop.

use crate::vm::{Event, Value, Vm};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    static LITE: RefCell<Option<Vm>> = const { RefCell::new(None) };
    static STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[wasm_bindgen]
pub async fn start_desktop(canvas_id: String) -> Result<(), JsValue> {
    let document = web_sys::window().ok_or("no window")?.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id(&canvas_id)
        .ok_or("missing canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("not a canvas"))?;
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(crate::app::SelfWorldApp::new(cc)))),
        )
        .await
}

fn set(o: &js_sys::Object, k: &str, v: JsValue) {
    let _ = js_sys::Reflect::set(o, &JsValue::from_str(k), &v);
}

/// Evaluate Self code in the text shell. Returns
/// `{ ok, result, printed, trace: [..], ms }`.
#[wasm_bindgen]
pub fn eval_self(src: &str) -> JsValue {
    // The VM is taken out while it runs, so that if the browser ever aborts
    // a run (for example its own stack limit) the next call starts afresh.
    let mut taken = LITE.with(|cell| cell.borrow_mut().take());
    let restarted = taken.is_none() && LITE.with(|c| c.borrow().is_none()) && STARTED.with(|s| s.get());
    STARTED.with(|s| s.set(true));
    let vm = taken.get_or_insert_with(Vm::new);
    let out = {
        let shell = Value::Ref(vm.sp.shell);
        let t0 = crate::vm::now_ms();
        let r = vm.eval_doit(src, shell);
        let ms = crate::vm::now_ms() - t0;
        let o = js_sys::Object::new();
        let (ok, result, trace) = match r {
            Ok(v) => (true, vm.print_string(v), vec![]),
            Err(e) => (false, e.msg, e.trace),
        };
        let mut printed = String::new();
        if restarted {
            printed.push_str("(the previous run was stopped by the browser, so the world was restarted)\n");
        }
        let mut inspected = vec![];
        for e in vm.events.drain(..) {
            match e {
                Event::Print(s) => printed.push_str(&s),
                Event::Inspect(v) => inspected.push(v),
                Event::ClearTranscript => {}
            }
        }
        for v in inspected {
            let d = vm.describe(v);
            printed.push_str(&format!("{d}:\n"));
            for s in vm.slots_of(v) {
                let name = if s.parent { format!("{}*", s.name) } else { s.name.clone() };
                let val = if s.is_method { "a method".to_string() } else { vm.describe(s.value) };
                printed.push_str(&format!("  {name} = {val}\n"));
            }
        }
        if vm.needs_gc() {
            vm.collect_garbage(&[]);
        }
        set(&o, "ok", JsValue::from_bool(ok));
        set(&o, "result", JsValue::from_str(&result));
        set(&o, "printed", JsValue::from_str(&printed));
        set(&o, "ms", JsValue::from_f64(ms));
        let t = js_sys::Array::new();
        for line in trace {
            t.push(&JsValue::from_str(&line));
        }
        set(&o, "trace", t.into());
        o
    };
    LITE.with(|cell| *cell.borrow_mut() = taken);
    out.into()
}

/// Throw away the text shell's world, so the next evaluation starts from a
/// fresh lobby (used by the playground's "New world" button).
#[wasm_bindgen]
pub fn reset_world() {
    LITE.with(|cell| *cell.borrow_mut() = None);
    STARTED.with(|s| s.set(false));
}

/// The example programs, as `[[title, code], ...]`.
#[wasm_bindgen]
pub fn example_list() -> JsValue {
    let a = js_sys::Array::new();
    for (t, c) in crate::app::EXAMPLES {
        let pair = js_sys::Array::new();
        pair.push(&JsValue::from_str(t));
        pair.push(&JsValue::from_str(c));
        a.push(&pair);
    }
    a.into()
}

/// Hand a file chosen in the browser's picker to the desktop.
#[wasm_bindgen]
pub fn queue_file(name: String, text: String) {
    crate::app::PENDING_FILES.with(|p| p.borrow_mut().push((name, text)));
}

/// Load a Self source file in the text shell. Returns
/// `{ ok, result, printed, notes: [..], ms }`.
#[wasm_bindgen]
pub fn load_self(name: &str, text: &str) -> JsValue {
    let mut taken = LITE.with(|cell| cell.borrow_mut().take());
    STARTED.with(|s| s.set(true));
    let vm = taken.get_or_insert_with(Vm::new);
    let t0 = crate::vm::now_ms();
    let r = vm.load_source(text);
    let ms = crate::vm::now_ms() - t0;
    let o = js_sys::Object::new();
    let notes = js_sys::Array::new();
    let (ok, result) = match r {
        Ok(rep) => {
            for f in rep.failures.iter().take(30) {
                notes.push(&JsValue::from_str(f));
            }
            if rep.failures.len() > 30 {
                notes.push(&JsValue::from_str(&format!("… and {} more", rep.failures.len() - 30)));
            }
            let mut s = format!("loaded {name}: {} statements, {} failed", rep.statements, rep.failures.len());
            if let (Some(v), true) = (rep.last, rep.failures.is_empty()) {
                s = format!("{s} — last result: {}", vm.print_string(v));
            }
            (true, s)
        }
        Err(e) => (false, format!("{name}: {}", e.msg)),
    };
    let mut printed = String::new();
    for e in vm.events.drain(..) {
        if let Event::Print(s) = e {
            printed.push_str(&s);
        }
    }
    set(&o, "ok", JsValue::from_bool(ok));
    set(&o, "result", JsValue::from_str(&result));
    set(&o, "printed", JsValue::from_str(&printed));
    set(&o, "ms", JsValue::from_f64(ms));
    set(&o, "notes", notes.into());
    set(&o, "trace", js_sys::Array::new().into());
    LITE.with(|cell| *cell.borrow_mut() = taken);
    o.into()
}

/// Self source translated to readable JavaScript. Throws a syntax error.
#[wasm_bindgen]
pub fn self_to_javascript(src: &str) -> Result<String, JsValue> {
    crate::jsgen::translate(src).map_err(|e| JsValue::from_str(&e))
}

/// A whole web page (runtime, starting world, morph renderer and the
/// program) that runs the Self program in any browser.
#[wasm_bindgen]
pub fn self_web_app(src: &str, title: &str) -> Result<String, JsValue> {
    crate::jsgen::web_app(src, title).map_err(|e| JsValue::from_str(&e))
}

/// The apps written in Self (examples/), as `[[title, file, code], ...]`.
#[wasm_bindgen]
pub fn app_list() -> JsValue {
    let a = js_sys::Array::new();
    for (t, _, file, src) in crate::app::APPS {
        let row = js_sys::Array::new();
        row.push(&JsValue::from_str(t));
        row.push(&JsValue::from_str(file));
        row.push(&JsValue::from_str(src));
        a.push(&row);
    }
    a.into()
}
