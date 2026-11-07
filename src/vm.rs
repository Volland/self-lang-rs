//! The Self object memory and interpreter.
//!
//! Objects are collections of slots. Messages are looked up through parent
//! slots exactly as described in the Self Language Reference (local slots
//! first, then every parent, with cycle detection and ambiguity errors).
//! Methods run in activations whose lexical chain blocks capture; `^` in a
//! block returns from the block's home method.

use crate::parser::{line_col, parse_doit, parse_file, parse_slots, Expr, ObjLit, SlotDesc, SlotKind, Stmt};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// A tiny FNV-style hasher for the lookup cache keys.
#[derive(Default)]
pub struct FastHasher(u64);
impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0 ^ n).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    }
    fn write_u32(&mut self, n: u32) {
        self.write_u64(n as u64)
    }
}

fn sel_hash(sel: &str, nargs: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ nargs as u64;
    for b in sel.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h
}
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use std::rc::Rc;

pub type ObjId = u32;

/// Name of the temporary parent slot given to objects created by bootstrap
/// paths before their own source file defines them.
const STAND_IN: &str = "standInParent";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Ref(ObjId),
}

impl Value {
    pub fn id(self) -> Option<ObjId> {
        match self {
            Value::Ref(i) => Some(i),
            _ => None,
        }
    }
    pub fn same(self, o: Value) -> bool {
        match (self, o) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Ref(a), Value::Ref(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub name: Rc<str>,
    pub value: Value,
    pub assignable: bool,
    pub parent: bool,
    pub arg: bool,
    pub category: Rc<str>,
}

pub enum Kind {
    Plain,
    Method(Rc<ObjLit>),
    Block { lit: Rc<ObjLit>, env: Rc<Frame> },
    Str(Rc<str>),
    Vector(Vec<Value>),
    /// An integer too big for 64 bits.
    Big(Rc<BigInt>),
    Free,
}

pub struct Obj {
    pub slots: Vec<Slot>,
    pub kind: Kind,
    pub creator: Option<(ObjId, Rc<str>)>,
    pub comment: Option<Rc<str>>,
}

pub struct Home {
    id: u64,
    alive: Cell<bool>,
}

pub struct Local {
    name: Rc<str>,
    value: Value,
    assignable: bool,
}

pub struct Frame {
    /// True for a parenthesised sub-expression with its own locals (not an activation).
    paren: bool,
    locals: RefCell<Vec<Local>>,
    self_val: Value,
    holder: Option<ObjId>,
    parent: Option<Rc<Frame>>,
    home: Rc<Home>,
}

#[derive(Debug, Clone)]
pub struct SelfError {
    pub msg: String,
    pub trace: Vec<String>,
}

/// What happened when a source file was loaded.
#[derive(Debug, Clone)]
pub struct LoadReport {
    pub statements: usize,
    pub failures: Vec<String>,
    pub last: Option<Value>,
}

pub enum Unwind {
    Return { home: u64, value: Value },
    Error(SelfError),
    /// `_Restart`: run the activation (identified by its frame) again from the top.
    Restart(usize),
}

type R<T> = Result<T, Unwind>;

#[cold]
#[inline(never)]
fn err<T>(msg: impl Into<String>) -> R<T> {
    Err(Unwind::Error(SelfError { msg: msg.into(), trace: vec![] }))
}

/// Something the interpreter wants the user interface to do.
#[derive(Debug, Clone)]
pub enum Event {
    Print(String),
    Inspect(Value),
    ClearTranscript,
}

#[derive(Clone, Copy)]
struct Found {
    holder: ObjId,
    index: usize,
    assign: bool,
}

/// What the outliner shows for one slot.
#[derive(Clone, Debug)]
pub struct SlotView {
    pub name: String,
    pub value: Value,
    pub assignable: bool,
    pub parent: bool,
    pub arg: bool,
    pub is_method: bool,
    pub category: String,
    pub virtual_slot: bool,
}

pub struct Specials {
    pub lobby: ObjId,
    pub globals: ObjId,
    pub traits: ObjId,
    pub nil: ObjId,
    pub true_: ObjId,
    pub false_: ObjId,
    pub default_behavior: ObjId,
    pub t_integer: ObjId,
    pub t_float: ObjId,
    pub t_string: ObjId,
    pub t_block: ObjId,
    pub t_method: ObjId,
    pub shell: ObjId,
}

/// Browsers give WebAssembly a fixed call stack of about 1 MB, which fits
/// roughly 1200 nested Self activations; stay well below it.
#[cfg(target_arch = "wasm32")]
const MAX_DEPTH: usize = 700;
#[cfg(not(target_arch = "wasm32"))]
const MAX_DEPTH: usize = 20000;

pub struct Vm {
    pub heap: Vec<Obj>,
    free: Vec<ObjId>,
    pub sp: Specials,
    next_home: u64,
    depth: usize,
    pub steps: u64,
    pub step_limit: u64,
    pub events: Vec<Event>,
    allocated_since_gc: usize,
    rng: u64,
    pub start_ms: f64,
    /// (lookup start object, selector hash) -> slot found. Cleared whenever
    /// slots are added/removed or a parent slot changes.
    cache: HashMap<(u32, u64), Found, BuildHasherDefault<FastHasher>>,
    /// Folder of the source file being loaded, so `bootstrap read: 'x' From: 'core'`
    /// can find its sibling files (desktop only).
    pub source_dir: Option<std::path::PathBuf>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    load_nesting: usize,
    /// `traits bootstrapPath`, remembered so original files that rebuild
    /// `traits` don't break the loader.
    bootstrap_traits: Option<ObjId>,
    /// The slots of the lobby, globals, traits, defaultBehavior and their
    /// children right after start-up, so the objects a person adds or changes
    /// can be saved as a Self file (see fileout.rs).
    pub initial_slots: Vec<(ObjId, Vec<(Rc<str>, Value)>)>,
}

pub fn now_ms() -> f64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

fn plain(slots: Vec<Slot>) -> Obj {
    Obj { slots, kind: Kind::Plain, creator: None, comment: None }
}

impl Vm {
    pub fn new() -> Vm {
        let mut vm = Vm {
            heap: Vec::with_capacity(4096),
            free: vec![],
            sp: Specials {
                lobby: 0,
                globals: 0,
                traits: 0,
                nil: 0,
                true_: 0,
                false_: 0,
                default_behavior: 0,
                t_integer: 0,
                t_float: 0,
                t_string: 0,
                t_block: 0,
                t_method: 0,
                shell: 0,
            },
            next_home: 1,
            depth: 0,
            steps: 0,
            step_limit: 30_000_000,
            events: vec![],
            allocated_since_gc: 0,
            rng: (now_ms() as u64) ^ 0x9E37_79B9_7F4A_7C15,
            start_ms: now_ms(),
            cache: HashMap::default(),
            source_dir: None,
            load_nesting: 0,
            bootstrap_traits: None,
            initial_slots: vec![],
        };
        vm.bootstrap();
        vm.initial_slots = vm.snapshot_slots();
        vm
    }

    // ------------------------------------------------------------ heap

    pub fn alloc(&mut self, o: Obj) -> ObjId {
        self.allocated_since_gc += 1;
        if let Some(id) = self.free.pop() {
            self.heap[id as usize] = o;
            id
        } else {
            self.heap.push(o);
            (self.heap.len() - 1) as ObjId
        }
    }

    pub fn obj(&self, id: ObjId) -> &Obj {
        &self.heap[id as usize]
    }

    pub fn nil(&self) -> Value {
        Value::Ref(self.sp.nil)
    }
    pub fn boolean(&self, b: bool) -> Value {
        Value::Ref(if b { self.sp.true_ } else { self.sp.false_ })
    }

    pub fn new_string(&mut self, s: &str) -> Value {
        Value::Ref(self.alloc(Obj { slots: vec![], kind: Kind::Str(s.into()), creator: None, comment: None }))
    }

    pub fn string_of(&self, v: Value) -> Option<Rc<str>> {
        match v {
            Value::Ref(id) => match &self.obj(id).kind {
                Kind::Str(s) => Some(s.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn slot(name: &str, value: Value) -> Slot {
        Slot { name: name.into(), value, assignable: false, parent: false, arg: false, category: "".into() }
    }
    fn parent_slot(name: &str, value: Value) -> Slot {
        Slot { parent: true, ..Self::slot(name, value) }
    }

    fn set_creator_if_none(&mut self, v: Value, holder: ObjId, name: &Rc<str>) {
        if let Value::Ref(id) = v {
            let o = &mut self.heap[id as usize];
            if o.creator.is_none() && matches!(o.kind, Kind::Plain | Kind::Vector(_)) && id != holder {
                o.creator = Some((holder, name.clone()));
            }
        }
    }

    fn bootstrap(&mut self) {
        // nil must exist first so every other slot can be initialised to it.
        let nil = self.alloc(plain(vec![]));
        let mk = |vm: &mut Vm| vm.alloc(plain(vec![]));
        let lobby = mk(self);
        let globals = mk(self);
        let traits = mk(self);
        let t = mk(self);
        let f = mk(self);
        let db = mk(self);
        let ti = mk(self);
        let tf = mk(self);
        let ts = mk(self);
        let tb = mk(self);
        let tm = mk(self);
        let shell = mk(self);
        self.sp = Specials {
            lobby,
            globals,
            traits,
            nil,
            true_: t,
            false_: f,
            default_behavior: db,
            t_integer: ti,
            t_float: tf,
            t_string: ts,
            t_block: tb,
            t_method: tm,
            shell,
        };
        self.heap[lobby as usize].slots =
            vec![
                Self::parent_slot("globals", Value::Ref(globals)),
                Self::parent_slot("defaultBehavior", Value::Ref(db)),
                Self::slot("lobby", Value::Ref(lobby)),
            ];
        self.heap[globals as usize].slots = vec![
            Self::slot("nil", Value::Ref(nil)),
            Self::slot("true", Value::Ref(t)),
            Self::slot("false", Value::Ref(f)),
            Self::slot("traits", Value::Ref(traits)),
            Self::slot("shell", Value::Ref(shell)),
        ];
        self.heap[traits as usize].slots = vec![
            Self::slot("integer", Value::Ref(ti)),
            Self::slot("float", Value::Ref(tf)),
            Self::slot("string", Value::Ref(ts)),
            Self::slot("block", Value::Ref(tb)),
            Self::slot("method", Value::Ref(tm)),
        ];
        self.heap[shell as usize].slots = vec![Self::parent_slot("lobby", Value::Ref(lobby))];
        let named: Vec<(ObjId, ObjId, &str)> = vec![
            (globals, lobby, "globals"),
            (nil, globals, "nil"),
            (t, globals, "true"),
            (f, globals, "false"),
            (traits, globals, "traits"),
            (db, lobby, "defaultBehavior"),
            (shell, globals, "shell"),
            (ti, traits, "integer"),
            (tf, traits, "float"),
            (ts, traits, "string"),
            (tb, traits, "block"),
            (tm, traits, "method"),
        ];
        for (o, h, nm) in named {
            self.heap[o as usize].creator = Some((h, nm.into()));
        }
        let world = crate::world::WORLD_SOURCE;
        if let Err(e) = self.run_file(world) {
            panic!("the built-in Self world failed to load: {} {:?}", e.msg, e.trace);
        }
        self.bootstrap_traits =
            self.obj(self.sp.traits).slots.iter().find(|s| &*s.name == "bootstrapPath").and_then(|s| s.value.id());
        if let Err(e) = self.run_file(crate::world::MORPHIC_SOURCE) {
            panic!("the built-in morph library failed to load: {} {:?}", e.msg, e.trace);
        }
        if let Err(e) = self.run_file(crate::world::SELFTEST_SOURCE) {
            panic!("the built-in self-test failed to load: {} {:?}", e.msg, e.trace);
        }
        self.events.clear();
    }

    /// Send a message from the user interface (outside any running evaluation):
    /// errors and stray returns come back as a `SelfError`.
    pub fn ui_send(&mut self, recv: Value, sel: &str, args: Vec<Value>) -> Result<Value, SelfError> {
        let saved = self.step_limit;
        self.steps = 0;
        self.depth = 0;
        self.step_limit = 3_000_000;
        let r = self.send(recv, sel, args);
        self.step_limit = saved;
        match r {
            Ok(v) => Ok(v),
            Err(Unwind::Error(e)) => Err(e),
            Err(Unwind::Return { .. }) => {
                Err(SelfError { msg: "a block returned to a method that has already finished".into(), trace: vec![] })
            }
            Err(Unwind::Restart(_)) => Err(SelfError { msg: "_Restart used outside a method or block".into(), trace: vec![] }),
        }
    }

    /// Run the built-in self-test; returns the summary line.
    pub fn run_self_test(&mut self) -> Result<String, SelfError> {
        let v = self.eval_doit("selfTest run", Value::Ref(self.sp.lobby))?;
        Ok(self.string_of(v).map(|s| s.to_string()).unwrap_or_else(|| self.describe(v)))
    }

    /// Run a whole source file of statements against the lobby.
    pub fn run_file(&mut self, src: &str) -> Result<Value, SelfError> {
        self.eval_doit(src, Value::Ref(self.sp.lobby))
    }

    // ------------------------------------------------------------ entry points

    pub fn eval_doit(&mut self, src: &str, receiver: Value) -> Result<Value, SelfError> {
        let lit = parse_doit(src).map_err(|e| SelfError { msg: format!("Syntax error at {}", e.describe_in(src)), trace: vec![] })?;
        self.steps = 0;
        self.depth = 0;
        let home = self.new_home();
        let holder = receiver.id();
        let frame = Rc::new(Frame {
            paren: false,
            locals: RefCell::new(vec![]),
            self_val: receiver,
            holder,
            parent: None,
            home: home.clone(),
        });
        let res = (|| -> R<Value> {
            let locals = self.make_locals(&lit.slots, &[], &frame)?;
            *frame.locals.borrow_mut() = locals;
            self.eval_code(lit.code.as_deref().unwrap_or(&[]), &frame)
        })();
        home.alive.set(false);
        match res {
            Ok(v) => Ok(v),
            Err(Unwind::Return { home: h, value }) if h == home.id => Ok(value),
            Err(Unwind::Return { .. }) => {
                Err(SelfError { msg: "a block returned to a method that has already finished".into(), trace: vec![] })
            }
            Err(Unwind::Error(e)) => Err(e),
            Err(Unwind::Restart(_)) => Err(SelfError { msg: "_Restart used outside a method or block".into(), trace: vec![] }),
        }
    }

    /// Load a whole Self source file: statements are separated by periods or
    /// by line breaks outside brackets (the original Self file format), and
    /// each statement runs on its own, so one failure doesn't stop the rest.
    pub fn load_source(&mut self, src: &str) -> Result<LoadReport, SelfError> {
        let (lit, starts) =
            parse_file(src).map_err(|e| SelfError { msg: format!("Syntax error at {}", e.describe_in(src)), trace: vec![] })?;
        let receiver = Value::Ref(self.sp.lobby);
        let home = self.new_home();
        let frame = Rc::new(Frame {
            paren: false,
            locals: RefCell::new(vec![]),
            self_val: receiver,
            holder: receiver.id(),
            parent: None,
            home: home.clone(),
        });
        let mut report = LoadReport { statements: 0, failures: vec![], last: None };
        self.steps = 0;
        self.depth = 0;
        match self.make_locals(&lit.slots, &[], &frame) {
            Ok(l) => *frame.locals.borrow_mut() = l,
            Err(Unwind::Error(e)) => report.failures.push(format!("locals: {}", e.msg)),
            Err(_) => {}
        }
        for (i, st) in lit.code.as_deref().unwrap_or(&[]).iter().enumerate() {
            self.steps = 0;
            self.depth = 0;
            report.statements += 1;
            let line = starts.get(i).map(|p| line_col(src, *p).0).unwrap_or(0);
            let r = match st {
                Stmt::Expr(e) => self.eval(e, &frame),
                Stmt::Return(e) => self.eval(e, &frame),
            };
            match r {
                Ok(v) => report.last = Some(v),
                Err(Unwind::Error(e)) => report.failures.push(format!("line {line}: {}", e.msg)),
                Err(Unwind::Return { value, .. }) => {
                    report.last = Some(value);
                    break;
                }
                Err(Unwind::Restart(_)) => report.failures.push(format!("line {line}: _Restart used outside a method or block")),
            }
        }
        home.alive.set(false);
        Ok(report)
    }

    /// Follow slot names from the lobby (the first may be inherited, the
    /// rest must be the object's own), creating stand-ins that are missing.
    fn resolve_bootstrap_path(&mut self, names: &str) -> Value {
        let mut cur = self.sp.lobby;
        for (i, n) in names.split('\u{1}').filter(|n| !n.is_empty()).enumerate() {
            let found = if i == 0 {
                let mut f = vec![];
                let mut path = vec![];
                self.lookup_value_into(Value::Ref(cur), n, 0, &mut path, &mut f);
                dedupe(&mut f);
                f.first().map(|f| self.heap[f.holder as usize].slots[f.index].value)
            } else {
                self.obj(cur).slots.iter().find(|s| &*s.name == n).map(|s| s.value)
            };
            cur = match found {
                Some(Value::Ref(id)) => id,
                Some(other) => return other,
                None => {
                    let stand_in = self.alloc(Obj {
                        slots: vec![Self::parent_slot(STAND_IN, Value::Ref(self.sp.lobby))],
                        kind: Kind::Plain,
                        creator: None,
                        comment: Some("Created by a bootstrap path; not defined yet.".into()),
                    });
                    let name: Rc<str> = n.into();
                    self.cache.clear();
                    self.heap[cur as usize].slots.push(Self::slot(n, Value::Ref(stand_in)));
                    self.heap[stand_in as usize].creator = Some((cur, name));
                    stand_in
                }
            };
        }
        Value::Ref(cur)
    }

    /// Load `<dir>/<name>.self` next to the file being loaded (as the original
    /// world builder does). Failures are written to the transcript.
    fn load_subpart(&mut self, name: &str, dir: &str) -> Result<Value, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, dir);
            Err("this browser version can't read other files; load each file yourself, or use the desktop app".into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let base = self.source_dir.clone().ok_or("no folder known for this file; load it from a file on disk")?;
            if self.load_nesting > 30 {
                return Err("files read each other too deeply".into());
            }
            let file = format!("{name}.self");
            let candidates = [
                base.join(&file),
                base.parent().map(|p| p.join(dir).join(&file)).unwrap_or_default(),
                base.join(dir).join(&file),
            ];
            let path = candidates.iter().find(|p| p.is_file()).ok_or(format!("can't find {dir}/{file}"))?.clone();
            let src = std::fs::read_to_string(&path).map_err(|e| format!("can't read {}: {e}", path.display()))?;
            let saved = (self.steps, self.depth, self.source_dir.clone());
            self.source_dir = path.parent().map(|p| p.to_path_buf());
            self.load_nesting += 1;
            let r = self.load_source(&src);
            self.load_nesting -= 1;
            self.steps = saved.0;
            self.depth = saved.1;
            self.source_dir = saved.2;
            match r {
                Ok(rep) => {
                    let mut msg = format!("read {dir}/{file}: {} statements, {} failed\n", rep.statements, rep.failures.len());
                    for f in rep.failures.iter().take(10) {
                        msg.push_str(&format!("    {f}\n"));
                    }
                    self.events.push(Event::Print(msg));
                    Ok(rep.last.unwrap_or(self.nil()))
                }
                Err(e) => Err(format!("{dir}/{file}: {}", e.msg)),
            }
        }
    }

    fn new_home(&mut self) -> Rc<Home> {
        self.next_home += 1;
        Rc::new(Home { id: self.next_home, alive: Cell::new(true) })
    }

    // ------------------------------------------------------------ evaluation

    #[inline(never)]
    fn make_locals(&mut self, slots: &[SlotDesc], args: &[Value], frame: &Rc<Frame>) -> R<Vec<Local>> {
        let mut out = Vec::with_capacity(slots.len());
        let mut ai = 0;
        for s in slots {
            let value = match s.kind {
                SlotKind::Argument => {
                    let v = args.get(ai).copied().unwrap_or(self.nil());
                    ai += 1;
                    v
                }
                _ => match &s.init {
                    Some(e) => self.eval(e, frame)?,
                    None => self.nil(),
                },
            };
            out.push(Local { name: s.name.clone(), value, assignable: s.kind != SlotKind::Constant });
        }
        Ok(out)
    }

    /// Run a method or block body, starting again whenever it sends `_Restart`.
    fn run_activation(&mut self, code: &[Stmt], frame: &Rc<Frame>) -> R<Value> {
        let me = Rc::as_ptr(frame) as usize;
        loop {
            match self.eval_code(code, frame) {
                Err(Unwind::Restart(f)) if f == me => {
                    self.steps += 1;
                    if self.steps > self.step_limit {
                        return err("stopped: this loop ran too long — is it infinite?");
                    }
                }
                other => return other,
            }
        }
    }

    fn eval_code(&mut self, code: &[Stmt], frame: &Rc<Frame>) -> R<Value> {
        let mut last = self.nil();
        for st in code {
            match st {
                Stmt::Expr(e) => last = self.eval(e, frame)?,
                Stmt::Return(e) => {
                    let v = self.eval(e, frame)?;
                    if !frame.home.alive.get() {
                        return err("non-local return from a block whose method has already returned");
                    }
                    return Err(Unwind::Return { home: frame.home.id, value: v });
                }
            }
        }
        Ok(last)
    }

    fn eval(&mut self, e: &Expr, frame: &Rc<Frame>) -> R<Value> {
        match e {
            Expr::Int(v) => Ok(Value::Int(*v)),
            Expr::Float(v) => Ok(Value::Float(*v)),
            Expr::Str(s) => Ok(Value::Ref(self.alloc(Obj {
                slots: vec![],
                kind: Kind::Str(s.clone()),
                creator: None,
                comment: None,
            }))),
            Expr::SelfRef => Ok(frame.self_val),
            Expr::Object(lit) => self.make_object(lit, frame),
            Expr::Method(lit) => self.make_method(lit, frame),
            Expr::Paren(lit) => self.eval_paren(lit, frame),
            Expr::Block(lit) => Ok(Value::Ref(self.alloc(Obj {
                slots: vec![],
                kind: Kind::Block { lit: lit.clone(), env: frame.clone() },
                creator: None,
                comment: None,
            }))),
            Expr::Send { recv, sel, args, resend } => {
                let rv = match recv {
                    Some(r) => Some(self.eval(r, frame)?),
                    None => None,
                };
                let mut av = Vec::with_capacity(args.len());
                for a in args {
                    av.push(self.eval(a, frame)?);
                }
                if let Some(target) = resend {
                    return self.resend(frame, target, sel, av);
                }
                match rv {
                    Some(r) => self.send(r, sel, av),
                    None => self.implicit_send(frame, sel, av),
                }
            }
        }
    }

    #[inline(never)]
    fn eval_paren(&mut self, lit: &Rc<ObjLit>, frame: &Rc<Frame>) -> R<Value> {
        if lit.slots.is_empty() {
            return self.eval_code(lit.code.as_deref().unwrap_or(&[]), frame);
        }
        let sub = Rc::new(Frame {
            paren: true,
            locals: RefCell::new(vec![]),
            self_val: frame.self_val,
            holder: frame.holder,
            parent: Some(frame.clone()),
            home: frame.home.clone(),
        });
        let locals = self.make_locals(&lit.slots, &[], frame)?;
        *sub.locals.borrow_mut() = locals;
        self.eval_code(lit.code.as_deref().unwrap_or(&[]), &sub)
    }

    #[inline(never)]
    fn make_object(&mut self, lit: &Rc<ObjLit>, frame: &Rc<Frame>) -> R<Value> {
        let id = self.alloc(plain(vec![]));
        let mut slots = Vec::with_capacity(lit.slots.len());
        for s in &lit.slots {
            let value = match &s.init {
                Some(e) => self.eval(e, frame)?,
                None => self.nil(),
            };
            slots.push(Slot {
                name: s.name.clone(),
                value,
                assignable: s.kind == SlotKind::Assignable,
                parent: s.parent,
                arg: s.kind == SlotKind::Argument,
                category: s.category.clone(),
            });
        }
        for s in &slots {
            self.set_creator_if_none(s.value, id, &s.name);
        }
        let o = &mut self.heap[id as usize];
        o.slots = slots;
        o.comment = lit.comment.clone();
        Ok(Value::Ref(id))
    }

    #[inline(never)]
    fn make_method(&mut self, lit: &Rc<ObjLit>, frame: &Rc<Frame>) -> R<Value> {
        let mut slots = Vec::with_capacity(lit.slots.len());
        for s in &lit.slots {
            let value = match (&s.kind, &s.init) {
                (SlotKind::Argument, _) => self.nil(),
                (_, Some(e)) => self.eval(e, frame)?,
                (_, None) => self.nil(),
            };
            slots.push(Slot {
                name: s.name.clone(),
                value,
                assignable: s.kind == SlotKind::Assignable,
                parent: s.parent,
                arg: s.kind == SlotKind::Argument,
                category: s.category.clone(),
            });
        }
        Ok(Value::Ref(self.alloc(Obj {
            slots,
            kind: Kind::Method(lit.clone()),
            creator: None,
            comment: lit.comment.clone(),
        })))
    }

    fn implicit_send(&mut self, frame: &Rc<Frame>, sel: &Rc<str>, args: Vec<Value>) -> R<Value> {
        if &**sel == "_Restart" {
            // restart the innermost method or block activation
            let mut f = frame;
            while f.paren {
                match f.parent.as_ref() {
                    Some(p) => f = p,
                    None => break,
                }
            }
            return Err(Unwind::Restart(Rc::as_ptr(f) as usize));
        }
        if sel.starts_with('_') {
            return self.primitive(frame.self_val, sel, args);
        }
        // search the activation chain (locals and arguments) first
        let mut f: Option<&Rc<Frame>> = Some(frame);
        while let Some(fr) = f {
            {
                let mut locals = fr.locals.borrow_mut();
                for l in locals.iter_mut() {
                    if args.is_empty() && *l.name == **sel {
                        return Ok(l.value);
                    }
                    if args.len() == 1 && l.assignable && is_setter_of(sel, &l.name) {
                        l.value = args[0];
                        return Ok(fr.self_val);
                    }
                }
            }
            f = fr.parent.as_ref();
        }
        self.send(frame.self_val, sel, args)
    }

    #[inline(never)]
    fn resend(&mut self, frame: &Rc<Frame>, target: &Rc<str>, sel: &Rc<str>, args: Vec<Value>) -> R<Value> {
        let holder = match frame.holder {
            Some(h) => h,
            None => return err("resend used outside of a method"),
        };
        let mut found = vec![];
        if &**target == "resend" {
            let parents: Vec<Value> = self.obj(holder).slots.iter().filter(|s| s.parent).map(|s| s.value).collect();
            let mut path = vec![holder];
            for p in parents {
                self.lookup_value_into(p, sel, args.len(), &mut path, &mut found);
            }
            if let Some(ip) = self.implicit_parent(Value::Ref(holder)) {
                self.lookup_in(ip, sel, args.len(), &mut path, &mut found);
            }
        } else {
            let del = self.obj(holder).slots.iter().find(|s| *s.name == **target).map(|s| s.value);
            match del {
                Some(d) => {
                    let mut path = vec![];
                    self.lookup_value_into(d, sel, args.len(), &mut path, &mut found);
                }
                None => return err(format!("directed resend: no parent slot named '{target}'")),
            }
        }
        dedupe(&mut found);
        match found.len() {
            0 => err(format!("resend of '{sel}' found no inherited slot")),
            1 => self.eval_slot(frame.self_val, found[0], sel, args),
            _ => err(format!("resend of '{sel}' is ambiguous")),
        }
    }

    /// The parent that values without real slots (numbers, strings, blocks) inherit from.
    pub fn implicit_parent(&self, v: Value) -> Option<ObjId> {
        match v {
            Value::Int(_) => Some(self.sp.t_integer),
            Value::Float(_) => Some(self.sp.t_float),
            Value::Ref(id) => match &self.obj(id).kind {
                Kind::Str(_) => Some(self.sp.t_string),
                Kind::Block { .. } => Some(self.sp.t_block),
                Kind::Method(_) => Some(self.sp.t_method),
                Kind::Big(_) => Some(self.sp.t_integer),
                _ => None,
            },
        }
    }

    fn lookup_value_into(&self, v: Value, sel: &str, nargs: usize, path: &mut Vec<ObjId>, out: &mut Vec<Found>) {
        match v {
            Value::Ref(id) => self.lookup_in(id, sel, nargs, path, out),
            _ => {
                if let Some(p) = self.implicit_parent(v) {
                    self.lookup_in(p, sel, nargs, path, out)
                }
            }
        }
    }

    fn lookup_in(&self, id: ObjId, sel: &str, nargs: usize, path: &mut Vec<ObjId>, out: &mut Vec<Found>) {
        if path.contains(&id) {
            return;
        }
        let o = self.obj(id);
        let before = out.len();
        for (i, s) in o.slots.iter().enumerate() {
            if *s.name == *sel {
                out.push(Found { holder: id, index: i, assign: false });
            } else if nargs == 1 && s.assignable && is_setter_of(sel, &s.name) {
                out.push(Found { holder: id, index: i, assign: true });
            }
        }
        if out.len() > before {
            return;
        }
        path.push(id);
        for s in o.slots.iter() {
            if s.parent {
                self.lookup_value_into(s.value, sel, nargs, path, out);
            }
        }
        if let Some(ip) = self.implicit_parent(Value::Ref(id)) {
            self.lookup_in(ip, sel, nargs, path, out);
        }
        path.pop();
    }

    pub fn send(&mut self, recv: Value, sel: &str, args: Vec<Value>) -> R<Value> {
        self.steps += 1;
        if self.steps > self.step_limit {
            return err(format!(
                "stopped after {} million message sends — is something looping forever?",
                self.step_limit / 1_000_000
            ));
        }
        if sel.starts_with('_') {
            return self.primitive(recv, sel, args);
        }
        if let Value::Ref(id) = recv {
            if let Kind::Block { .. } = self.obj(id).kind {
                if is_value_selector(sel, args.len()) {
                    return self.call_block(id, args);
                }
            }
        }
        let start = match recv {
            Value::Ref(id) => id,
            _ => self.implicit_parent(recv).unwrap_or(self.sp.nil),
        };
        let key = (start, sel_hash(sel, args.len()));
        if let Some(f) = self.cache.get(&key).copied() {
            let s = &self.heap[f.holder as usize].slots;
            if f.index < s.len() && (if f.assign { is_setter_of(sel, &s[f.index].name) } else { *s[f.index].name == *sel }) {
                return self.eval_slot(recv, f, sel, args);
            }
        }
        let mut found = vec![];
        let mut path = vec![];
        self.lookup_value_into(recv, sel, args.len(), &mut path, &mut found);
        dedupe(&mut found);
        match found.len() {
            1 => {
                if self.cache.len() > 100_000 {
                    self.cache.clear();
                }
                self.cache.insert(key, found[0]);
                self.eval_slot(recv, found[0], sel, args)
            }
            0 => self.not_understood(recv, sel),
            _ => err(format!("'{sel}' sent to {} is ambiguous (found in several parents)", self.describe(recv))),
        }
    }

    #[cold]
    #[inline(never)]
    fn not_understood(&self, recv: Value, sel: &str) -> R<Value> {
        let d = self.describe(recv);
        err(format!("{d} does not understand '{sel}'"))
    }

    fn eval_slot(&mut self, recv: Value, f: Found, sel: &str, args: Vec<Value>) -> R<Value> {
        if f.assign {
            let v = args[0];
            let name = self.heap[f.holder as usize].slots[f.index].name.clone();
            if self.heap[f.holder as usize].slots[f.index].parent {
                self.cache.clear();
            }
            self.heap[f.holder as usize].slots[f.index].value = v;
            self.set_creator_if_none(v, f.holder, &name);
            return Ok(recv);
        }
        let v = self.heap[f.holder as usize].slots[f.index].value;
        if let Value::Ref(m) = v {
            if let Kind::Method(lit) = &self.obj(m).kind {
                let lit = lit.clone();
                return self.invoke_method(recv, f.holder, m, lit, sel, args);
            }
        }
        Ok(v)
    }

    fn invoke_method(
        &mut self,
        recv: Value,
        holder: ObjId,
        method: ObjId,
        lit: Rc<ObjLit>,
        sel: &str,
        args: Vec<Value>,
    ) -> R<Value> {
        let nargs = self.obj(method).slots.iter().filter(|s| s.arg).count();
        if nargs != args.len() {
            return err(format!("'{sel}' expects {nargs} argument(s) but got {}", args.len()));
        }
        if self.depth >= MAX_DEPTH {
            return err(format!("stack overflow: more than {MAX_DEPTH} nested calls"));
        }
        let mut ai = 0;
        let locals: Vec<Local> = self
            .obj(method)
            .slots
            .iter()
            .map(|s| {
                let value = if s.arg {
                    let v = args[ai];
                    ai += 1;
                    v
                } else {
                    s.value
                };
                Local { name: s.name.clone(), value, assignable: s.assignable || s.arg }
            })
            .collect();
        let home = self.new_home();
        let frame = Rc::new(Frame {
            paren: false,
            locals: RefCell::new(locals),
            self_val: recv,
            holder: Some(holder),
            parent: None,
            home: home.clone(),
        });
        self.depth += 1;
        let code = lit.code.as_deref().unwrap_or(&[]);
        let r = grow_stack(|| self.run_activation(code, &frame));
        self.depth -= 1;
        home.alive.set(false);
        match r {
            Err(Unwind::Return { home: h, value }) if h == home.id => Ok(value),
            Err(Unwind::Error(e)) => Err(self.add_trace(e, recv, sel)),
            other => other,
        }
    }

    #[cold]
    #[inline(never)]
    fn add_trace(&self, mut e: SelfError, recv: Value, sel: &str) -> Unwind {
        if e.trace.len() < 12 {
            let d = self.describe(recv);
            e.trace.push(format!("{d}  ›  {sel}"));
        }
        Unwind::Error(e)
    }

    pub fn call_block(&mut self, block: ObjId, args: Vec<Value>) -> R<Value> {
        let (lit, env) = match &self.obj(block).kind {
            Kind::Block { lit, env } => (lit.clone(), env.clone()),
            _ => return err("not a block"),
        };
        let nargs = lit.slots.iter().filter(|s| s.kind == SlotKind::Argument).count();
        // like Self's enumerators, extra arguments (an index or key) may be ignored
        if nargs > args.len() {
            return err(format!("this block takes {nargs} argument(s) but was given {}", args.len()));
        }
        let mut args = args;
        args.truncate(nargs);
        if self.depth >= MAX_DEPTH {
            return err(format!("stack overflow: more than {MAX_DEPTH} nested calls"));
        }
        let frame = Rc::new(Frame {
            paren: false,
            locals: RefCell::new(vec![]),
            self_val: env.self_val,
            holder: env.holder,
            parent: Some(env.clone()),
            home: env.home.clone(),
        });
        let locals = self.make_locals(&lit.slots, &args, &env)?;
        *frame.locals.borrow_mut() = locals;
        self.depth += 1;
        let code = lit.code.as_deref().unwrap_or(&[]);
        let r = grow_stack(|| self.run_activation(code, &frame));
        self.depth -= 1;
        r
    }

    /// Call a block with as many of the given arguments as it accepts.
    fn call_block_lenient(&mut self, v: Value, args: Vec<Value>) -> R<Value> {
        if let Value::Ref(id) = v {
            if let Kind::Block { lit, .. } = &self.obj(id).kind {
                let n = lit.slots.iter().filter(|s| s.kind == SlotKind::Argument).count();
                let a: Vec<Value> = args.into_iter().take(n).collect();
                return self.call_block(id, a);
            }
        }
        self.send(v, "value", vec![])
    }

    pub fn is_true(&self, v: Value) -> R<bool> {
        match v {
            Value::Ref(id) if id == self.sp.true_ => Ok(true),
            Value::Ref(id) if id == self.sp.false_ => Ok(false),
            _ => err(format!("expected true or false, got {}", self.describe(v))),
        }
    }

    // ------------------------------------------------------------ reflection helpers

    pub fn path_name(&self, id: ObjId) -> Option<String> {
        let mut parts: Vec<String> = vec![];
        let mut cur = id;
        for _ in 0..8 {
            if cur == self.sp.lobby || cur == self.sp.globals {
                break;
            }
            let (h, n) = self.obj(cur).creator.clone()?;
            if (h as usize) >= self.heap.len() {
                return None;
            }
            let holds = self.obj(h).slots.iter().any(|s| *s.name == *n && s.value == Value::Ref(cur));
            if !holds {
                return None;
            }
            parts.push(n.to_string());
            cur = h;
        }
        if cur != self.sp.lobby && cur != self.sp.globals {
            return None;
        }
        if parts.is_empty() {
            return Some(if id == self.sp.globals { "globals".into() } else { "lobby".into() });
        }
        parts.reverse();
        Some(parts.join(" "))
    }

    /// A short human name such as `traits integer`, `a point`, `'hi'` or `3`.
    #[inline(never)]
    pub fn describe(&self, v: Value) -> String {
        match v {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => fmt_float(f),
            Value::Ref(id) => {
                let o = self.obj(id);
                match &o.kind {
                    Kind::Str(s) => quote(s),
                    Kind::Block { .. } => "a block".into(),
                    Kind::Method(_) => "a method".into(),
                    Kind::Free => "a freed object".into(),
                    Kind::Big(b) => b.to_string(),
                    Kind::Vector(_) | Kind::Plain => {
                        if let Some(p) = self.path_name(id) {
                            return p;
                        }
                        for s in &o.slots {
                            if s.parent {
                                if let Value::Ref(pid) = s.value {
                                    if let Some(pn) = self.path_name(pid) {
                                        let base = pn.strip_prefix("traits ").unwrap_or(&pn).to_string();
                                        if base == "lobby" || base == "clonable" || base == "oddball" {
                                            continue;
                                        }
                                        let art = if base.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
                                        return format!("{art} {base}");
                                    }
                                }
                            }
                        }
                        if matches!(o.kind, Kind::Vector(_)) {
                            "a vector".into()
                        } else {
                            "an object".into()
                        }
                    }
                }
            }
        }
    }

    /// `printString` sent from the outside; falls back to `describe`.
    pub fn print_string(&mut self, v: Value) -> String {
        let saved = (self.steps, self.depth, self.step_limit);
        self.steps = 0;
        self.step_limit = 200_000;
        let r = self.send(v, "printString", vec![]);
        self.steps = saved.0;
        self.depth = saved.1;
        self.step_limit = saved.2;
        match r {
            Ok(s) => match self.string_of(s) {
                Some(s) => s.to_string(),
                None => self.describe(v),
            },
            Err(_) => self.describe(v),
        }
    }

    pub fn kind_label(&self, v: Value) -> &'static str {
        match v {
            Value::Int(_) => "integer",
            Value::Float(_) => "float",
            Value::Ref(id) => match &self.obj(id).kind {
                Kind::Str(_) => "string",
                Kind::Block { .. } => "block",
                Kind::Method(_) => "method",
                Kind::Vector(_) => "vector",
                Kind::Free => "freed",
                Kind::Big(_) => "integer",
                Kind::Plain => "object",
            },
        }
    }

    pub fn is_method(&self, v: Value) -> bool {
        matches!(v, Value::Ref(m) if matches!(self.obj(m).kind, Kind::Method(_)))
    }

    pub fn slots_of(&self, v: Value) -> Vec<SlotView> {
        let mut out = vec![];
        if let Value::Ref(id) = v {
            for s in &self.obj(id).slots {
                out.push(SlotView {
                    name: s.name.to_string(),
                    value: s.value,
                    assignable: s.assignable,
                    parent: s.parent,
                    arg: s.arg,
                    is_method: self.is_method(s.value),
                    category: s.category.to_string(),
                    virtual_slot: false,
                });
            }
        }
        if let Some(p) = self.implicit_parent(v) {
            out.insert(
                0,
                SlotView {
                    name: "parent".into(),
                    value: Value::Ref(p),
                    assignable: false,
                    parent: true,
                    arg: false,
                    is_method: false,
                    category: "".into(),
                    virtual_slot: true,
                },
            );
        }
        out
    }

    pub fn vector_items(&self, v: Value) -> Option<Vec<Value>> {
        match v {
            Value::Ref(id) => match &self.obj(id).kind {
                Kind::Vector(items) => Some(items.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn source_of(&self, v: Value) -> Option<String> {
        match v {
            Value::Ref(id) => match &self.obj(id).kind {
                Kind::Method(lit) => Some(lit.source.to_string()),
                Kind::Block { lit, .. } => Some(lit.source.to_string()),
                _ => None,
            },
            _ => None,
        }
    }

    /// The full slot declaration of a method, e.g. `at: i Put: x = ( ... )`,
    /// so it can be edited and accepted again.
    pub fn method_declaration(&self, name: &str, v: Value) -> Option<String> {
        let id = v.id()?;
        let lit = match &self.obj(id).kind {
            Kind::Method(lit) => lit.clone(),
            _ => return None,
        };
        let args: Vec<String> = lit.slots.iter().take(lit.inline_args).map(|s| s.name.to_string()).collect();
        let header = if args.is_empty() {
            name.to_string()
        } else if !name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            format!("{name} {}", args[0])
        } else {
            name.split_inclusive(':').zip(args.iter()).map(|(k, a)| format!("{k} {a}")).collect::<Vec<_>>().join(" ")
        };
        Some(format!("{header} = {}", lit.source))
    }

    pub fn comment_of(&self, v: Value) -> Option<String> {
        v.id().and_then(|id| self.obj(id).comment.as_ref().map(|c| c.to_string()))
    }

    pub fn is_alive(&self, v: Value) -> bool {
        match v {
            Value::Ref(id) => (id as usize) < self.heap.len() && !matches!(self.obj(id).kind, Kind::Free),
            _ => true,
        }
    }

    /// Parse a slot list such as `x <- 3. area = ( x * y )` and add it to `target`.
    pub fn add_slots_from_source(&mut self, target: Value, src: &str) -> Result<(), String> {
        let id = target.id().ok_or("numbers cannot have slots added")?;
        let descs = parse_slots(src).map_err(|e| format!("Syntax error at {}", e.describe_in(src)))?;
        let lit = Rc::new(ObjLit { slots: descs, code: None, source: src.into(), comment: None, inline_args: 0 });
        let home = self.new_home();
        let frame = Rc::new(Frame {
            paren: false,
            locals: RefCell::new(vec![]),
            self_val: Value::Ref(self.sp.lobby),
            holder: Some(self.sp.lobby),
            parent: None,
            home,
        });
        self.steps = 0;
        self.depth = 0;
        let obj = match self.make_object(&lit, &frame) {
            Ok(v) => v,
            Err(Unwind::Error(e)) => return Err(e.msg),
            Err(_) => return Err("unexpected return".into()),
        };
        self.add_slots(id, obj.id().unwrap(), true);
        Ok(())
    }

    pub fn remove_slot(&mut self, target: Value, name: &str) -> bool {
        self.cache.clear();
        if let Some(id) = target.id() {
            let o = &mut self.heap[id as usize];
            let before = o.slots.len();
            o.slots.retain(|s| *s.name != *name);
            return o.slots.len() != before;
        }
        false
    }

    pub fn set_slot_value(&mut self, target: Value, name: &str, v: Value) -> bool {
        self.cache.clear();
        if let Some(id) = target.id() {
            if let Some(s) = self.heap[id as usize].slots.iter_mut().find(|s| *s.name == *name) {
                s.value = v;
                let n = s.name.clone();
                self.set_creator_if_none(v, id, &n);
                return true;
            }
        }
        false
    }

    fn add_slots(&mut self, target: ObjId, from: ObjId, replace: bool) {
        self.cache.clear();
        let new_slots = self.obj(from).slots.clone();
        for s in new_slots {
            // the temporary parent of objects made by bootstrap paths is never copied
            if &*s.name == STAND_IN && from != target {
                continue;
            }
            // objects created by the literal now belong to the target
            if let Value::Ref(v) = s.value {
                if let Some((h, n)) = &self.obj(v).creator {
                    if *h == from && **n == *s.name {
                        self.heap[v as usize].creator = Some((target, s.name.clone()));
                    }
                }
            }
            let t = &mut self.heap[target as usize];
            if let Some(existing) = t.slots.iter_mut().find(|x| x.name == s.name) {
                if replace {
                    *existing = s;
                }
            } else {
                t.slots.push(s);
            }
        }
        if self.obj(target).comment.is_none() {
            let c = self.obj(from).comment.clone();
            self.heap[target as usize].comment = c;
        }
        // once a stand-in gets a real parent, its temporary one goes
        let t = &mut self.heap[target as usize];
        if t.slots.iter().any(|x| x.parent && &*x.name != STAND_IN) {
            t.slots.retain(|x| &*x.name != STAND_IN);
        }
    }

    /// Named objects that have a slot whose name contains `text`.
    pub fn implementors(&self, text: &str) -> Vec<(Value, String, String)> {
        let needle = text.to_lowercase();
        let mut out = vec![];
        for (i, o) in self.heap.iter().enumerate() {
            if !matches!(o.kind, Kind::Plain | Kind::Vector(_)) {
                continue;
            }
            let mut owner: Option<String> = None;
            for s in &o.slots {
                if s.name.to_lowercase().contains(&needle) {
                    if owner.is_none() {
                        owner = self.path_name(i as ObjId);
                        if owner.is_none() {
                            break;
                        }
                    }
                    out.push((Value::Ref(i as ObjId), owner.clone().unwrap(), s.name.to_string()));
                }
            }
        }
        out.sort_by(|a, b| a.2.len().cmp(&b.2.len()).then(a.2.cmp(&b.2)));
        out.truncate(300);
        out
    }

    pub fn object_count(&self) -> usize {
        self.heap.len() - self.free.len()
    }

    // ------------------------------------------------------------ garbage collection

    pub fn needs_gc(&self) -> bool {
        self.allocated_since_gc > 200_000
    }

    /// Mark-and-sweep, run only between evaluations so no Rust frames hold values.
    pub fn collect_garbage(&mut self, extra_roots: &[Value]) -> usize {
        let mut marked = vec![false; self.heap.len()];
        let mut stack: Vec<ObjId> = vec![
            self.sp.lobby,
            self.sp.globals,
            self.sp.traits,
            self.sp.nil,
            self.sp.true_,
            self.sp.false_,
            self.sp.default_behavior,
            self.sp.t_integer,
            self.sp.t_float,
            self.sp.t_string,
            self.sp.t_block,
            self.sp.t_method,
            self.sp.shell,
        ];
        stack.extend(extra_roots.iter().filter_map(|v| v.id()));
        stack.extend(self.bootstrap_traits);
        let mut seen_frames: HashSet<*const Frame> = HashSet::new();
        while let Some(id) = stack.pop() {
            let i = id as usize;
            if i >= marked.len() || marked[i] {
                continue;
            }
            marked[i] = true;
            let o = &self.heap[i];
            for s in &o.slots {
                if let Value::Ref(r) = s.value {
                    stack.push(r);
                }
            }
            if let Some((h, _)) = &o.creator {
                stack.push(*h);
            }
            match &o.kind {
                Kind::Vector(items) => stack.extend(items.iter().filter_map(|v| v.id())),
                Kind::Block { env, .. } => {
                    let mut f = Some(env.clone());
                    while let Some(fr) = f {
                        if !seen_frames.insert(Rc::as_ptr(&fr)) {
                            break;
                        }
                        for l in fr.locals.borrow().iter() {
                            if let Value::Ref(r) = l.value {
                                stack.push(r);
                            }
                        }
                        if let Value::Ref(r) = fr.self_val {
                            stack.push(r);
                        }
                        if let Some(h) = fr.holder {
                            stack.push(h);
                        }
                        f = fr.parent.clone();
                    }
                }
                _ => {}
            }
        }
        self.cache.clear();
        let mut freed = 0;
        for (i, m) in marked.iter().enumerate() {
            if !m && !matches!(self.heap[i].kind, Kind::Free) {
                self.heap[i] = Obj { slots: vec![], kind: Kind::Free, creator: None, comment: None };
                self.free.push(i as ObjId);
                freed += 1;
            }
        }
        self.allocated_since_gc = 0;
        freed
    }

    // ------------------------------------------------------------ primitives

    #[inline(never)]
    fn primitive(&mut self, recv: Value, sel: &str, mut args: Vec<Value>) -> R<Value> {
        let (base, fail) = if sel.ends_with("IfFail:") && sel.len() > 8 {
            let b = &sel[..sel.len() - 7];
            let f = args.pop();
            (b.to_string(), f)
        } else {
            (sel.to_string(), None)
        };
        match self.prim(recv, &base, &args) {
            Ok(v) => Ok(v),
            Err(PrimErr::Unwind(u)) => Err(u),
            Err(PrimErr::Fail(code)) => match fail {
                Some(fb) => {
                    let es = self.new_string(&code);
                    let ps = self.new_string(&base);
                    self.call_block_lenient(fb, vec![es, ps])
                }
                None => err(format!("primitive {base} failed: {code}")),
            },
        }
    }

    fn rand(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    fn too_long() -> PrimErr {
        PrimErr::Unwind(Unwind::Error(SelfError {
            msg: "stopped: this loop ran too long — is it infinite?".into(),
            trace: vec![],
        }))
    }

    #[inline(never)]
    fn prim(&mut self, recv: Value, sel: &str, a: &[Value]) -> Result<Value, PrimErr> {
        use PrimErr::Fail;
        let need = |n: usize| -> Result<(), PrimErr> {
            if a.len() == n {
                Ok(())
            } else {
                Err(Fail("wrongNumberOfArgumentsError".into()))
            }
        };
        let int = |v: Value| -> Result<i64, PrimErr> {
            match v {
                Value::Int(i) => Ok(i),
                _ => Err(Fail("badTypeError".into())),
            }
        };
        let heap = &self.heap;
        let num = |v: Value| -> Result<f64, PrimErr> {
            match v {
                Value::Int(i) => Ok(i as f64),
                Value::Float(f) => Ok(f),
                Value::Ref(id) => match &heap[id as usize].kind {
                    Kind::Big(b) => Ok(b.to_f64().unwrap_or(f64::INFINITY)),
                    _ => Err(Fail("badTypeError".into())),
                },
            }
        };
        let overflow = || Fail("overflowError".into());
        macro_rules! flt2 {
            ($f:expr) => {{
                need(1)?;
                let x = num(recv)?;
                let y = num(a[0])?;
                $f(x, y)
            }};
        }
        match sel {
            // ---- identity & objects
            "_Eq:" => {
                need(1)?;
                Ok(self.boolean(recv.same(a[0])))
            }
            "_IdentityHash" => Ok(Value::Int(match recv {
                Value::Int(i) => i,
                Value::Float(f) => f.to_bits() as i64,
                Value::Ref(id) => id as i64,
            })),
            "_Clone" => match recv {
                Value::Ref(id) => {
                    let o = self.obj(id);
                    let kind = match &o.kind {
                        Kind::Plain => Kind::Plain,
                        Kind::Vector(v) => Kind::Vector(v.clone()),
                        Kind::Str(s) => Kind::Str(s.clone()),
                        Kind::Method(l) => Kind::Method(l.clone()),
                        Kind::Block { lit, env } => Kind::Block { lit: lit.clone(), env: env.clone() },
                        Kind::Big(b) => Kind::Big(b.clone()),
                        Kind::Free => return Err(Fail("deadObjectError".into())),
                    };
                    let slots = o.slots.clone();
                    let comment = o.comment.clone();
                    Ok(Value::Ref(self.alloc(Obj { slots, kind, creator: None, comment })))
                }
                v => Ok(v),
            },
            "_AddSlots:" | "_AddSlotsIfAbsent:" | "_Define:" => {
                need(1)?;
                let t = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                let f = a[0].id().ok_or_else(|| Fail("badTypeError".into()))?;
                if sel == "_Define:" {
                    self.cache.clear();
                    self.heap[t as usize].slots.clear();
                }
                self.add_slots(t, f, sel != "_AddSlotsIfAbsent:");
                Ok(recv)
            }
            "_RemoveSlot:" => {
                need(1)?;
                let n = self.str_arg(a[0])?;
                if self.remove_slot(recv, &n) {
                    Ok(recv)
                } else {
                    Err(Fail("slotNameError".into()))
                }
            }
            "_ObjectName" => {
                let d = self.describe(recv);
                Ok(self.new_string(&d))
            }
            "_KindName" => {
                let d = self.kind_label(recv);
                Ok(self.new_string(d))
            }
            "_SlotNames" => {
                let names: Vec<String> = match recv {
                    Value::Ref(id) => self.obj(id).slots.iter().map(|s| s.name.to_string()).collect(),
                    _ => vec![],
                };
                let items: Vec<Value> = names.iter().map(|n| self.new_string(n)).collect();
                Ok(self.new_vector(items))
            }
            "_SlotValue:" => {
                need(1)?;
                let n = self.str_arg(a[0])?;
                match recv {
                    Value::Ref(id) => self
                        .obj(id)
                        .slots
                        .iter()
                        .find(|s| *s.name == *n)
                        .map(|s| s.value)
                        .ok_or_else(|| Fail("slotNameError".into())),
                    _ => Err(Fail("slotNameError".into())),
                }
            }
            "_SlotLookup:" => {
                // find a slot through parents without running it
                need(1)?;
                let n = self.str_arg(a[0])?;
                let mut found = vec![];
                let mut path = vec![];
                self.lookup_value_into(recv, &n, 0, &mut path, &mut found);
                dedupe(&mut found);
                match found.first() {
                    Some(f) if found.len() == 1 => Ok(self.heap[f.holder as usize].slots[f.index].value),
                    _ => Err(Fail("slotNameError".into())),
                }
            }
            "_DefineSlot:Value:" => {
                need(2)?;
                let n = self.str_arg(a[0])?;
                let t = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                self.cache.clear();
                let name: Rc<str> = n.clone();
                let slot = Slot { name: name.clone(), value: a[1], assignable: false, parent: false, arg: false, category: "".into() };
                let o = &mut self.heap[t as usize];
                match o.slots.iter_mut().find(|x| x.name == name) {
                    Some(x) => *x = slot,
                    None => o.slots.push(slot),
                }
                self.set_creator_if_none(a[1], t, &name);
                Ok(recv)
            }
            "_BootstrapStub" => {
                // a new, empty path; built into the VM so that loading original
                // files that redefine strings or collections can't break it
                if self.bootstrap_traits.is_none() {
                    self.bootstrap_traits =
                        self.obj(self.sp.traits).slots.iter().find(|s| &*s.name == "bootstrapPath").and_then(|s| s.value.id());
                }
                let tp = self.bootstrap_traits.map(Value::Ref);
                let names = self.new_string("");
                let mut slots = vec![Self::slot("names", names)];
                if let Some(tp) = tp {
                    slots.insert(0, Self::parent_slot("parent", tp));
                }
                Ok(Value::Ref(self.alloc(plain(slots))))
            }
            "_BootstrapArrow:" => {
                need(1)?;
                let rid = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                let names = self
                    .obj(rid)
                    .slots
                    .iter()
                    .find(|s| &*s.name == "names")
                    .and_then(|s| self.string_of(s.value))
                    .unwrap_or_else(|| "".into());
                if let Some(n) = self.string_of(a[0]) {
                    let joined = format!("{names}\u{1}{n}");
                    let nv = self.new_string(&joined);
                    let mut slots = self.obj(rid).slots.clone();
                    for s in slots.iter_mut() {
                        if &*s.name == "names" {
                            s.value = nv;
                        }
                    }
                    return Ok(Value::Ref(self.alloc(plain(slots))));
                }
                Ok(self.resolve_bootstrap_path(&names))
            }
            "_LoadSubpart:From:" => {
                need(2)?;
                let name = self.str_arg(a[0])?.to_string();
                let dir = self.str_arg(a[1])?.to_string();
                self.load_subpart(&name, &dir).map_err(|m| PrimErr::Unwind(Unwind::Error(SelfError { msg: m, trace: vec![] })))
            }
            "_Perform:" | "_Perform:With:" | "_Perform:With:With:" | "_Perform:With:With:With:" => {
                let s = self.str_arg(a[0])?;
                self.send(recv, &s, a[1..].to_vec()).map_err(PrimErr::Unwind)
            }
            "_Error:" => {
                need(1)?;
                let m = self.string_of(a[0]).map(|s| s.to_string()).unwrap_or_else(|| self.describe(a[0]));
                Err(PrimErr::Unwind(Unwind::Error(SelfError { msg: m, trace: vec![] })))
            }
            "_Inspect" => {
                self.events.push(Event::Inspect(recv));
                Ok(recv)
            }
            "_TranscriptClear" => {
                self.events.push(Event::ClearTranscript);
                Ok(recv)
            }
            "_CurrentTimeString" => {
                let secs = (now_ms() / 1000.0) as i64;
                let (d, t) = (secs.div_euclid(86400), secs.rem_euclid(86400));
                let s = format!("day {d} {:02}:{:02}:{:02} UTC", t / 3600, (t / 60) % 60, t % 60);
                Ok(self.new_string(&s))
            }
            "_TimeMsec" => Ok(Value::Int((now_ms() - self.start_ms) as i64)),
            "_Random:" => {
                need(1)?;
                let n = int(a[0])?;
                if n <= 0 {
                    return Err(Fail("badSizeError".into()));
                }
                let r = self.rand();
                Ok(Value::Int((r % n as u64) as i64))
            }
            "_ObjectCount" => Ok(Value::Int(self.object_count() as i64)),

            // ---- integers (64-bit, growing into big integers when needed)
            _ if sel.starts_with("_Int") => self.int_prim(recv, sel, a),

            // ---- floats
            "_FloatAdd:" => flt2!(|x: f64, y: f64| Ok(Value::Float(x + y))),
            "_FloatSub:" => flt2!(|x: f64, y: f64| Ok(Value::Float(x - y))),
            "_FloatMul:" => flt2!(|x: f64, y: f64| Ok(Value::Float(x * y))),
            "_FloatDiv:" => flt2!(|x: f64, y: f64| if y == 0.0 {
                Err(Fail("divisionByZeroError".into()))
            } else {
                Ok(Value::Float(x / y))
            }),
            "_FloatMod:" => flt2!(|x: f64, y: f64| Ok(Value::Float(x.rem_euclid(y)))),
            "_FloatPow:" => flt2!(|x: f64, y: f64| Ok(Value::Float(x.powf(y)))),
            "_FloatLT:" => flt2!(|x: f64, y: f64| Ok(self.boolean(x < y))),
            "_FloatLE:" => flt2!(|x: f64, y: f64| Ok(self.boolean(x <= y))),
            "_FloatGT:" => flt2!(|x: f64, y: f64| Ok(self.boolean(x > y))),
            "_FloatGE:" => flt2!(|x: f64, y: f64| Ok(self.boolean(x >= y))),
            "_FloatEQ:" => flt2!(|x: f64, y: f64| Ok(self.boolean(x == y))),
            "_FloatNE:" => flt2!(|x: f64, y: f64| Ok(self.boolean(x != y))),
            "_FloatAsInt" | "_FloatTruncate" => {
                let f = num(recv)?;
                if !f.is_finite() || f.abs() > 9.2e18 {
                    return Err(overflow());
                }
                Ok(Value::Int(f.trunc() as i64))
            }
            "_FloatRound" => Ok(Value::Int(num(recv)?.round() as i64)),
            "_FloatFloor" => Ok(Value::Int(num(recv)?.floor() as i64)),
            "_FloatCeil" => Ok(Value::Int(num(recv)?.ceil() as i64)),
            "_FloatSqrt" => Ok(Value::Float(num(recv)?.sqrt())),
            "_FloatSin" => Ok(Value::Float(num(recv)?.sin())),
            "_FloatCos" => Ok(Value::Float(num(recv)?.cos())),
            "_FloatTan" => Ok(Value::Float(num(recv)?.tan())),
            "_FloatArcTan" => Ok(Value::Float(num(recv)?.atan())),
            "_FloatLn" => Ok(Value::Float(num(recv)?.ln())),
            "_FloatExp" => Ok(Value::Float(num(recv)?.exp())),
            "_FloatPrintString" => {
                let s = fmt_float(num(recv)?);
                Ok(self.new_string(&s))
            }

            // ---- strings (immutable, 0-indexed; characters are 1-long strings)
            "_StringSize" => Ok(Value::Int(self.str_arg(recv)?.chars().count() as i64)),
            "_StringAt:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                let i = int(a[0])?;
                let c = if i >= 0 { s.chars().nth(i as usize) } else { None };
                match c {
                    Some(c) => Ok(self.new_string(&c.to_string())),
                    None => Err(Fail("badIndexError".into())),
                }
            }
            "_StringConcat:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                let t = self.str_arg(a[0])?;
                let r = format!("{s}{t}");
                Ok(self.new_string(&r))
            }
            "_StringEQ:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                Ok(self.boolean(self.string_of(a[0]).map(|t| *t == *s).unwrap_or(false)))
            }
            "_StringLT:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                let t = self.str_arg(a[0])?;
                Ok(self.boolean(*s < *t))
            }
            "_StringHash" => {
                let s = self.str_arg(recv)?;
                let mut h: i64 = 1469598103;
                for b in s.bytes() {
                    h = (h ^ b as i64).wrapping_mul(16777619) & 0x3fff_ffff;
                }
                Ok(Value::Int(h))
            }
            "_StringCopyFrom:UpTo:" => {
                need(2)?;
                let s = self.str_arg(recv)?;
                let chars: Vec<char> = s.chars().collect();
                let (from, to) = (int(a[0])?, int(a[1])?);
                if from < 0 || to < from || to as usize > chars.len() {
                    return Err(Fail("badIndexError".into()));
                }
                let r: String = chars[from as usize..to as usize].iter().collect();
                Ok(self.new_string(&r))
            }
            "_StringIndexOf:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                let t = self.str_arg(a[0])?;
                Ok(Value::Int(match s.find(&*t) {
                    Some(b) => s[..b].chars().count() as i64,
                    None => -1,
                }))
            }
            "_StringAsInteger" => {
                let s = self.str_arg(recv)?;
                s.trim().parse::<i64>().map(Value::Int).map_err(|_| Fail("badFormatError".into()))
            }
            "_StringAsFloat" => {
                let s = self.str_arg(recv)?;
                s.trim().parse::<f64>().map(Value::Float).map_err(|_| Fail("badFormatError".into()))
            }
            "_StringAsUppercase" => {
                let s = self.str_arg(recv)?.to_uppercase();
                Ok(self.new_string(&s))
            }
            "_StringAsLowercase" => {
                let s = self.str_arg(recv)?.to_lowercase();
                Ok(self.new_string(&s))
            }
            "_StringReverse" => {
                let s: String = self.str_arg(recv)?.chars().rev().collect();
                Ok(self.new_string(&s))
            }
            "_StringTrim" => {
                let s = self.str_arg(recv)?.trim().to_string();
                Ok(self.new_string(&s))
            }
            "_StringSplit:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                let sep = self.str_arg(a[0])?;
                // ' ' splits on runs of white space (for `words`), '\n' on line
                // breaks (also \r\n); anything else exactly
                let parts: Vec<String> = if &*sep == " " {
                    s.split_whitespace().map(|x| x.to_string()).collect()
                } else if &*sep == "\n" {
                    s.split('\n').map(|x| x.strip_suffix('\r').unwrap_or(x).to_string()).collect()
                } else if sep.is_empty() {
                    s.chars().map(|c| c.to_string()).collect()
                } else {
                    s.split(&*sep).map(|x| x.to_string()).collect()
                };
                let items: Vec<Value> = parts.iter().map(|p| self.new_string(p)).collect();
                Ok(self.new_vector(items))
            }
            "_StringByteAt:" => {
                need(1)?;
                let s = self.str_arg(recv)?;
                let i = int(a[0])?;
                match if i >= 0 { s.chars().nth(i as usize) } else { None } {
                    Some(c) => Ok(Value::Int(c as i64)),
                    None => Err(Fail("badIndexError".into())),
                }
            }
            "_StringPrintString" => {
                let s = self.str_arg(recv)?;
                let q = quote(&s);
                Ok(self.new_string(&q))
            }
            "_StringPrint" => {
                let s = self.str_arg(recv)?;
                self.events.push(Event::Print(s.to_string()));
                Ok(recv)
            }
            "_StringPrintLine" => {
                let s = self.str_arg(recv)?;
                self.events.push(Event::Print(format!("{s}\n")));
                Ok(recv)
            }
            "_StringEval" => {
                let s = self.str_arg(recv)?;
                let lobby = Value::Ref(self.sp.lobby);
                let saved = (self.steps, self.depth);
                let r = self.eval_doit(&s, lobby);
                self.steps = saved.0;
                self.depth = saved.1;
                r.map_err(|e| PrimErr::Unwind(Unwind::Error(e)))
            }

            // ---- vectors (0-indexed)
            "_Size" => match recv {
                Value::Ref(id) => match &self.obj(id).kind {
                    Kind::Vector(v) => Ok(Value::Int(v.len() as i64)),
                    Kind::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                    _ => Ok(Value::Int(0)),
                },
                _ => Err(Fail("badTypeError".into())),
            },
            "_At:" => {
                need(1)?;
                let i = int(a[0])?;
                match recv {
                    Value::Ref(id) => match &self.obj(id).kind {
                        Kind::Vector(v) => {
                            if i >= 0 && (i as usize) < v.len() {
                                Ok(v[i as usize])
                            } else {
                                Err(Fail("badIndexError".into()))
                            }
                        }
                        _ => Err(Fail("badTypeError".into())),
                    },
                    _ => Err(Fail("badTypeError".into())),
                }
            }
            "_At:Put:" => {
                need(2)?;
                let i = int(a[0])?;
                let id = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                match &mut self.heap[id as usize].kind {
                    Kind::Vector(v) => {
                        if i >= 0 && (i as usize) < v.len() {
                            v[i as usize] = a[1];
                            Ok(recv)
                        } else {
                            Err(Fail("badIndexError".into()))
                        }
                    }
                    _ => Err(Fail("badTypeError".into())),
                }
            }
            "_Clone:Filler:" => {
                need(2)?;
                let n = int(a[0])?;
                if !(0..=50_000_000).contains(&n) {
                    return Err(Fail("badSizeError".into()));
                }
                let id = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                let o = self.obj(id);
                let old: Vec<Value> = match &o.kind {
                    Kind::Vector(v) => v.clone(),
                    _ => return Err(Fail("badTypeError".into())),
                };
                let mut items: Vec<Value> = old.into_iter().take(n as usize).collect();
                items.resize(n as usize, a[1]);
                let slots = o.slots.clone();
                Ok(Value::Ref(self.alloc(Obj { slots, kind: Kind::Vector(items), creator: None, comment: None })))
            }
            "_MakeVector" => {
                let slots = match recv {
                    Value::Ref(id) => self.obj(id).slots.clone(),
                    _ => vec![],
                };
                Ok(Value::Ref(self.alloc(Obj { slots, kind: Kind::Vector(vec![]), creator: None, comment: None })))
            }

            // ---- blocks
            "_BlockNumArgs" => match recv {
                Value::Ref(id) => match &self.obj(id).kind {
                    Kind::Block { lit, .. } => Ok(Value::Int(lit.arg_names().len() as i64)),
                    _ => Err(Fail("badTypeError".into())),
                },
                _ => Err(Fail("badTypeError".into())),
            },
            "_WhileTrue:" | "_WhileFalse:" => {
                need(1)?;
                let want = sel == "_WhileTrue:";
                let cond = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                loop {
                    let c = self.call_block(cond, vec![]).map_err(PrimErr::Unwind)?;
                    let b = self.is_true(c).map_err(PrimErr::Unwind)?;
                    if b != want {
                        break;
                    }
                    self.send(a[0], "value", vec![]).map_err(PrimErr::Unwind)?;
                    self.steps += 1;
                    if self.steps > self.step_limit {
                        return Err(Self::too_long());
                    }
                }
                Ok(self.nil())
            }
            "_Loop" => {
                let b = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                loop {
                    self.call_block(b, vec![]).map_err(PrimErr::Unwind)?;
                    self.steps += 1;
                    if self.steps > self.step_limit {
                        return Err(Self::too_long());
                    }
                }
            }
            "_ValueWithArguments:" => {
                need(1)?;
                let items = self.vector_items(a[0]).ok_or_else(|| Fail("badTypeError".into()))?;
                let b = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                self.call_block(b, items).map_err(PrimErr::Unwind)
            }
            "_OnError:" => {
                // run the receiver block; on a Self error, call the handler with the message
                need(1)?;
                let b = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                let saved = self.depth;
                match self.call_block(b, vec![]) {
                    Ok(v) => Ok(v),
                    Err(Unwind::Error(e)) => {
                        self.depth = saved;
                        let m = self.new_string(&e.msg);
                        self.call_block_lenient(a[0], vec![m]).map_err(PrimErr::Unwind)
                    }
                    Err(u) => Err(PrimErr::Unwind(u)),
                }
            }
            "_OnReturn:" | "_OnNonLocalReturn:" => {
                // unwind protection, as in the original: run the receiver block, then the
                // argument block (always for _OnReturn:, only when a non-local return or an
                // error leaves the receiver for _OnNonLocalReturn:)
                need(1)?;
                let b = recv.id().ok_or_else(|| Fail("badTypeError".into()))?;
                let saved = self.depth;
                let r = self.call_block(b, vec![]);
                self.depth = saved;
                let always = sel == "_OnReturn:";
                if always || r.is_err() {
                    let arg = match &r {
                        Ok(v) => *v,
                        Err(_) => self.nil(),
                    };
                    self.call_block_lenient(a[0], vec![arg]).map_err(PrimErr::Unwind)?;
                }
                r.map_err(PrimErr::Unwind)
            }
            "_FileContents" | "_SetFileContents:" | "_FileExists" => self.file_prim(recv, sel, a),
            "_SourceFolder" => {
                // the folder of the file being loaded (nil in the browser or for typed code)
                #[cfg(not(target_arch = "wasm32"))]
                let dir = self.source_dir.as_ref().map(|d| std::fs::canonicalize(d).unwrap_or(d.clone()).to_string_lossy().to_string());
                #[cfg(target_arch = "wasm32")]
                let dir = self.source_dir.as_ref().map(|d| d.to_string_lossy().to_string());
                Ok(match dir {
                    Some(d) if !d.is_empty() => self.new_string(&d),
                    _ => self.nil(),
                })
            }
            "_FileOut" => {
                let s = self.file_out_object(recv);
                Ok(self.new_string(&s))
            }
            "_FileOutChanges" => {
                let s = self.file_out_changes();
                Ok(self.new_string(&s))
            }
            _ => Err(PrimErr::Unwind(Unwind::Error(SelfError {
                msg: format!("unknown primitive {sel}"),
                trace: vec![],
            }))),
        }
    }

    fn big_of(&self, v: Value) -> Option<BigInt> {
        match v {
            Value::Int(i) => Some(BigInt::from(i)),
            Value::Ref(id) => match &self.obj(id).kind {
                Kind::Big(b) => Some((**b).clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn norm_big(&mut self, b: BigInt) -> Value {
        match b.to_i64() {
            Some(i) => Value::Int(i),
            None => Value::Ref(self.alloc(Obj { slots: vec![], kind: Kind::Big(Rc::new(b)), creator: None, comment: None })),
        }
    }

    #[inline(never)]
    fn int_prim(&mut self, recv: Value, sel: &str, a: &[Value]) -> Result<Value, PrimErr> {
        use PrimErr::Fail;
        let bad = || Fail("badTypeError".into());
        // unary
        match sel {
            "_IntAsFloat" => {
                let b = self.big_of(recv).ok_or_else(bad)?;
                return Ok(Value::Float(b.to_f64().unwrap_or(f64::INFINITY)));
            }
            "_IntPrintString" => {
                let s = self.big_of(recv).ok_or_else(bad)?.to_string();
                return Ok(self.new_string(&s));
            }
            "_IntAsCharacter" => {
                let i = self.big_of(recv).and_then(|b| b.to_u32()).ok_or_else(bad)?;
                let c = char::from_u32(i).ok_or_else(|| Fail("badSizeError".into()))?;
                return Ok(self.new_string(&c.to_string()));
            }
            _ => {}
        }
        if a.len() != 1 {
            return Err(Fail("wrongNumberOfArgumentsError".into()));
        }
        if sel == "_IntPrintStringBase:" {
            let b = self.big_of(recv).ok_or_else(bad)?;
            let base = match a[0] {
                Value::Int(x) if (2..=36).contains(&x) => x as u32,
                _ => return Err(Fail("badSizeError".into())),
            };
            let s = b.to_str_radix(base);
            return Ok(self.new_string(&s));
        }
        // fast path: both small
        if let (Value::Int(x), Value::Int(y)) = (recv, a[0]) {
            let fast = match sel {
                "_IntAdd:" => x.checked_add(y).map(Value::Int),
                "_IntSub:" => x.checked_sub(y).map(Value::Int),
                "_IntMul:" => x.checked_mul(y).map(Value::Int),
                "_IntDiv:" if y != 0 => x.checked_div_euclid(y).map(Value::Int),
                "_IntMod:" if y != 0 => x.checked_rem_euclid(y).map(Value::Int),
                "_IntRem:" if y != 0 => x.checked_rem(y).map(Value::Int),
                "_IntAnd:" => Some(Value::Int(x & y)),
                "_IntOr:" => Some(Value::Int(x | y)),
                "_IntXor:" => Some(Value::Int(x ^ y)),
                "_IntArithmeticShiftRight:" => Some(Value::Int(x.wrapping_shr(y.clamp(0, 63) as u32))),
                "_IntLT:" => Some(self.boolean(x < y)),
                "_IntLE:" => Some(self.boolean(x <= y)),
                "_IntGT:" => Some(self.boolean(x > y)),
                "_IntGE:" => Some(self.boolean(x >= y)),
                "_IntEQ:" => Some(self.boolean(x == y)),
                "_IntNE:" => Some(self.boolean(x != y)),
                _ => None,
            };
            if let Some(v) = fast {
                return Ok(v);
            }
        }
        let x = self.big_of(recv).ok_or_else(bad)?;
        let y = self.big_of(a[0]).ok_or_else(bad)?;
        let zero = || Fail("divisionByZeroError".into());
        let r = match sel {
            "_IntAdd:" => x + y,
            "_IntSub:" => x - y,
            "_IntMul:" => x * y,
            "_IntDiv:" => {
                if y.is_zero() {
                    return Err(zero());
                }
                x.div_floor(&y)
            }
            "_IntMod:" => {
                if y.is_zero() {
                    return Err(zero());
                }
                x.mod_floor(&y)
            }
            "_IntRem:" => {
                if y.is_zero() {
                    return Err(zero());
                }
                x % y
            }
            "_IntAnd:" => x & y,
            "_IntOr:" => x | y,
            "_IntXor:" => x ^ y,
            "_IntArithmeticShiftLeft:" => {
                let n = y.to_u32().filter(|n| *n < 100_000).ok_or_else(|| Fail("badSizeError".into()))?;
                x << n
            }
            "_IntArithmeticShiftRight:" => {
                let n = y.to_u32().ok_or_else(|| Fail("badSizeError".into()))?;
                x >> n
            }
            "_IntLT:" => return Ok(self.boolean(x < y)),
            "_IntLE:" => return Ok(self.boolean(x <= y)),
            "_IntGT:" => return Ok(self.boolean(x > y)),
            "_IntGE:" => return Ok(self.boolean(x >= y)),
            "_IntEQ:" => return Ok(self.boolean(x == y)),
            "_IntNE:" => return Ok(self.boolean(x != y)),
            _ => {
                return Err(PrimErr::Unwind(Unwind::Error(SelfError {
                    msg: format!("unknown primitive {sel}"),
                    trace: vec![],
                })))
            }
        };
        let _ = r.is_negative();
        Ok(self.norm_big(r))
    }

    /// Files, for Self programs on the desktop: `'notes.txt' asFileContents`,
    /// `'notes.txt' setFileContentsTo: 'hi'`, `'notes.txt' fileExists`.
    /// Relative names are looked up next to the file being loaded first.
    fn file_prim(&mut self, recv: Value, sel: &str, a: &[Value]) -> Result<Value, PrimErr> {
        use PrimErr::Fail;
        let name = self.str_arg(recv)?;
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, a);
            if sel == "_FileExists" {
                return Ok(self.boolean(false));
            }
            Err(Fail("filesAreNotAvailableInTheBrowser".into()))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut path = std::path::PathBuf::from(&*name);
            if path.is_relative() {
                if let Some(dir) = &self.source_dir {
                    let near = dir.join(&path);
                    if near.exists() || sel == "_SetFileContents:" {
                        path = near;
                    }
                }
            }
            match sel {
                "_FileExists" => Ok(self.boolean(path.exists())),
                "_FileContents" => match std::fs::read_to_string(&path) {
                    Ok(t) => Ok(self.new_string(&t)),
                    Err(e) => Err(Fail(format!("cannot read {}: {e}", path.display()))),
                },
                _ => {
                    if a.len() != 1 {
                        return Err(Fail("wrongNumberOfArgumentsError".into()));
                    }
                    let text = self.str_arg(a[0])?;
                    match std::fs::write(&path, text.as_bytes()) {
                        Ok(()) => Ok(recv),
                        Err(e) => Err(Fail(format!("cannot write {}: {e}", path.display()))),
                    }
                }
            }
        }
    }

    fn str_arg(&self, v: Value) -> Result<Rc<str>, PrimErr> {
        self.string_of(v).ok_or_else(|| PrimErr::Fail("badTypeError".into()))
    }

    pub fn new_vector(&mut self, items: Vec<Value>) -> Value {
        // vectors made by primitives get the vector prototype's slots
        let proto = self.obj(self.sp.globals).slots.iter().find(|s| &*s.name == "vector").and_then(|s| s.value.id());
        let slots = proto.map(|p| self.obj(p).slots.clone()).unwrap_or_default();
        Value::Ref(self.alloc(Obj { slots, kind: Kind::Vector(items), creator: None, comment: None }))
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

enum PrimErr {
    Fail(String),
    Unwind(Unwind),
}

fn is_setter_of(sel: &str, name: &str) -> bool {
    sel.len() == name.len() + 1 && sel.ends_with(':') && sel.starts_with(name)
}

fn is_value_selector(sel: &str, n: usize) -> bool {
    match n {
        0 => sel == "value",
        1 => sel == "value:",
        2 => sel == "value:With:",
        3 => sel == "value:With:With:",
        4 => sel == "value:With:With:With:",
        _ => false,
    }
}

fn dedupe(found: &mut Vec<Found>) {
    let mut seen: Vec<(ObjId, usize)> = vec![];
    found.retain(|f| {
        if seen.contains(&(f.holder, f.index)) {
            false
        } else {
            seen.push((f.holder, f.index));
            true
        }
    });
}

pub fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('\'');
    for c in s.chars() {
        match c {
            '\'' => o.push_str("\\'"),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('\'');
    o
}

pub fn fmt_float(f: f64) -> String {
    if f.is_finite() && f == f.trunc() && f.abs() < 1e15 {
        format!("{f:.1}")
    } else if f.is_finite() && (f.abs() >= 1e15 || (f != 0.0 && f.abs() < 1e-6)) {
        format!("{f:e}")
    } else {
        format!("{f}")
    }
}


#[cfg(not(target_arch = "wasm32"))]
fn grow_stack<T>(f: impl FnOnce() -> T) -> T {
    stacker::maybe_grow(256 * 1024, 4 * 1024 * 1024, f)
}

#[cfg(target_arch = "wasm32")]
fn grow_stack<T>(f: impl FnOnce() -> T) -> T {
    f()
}
