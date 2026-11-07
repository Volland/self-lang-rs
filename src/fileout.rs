//! Saving objects as Self source ("filing out", as the original Self called it).
//!
//! `file_out_changes` writes every slot a person added to (or method they
//! changed in) the lobby, globals, traits, defaultBehavior and their direct
//! children since start-up, as ordinary `_AddSlots:` statements. Loading the
//! file into a fresh world brings those objects back. `file_out_object`
//! writes one object as an object literal (Self's `storeString`).
//!
//! Objects that already have a name (`traits clonable`, `bankAccount`) are
//! written as that name; objects that belong to the slot being saved are
//! written out in full. Blocks are closures over running code and can't be
//! written as source, so they are saved as nil.

use crate::vm::{fmt_float, quote, Kind, ObjId, Slot, Value, Vm};
use std::collections::HashSet;
use std::rc::Rc;

const MAX_DEPTH: usize = 24;

impl Vm {
    /// The slots of the objects whose changes are saved, right after start-up.
    pub(crate) fn snapshot_slots(&self) -> Vec<(ObjId, Vec<(Rc<str>, Value)>)> {
        let tops = [self.sp.traits, self.sp.globals, self.sp.default_behavior, self.sp.lobby];
        let mut ids: Vec<ObjId> = tops.to_vec();
        for &t in &tops {
            for s in &self.obj(t).slots {
                if let Value::Ref(c) = s.value {
                    if !s.parent && matches!(self.obj(c).kind, Kind::Plain) && !ids.contains(&c) {
                        ids.push(c);
                    }
                }
            }
        }
        ids.into_iter().map(|id| (id, self.obj(id).slots.iter().map(|s| (s.name.clone(), s.value)).collect())).collect()
    }

    fn top_name(&self, id: ObjId) -> Option<String> {
        if id == self.sp.lobby {
            Some("lobby".into())
        } else if id == self.sp.globals {
            Some("globals".into())
        } else if id == self.sp.traits {
            Some("traits".into())
        } else if id == self.sp.default_behavior {
            Some("defaultBehavior".into())
        } else {
            self.path_name(id)
        }
    }

    /// Everything added or changed since start-up, as a loadable Self file.
    pub fn file_out_changes(&self) -> String {
        let mut out = String::from(
            "\"Objects saved from Self World.\n Load this file with World > Load a Self file, by dropping it on the window,\n or from the command line:  self-world --open this-file.self\"\n\n",
        );
        let mut count = 0;
        for (id, initial) in &self.initial_slots {
            let id = *id;
            if !self.is_alive(Value::Ref(id)) {
                continue;
            }
            let Some(path) = self.top_name(id) else { continue };
            for s in &self.obj(id).slots {
                if s.arg {
                    continue;
                }
                let before = initial.iter().find(|(n, _)| *n == s.name).map(|(_, v)| *v);
                let changed = match before {
                    None => true,
                    // a changed method or constant; assignable data changes as programs run
                    Some(v) => !v.same(s.value) && (self.is_method(s.value) || !s.assignable),
                };
                if !changed {
                    continue;
                }
                let top = if path == "lobby" || path == "globals" { s.name.to_string() } else { format!("{path} {}", s.name) };
                let decl = self.slot_decl(s, &top, &mut HashSet::new(), 1);
                let decl = if s.category.is_empty() { decl } else { format!("{{ 'Category: {}'\n  {decl} }}", escape(&s.category)) };
                out.push_str(&format!("{path} _AddSlots: (|\n  {decl}\n|).\n\n"));
                count += 1;
            }
        }
        if count == 0 {
            out.push_str("\"Nothing to save yet: add slots to the lobby, e.g.\n   lobby _AddSlots: (| dog = (| parent* = traits clonable. name <- 'Rex' |) |)\"\n");
        }
        out
    }

    /// One object as an object literal (or its name, if it has one elsewhere).
    pub fn file_out_object(&self, v: Value) -> String {
        let top = v.id().and_then(|id| self.path_name(id)).unwrap_or_default();
        self.expr_of(v, &top, &mut HashSet::new(), 0)
    }

    fn slot_decl(&self, s: &Slot, top: &str, seen: &mut HashSet<ObjId>, depth: usize) -> String {
        if self.is_method(s.value) {
            if let Some(d) = self.method_declaration(&s.name, s.value) {
                return d;
            }
        }
        let star = if s.parent { "*" } else { "" };
        let op = if s.assignable { "<-" } else { "=" };
        let expr = self.expr_of(s.value, top, seen, depth);
        format!("{}{star} {op} {expr}", s.name)
    }

    fn expr_of(&self, v: Value, top: &str, seen: &mut HashSet<ObjId>, depth: usize) -> String {
        let id = match v {
            Value::Int(i) => return i.to_string(),
            Value::Float(f) => return fmt_float(f),
            Value::Ref(id) => id,
        };
        let sp = &self.sp;
        for (special, name) in [(sp.nil, "nil"), (sp.true_, "true"), (sp.false_, "false"), (sp.lobby, "lobby"), (sp.globals, "globals")] {
            if id == special {
                return name.into();
            }
        }
        let o = self.obj(id);
        match &o.kind {
            Kind::Str(s) => return quote(s),
            Kind::Big(b) => return b.to_string(),
            Kind::Block { .. } => return "nil \"a block: blocks can't be saved as source\"".into(),
            Kind::Method(_) | Kind::Free => return "nil".into(),
            Kind::Plain | Kind::Vector(_) => {}
        }
        // an object with its own name elsewhere is referred to by that name
        if let Some(p) = self.path_name(id) {
            let inside = !top.is_empty() && (p == top || p.starts_with(&format!("{top} ")));
            if !inside {
                return p;
            }
        }
        if depth > MAX_DEPTH || !seen.insert(id) {
            return "nil \"shared or circular: saved once only\"".into();
        }
        let pad = "  ".repeat(depth + 1);
        if let Kind::Vector(items) = &o.kind {
            let parts: Vec<String> = items
                .iter()
                .map(|e| {
                    let s = self.expr_of(*e, top, seen, depth + 1);
                    if s.contains(' ') || s.starts_with('-') { format!("({s})") } else { s }
                })
                .collect();
            return match parts.len() {
                0 => "(vector copySize: 0)".into(),
                1 => format!("(vector copySize: 1 FillingWith: {})", parts[0]),
                _ => format!("({}) asVector", parts.join(" & ")),
            };
        }
        let mut decls: Vec<String> = vec![];
        if let Some(c) = &o.comment {
            decls.push(format!("{{}} = {}", quote(c)));
        }
        for s in o.slots.iter().filter(|s| !s.arg) {
            decls.push(self.slot_decl(s, top, seen, depth + 1));
        }
        if decls.is_empty() {
            return "(| |)".into();
        }
        let short = decls.iter().all(|d| !d.contains('\n')) && decls.iter().map(|d| d.len() + 2).sum::<usize>() < 70;
        if short {
            format!("(| {} |)", decls.join(". "))
        } else {
            format!("(|\n{pad}{} |)", decls.join(&format!(".\n{pad}")))
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}
