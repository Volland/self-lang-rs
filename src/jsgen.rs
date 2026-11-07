//! Translating Self to JavaScript.
//!
//! The parser's syntax tree becomes JavaScript that runs on a small runtime
//! (src/js/runtime.js) implementing Self's object model: objects made of
//! slots, lookup through parent slots, blocks as closures, `^` returns from
//! blocks (thrown and caught by their method), resends and the primitives.
//!
//! Local slots and arguments become JavaScript variables, so the lexical
//! scoping of methods and blocks is JavaScript's own; every other message is
//! a `$.send(receiver, selector, [arguments])` that looks the selector up at
//! run time, exactly as the interpreter does. The starting world
//! (world.self, morphic.self) goes through the same translation, so a
//! translated program has the whole Self library.
//!
//! `web_app` wraps it all, with a DOM renderer for morphs (src/js/morphic.js),
//! into one HTML file that runs in any browser.

use crate::parser::{parse_doit, parse_file, Expr, ObjLit, SlotKind, Stmt};
use std::rc::Rc;

/// The runtime every translated program needs.
pub const RUNTIME_JS: &str = include_str!("js/runtime.js");
/// Draws morphs with the browser's DOM, for exported web apps.
pub const MORPHIC_JS: &str = include_str!("js/morphic.js");

#[derive(Clone)]
struct Local {
    name: Rc<str>,
    assignable: bool,
}

struct Ctx {
    scopes: Vec<Vec<Local>>,
    /// Inside a block or a parenthesised scope: `^` must be thrown back to the method.
    nested: bool,
}

struct Gen {
    /// One flag per method being compiled: does it need a home for `^` from blocks?
    homes: Vec<bool>,
    pretty: bool,
}

fn js_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{2028}' => o.push_str("\\u2028"),
            '\u{2029}' => o.push_str("\\u2029"),
            '<' => o.push_str("\\u003c"), // keeps "</script>" out of exported pages
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn var(name: &str) -> String {
    format!("v_{name}")
}

impl Gen {
    fn nl(&self, indent: usize) -> String {
        if self.pretty {
            format!("\n{}", "  ".repeat(indent))
        } else {
            String::new()
        }
    }

    fn find<'a>(ctx: &'a Ctx, name: &str) -> Option<&'a Local> {
        ctx.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|l| &*l.name == name)
    }

    fn args(&mut self, args: &[Expr], ctx: &mut Ctx, ind: usize) -> String {
        let a: Vec<String> = args.iter().map(|e| self.expr(e, ctx, ind)).collect();
        format!("[{}]", a.join(", "))
    }

    fn expr(&mut self, e: &Expr, ctx: &mut Ctx, ind: usize) -> String {
        match e {
            Expr::Int(i) => {
                if *i < 0 {
                    format!("({i}n)")
                } else {
                    format!("{i}n")
                }
            }
            Expr::Float(f) => {
                let s = format!("{f:?}");
                if *f < 0.0 {
                    format!("({s})")
                } else {
                    s
                }
            }
            Expr::Str(s) => js_str(s),
            Expr::SelfRef => "self".into(),
            Expr::Object(lit) => self.object(lit, ctx, ind),
            Expr::Method(lit) => self.method(lit, ctx, ind),
            Expr::Block(lit) => self.block(lit, ctx, ind),
            Expr::Paren(lit) => self.paren(lit, ctx, ind),
            Expr::Send { recv, sel, args, resend } => {
                if let Some(target) = resend {
                    let a = self.args(args, ctx, ind);
                    return format!("$.resend(self, holder, {}, {}, {a})", js_str(target), js_str(sel));
                }
                match recv {
                    Some(r) => {
                        let rv = self.expr(r, ctx, ind);
                        let a = self.args(args, ctx, ind);
                        if sel.starts_with('_') {
                            format!("$.prim({rv}, {}, {a})", js_str(sel))
                        } else {
                            format!("$.send({rv}, {}, {a})", js_str(sel))
                        }
                    }
                    None => {
                        if &**sel == "_Restart" {
                            return "$.restart()".into();
                        }
                        // locals and arguments are JavaScript variables
                        if args.is_empty() {
                            if Self::find(ctx, sel).is_some() {
                                return var(sel);
                            }
                        } else if args.len() == 1 && sel.ends_with(':') && !sel[..sel.len() - 1].contains(':') {
                            let name = &sel[..sel.len() - 1];
                            if Self::find(ctx, name).map(|l| l.assignable).unwrap_or(false) {
                                let v = self.expr(&args[0], ctx, ind);
                                return format!("({} = {v}, self)", var(name));
                            }
                        }
                        let a = self.args(args, ctx, ind);
                        if sel.starts_with('_') {
                            format!("$.prim(self, {}, {a})", js_str(sel))
                        } else {
                            format!("$.send(self, {}, {a})", js_str(sel))
                        }
                    }
                }
            }
        }
    }

    fn object(&mut self, lit: &ObjLit, ctx: &mut Ctx, ind: usize) -> String {
        let mut parts = vec![];
        for s in &lit.slots {
            let flags = (if s.kind == SlotKind::Assignable { 1 } else { 0 })
                | (if s.parent { 2 } else { 0 })
                | (if s.kind == SlotKind::Argument { 4 } else { 0 });
            let v = match &s.init {
                Some(e) => self.expr(e, ctx, ind + 1),
                None => "$.nil".into(),
            };
            parts.push(format!("[{}, {flags}, {v}]", js_str(&s.name)));
        }
        let comment = lit.comment.as_ref().map(|c| format!(", {}", js_str(c))).unwrap_or_default();
        if parts.is_empty() {
            return format!("$.obj([]{comment})");
        }
        let nl = self.nl(ind + 1);
        let end = self.nl(ind);
        format!("$.obj([{nl}{}{end}]{comment})", parts.join(&format!(",{nl}")))
    }

    /// Declarations for a scope's slots; `inits` gives each non-argument
    /// slot's initial value (already compiled).
    fn declare(&self, lit: &ObjLit, inits: &[String], arg_src: &str, ind: usize) -> (String, Vec<Local>) {
        let mut out = String::new();
        let mut locals = vec![];
        let (mut ai, mut li) = (0, 0);
        for s in &lit.slots {
            let v = if s.kind == SlotKind::Argument {
                ai += 1;
                format!("{arg_src}[{}]", ai - 1)
            } else {
                li += 1;
                inits[li - 1].clone()
            };
            out.push_str(&format!("{}let {} = {v};", self.nl(ind), var(&s.name)));
            locals.push(Local { name: s.name.clone(), assignable: s.kind != SlotKind::Constant });
        }
        (out, locals)
    }

    fn inits_in(&mut self, lit: &ObjLit, ctx: &mut Ctx, ind: usize) -> Vec<String> {
        lit.slots
            .iter()
            .filter(|s| s.kind != SlotKind::Argument)
            .map(|s| match &s.init {
                Some(e) => self.expr(e, ctx, ind),
                None => "$.nil".into(),
            })
            .collect()
    }

    /// The statements of a method, block or scope body.
    fn statements(&mut self, code: &[Stmt], ctx: &mut Ctx, ind: usize) -> String {
        let mut out = String::new();
        if code.is_empty() {
            return format!("{}return $.nil;", self.nl(ind));
        }
        for (i, st) in code.iter().enumerate() {
            let last = i + 1 == code.len();
            match st {
                Stmt::Expr(e) => {
                    let s = self.expr(e, ctx, ind);
                    if last {
                        out.push_str(&format!("{}return {s};", self.nl(ind)));
                    } else {
                        out.push_str(&format!("{}{s};", self.nl(ind)));
                    }
                }
                Stmt::Return(e) => {
                    let s = self.expr(e, ctx, ind);
                    if ctx.nested {
                        if let Some(h) = self.homes.last_mut() {
                            *h = true;
                        }
                        out.push_str(&format!("{}return $.nlr(home, {s});", self.nl(ind)));
                    } else {
                        out.push_str(&format!("{}return {s};", self.nl(ind)));
                    }
                    break;
                }
            }
        }
        out
    }

    fn method(&mut self, lit: &ObjLit, outer: &mut Ctx, ind: usize) -> String {
        // local slots get their values once, when the method is made (as in the interpreter)
        let inits = self.inits_in(lit, outer, ind + 1);
        let nargs = lit.slots.iter().filter(|s| s.kind == SlotKind::Argument).count();
        let captured: Vec<String> = (0..inits.len()).map(|i| format!("i{i}")).collect();
        let (decls, locals) = self.declare(lit, &captured, "args", ind + 2);
        let mut ctx = Ctx { scopes: vec![locals], nested: false };
        self.homes.push(false);
        let body = self.statements(lit.code.as_deref().unwrap_or(&[]), &mut ctx, ind + 2);
        let needs_home = self.homes.pop().unwrap_or(false);
        let n1 = self.nl(ind + 1);
        let inner = if needs_home {
            let n2 = self.nl(ind + 2);
            format!(
                "{n1}const home = {{ alive: true }};{n1}try {{{decls}{body}{n1}}} catch (e) {{{n2}if (e instanceof $.NLR && e.home === home) return e.value;{n2}throw e;{n1}}} finally {{{n2}home.alive = false;{n1}}}"
            )
        } else {
            format!("{decls}{body}")
        };
        let f = format!("$.method({nargs}, (self, holder, args) => {{{inner}{}}})", self.nl(ind));
        if inits.is_empty() {
            f
        } else {
            format!("(({}) => {f})({})", captured.join(", "), inits.join(", "))
        }
    }

    fn block(&mut self, lit: &ObjLit, outer: &mut Ctx, ind: usize) -> String {
        let inits = self.inits_in(lit, outer, ind + 1);
        let nargs = lit.slots.iter().filter(|s| s.kind == SlotKind::Argument).count();
        let (decls, locals) = self.declare(lit, &inits, "args", ind + 1);
        let saved = outer.nested;
        outer.scopes.push(locals);
        outer.nested = true;
        let body = self.statements(lit.code.as_deref().unwrap_or(&[]), outer, ind + 1);
        outer.scopes.pop();
        outer.nested = saved;
        format!("$.block({nargs}, (args) => {{{decls}{body}{}}})", self.nl(ind))
    }

    fn paren(&mut self, lit: &ObjLit, ctx: &mut Ctx, ind: usize) -> String {
        let code = lit.code.as_deref().unwrap_or(&[]);
        let has_return = code.iter().any(|s| matches!(s, Stmt::Return(_)));
        if lit.slots.is_empty() && !has_return {
            let parts: Vec<String> = code
                .iter()
                .map(|s| match s {
                    Stmt::Expr(e) | Stmt::Return(e) => self.expr(e, ctx, ind),
                })
                .collect();
            return match parts.len() {
                0 => "$.nil".into(),
                1 => format!("({})", parts[0]),
                _ => format!("({})", parts.join(", ")),
            };
        }
        // a scope with its own locals (or a return): an immediately-called function
        let inits = self.inits_in(lit, ctx, ind + 1);
        let (decls, locals) = self.declare(lit, &inits, "[]", ind + 1);
        let saved = ctx.nested;
        ctx.scopes.push(locals);
        ctx.nested = true;
        let body = self.statements(code, ctx, ind + 1);
        ctx.scopes.pop();
        ctx.nested = saved;
        format!("(() => {{{decls}{body}{}}})()", self.nl(ind))
    }

    /// A whole file or "do it": each statement runs on its own against the
    /// lobby, as when Self loads a file.
    fn program(&mut self, lit: &ObjLit, starts: &[usize], src: &str) -> String {
        let mut ctx = Ctx { scopes: vec![], nested: false };
        let inits = self.inits_in(&lit, &mut ctx, 1);
        let (decls, locals) = self.declare(lit, &inits, "[]", 1);
        ctx.scopes.push(locals);
        // statements are closures: `^` inside their blocks returns through the program's home
        ctx.nested = true;
        self.homes.push(false);
        let mut out = String::from("(function ($, self) {\n  const holder = self;\n  const home = { alive: true };\n  let last = $.nil;");
        out.push_str(&decls);
        for (i, st) in lit.code.as_deref().unwrap_or(&[]).iter().enumerate() {
            let line = starts.get(i).map(|p| crate::parser::line_col(src, *p).0).unwrap_or(i + 1);
            let e = match st {
                Stmt::Expr(e) | Stmt::Return(e) => e,
            };
            let saved = ctx.nested;
            ctx.nested = false;
            let s = self.expr(e, &mut ctx, 2);
            ctx.nested = saved;
            if self.pretty {
                out.push_str(&format!("\n  last = $.stmt({line}, () =>\n    {s});"));
            } else {
                out.push_str(&format!("\nlast = $.stmt({line}, () => {s});"));
            }
        }
        self.homes.pop();
        out.push_str("\n  return last;\n})");
        out
    }
}

fn compile(src: &str, pretty: bool) -> Result<String, String> {
    let (lit, starts) = parse_file(src).map_err(|e| format!("Syntax error at {}", e.describe_in(src)))?;
    let mut g = Gen { homes: vec![], pretty };
    Ok(g.program(&lit, &starts, src))
}

/// Translate a Self program (a file or a "do it") to readable JavaScript: a
/// function of the runtime `$` and the receiver, one `$.stmt` per statement.
pub fn translate(src: &str) -> Result<String, String> {
    let body = compile(src, true)?;
    Ok(format!(
        "// Translated from Self by Self World. Run it with the Self World runtime:\n//   (program)(SelfRT, SelfRT.lobby)\n// Every message is $.send(receiver, selector, [arguments]); locals are JavaScript variables.\nconst program = {body};\n"
    ))
}

/// Check an expression parses (for the "do it" style translator).
pub fn check(src: &str) -> Result<(), String> {
    parse_doit(src).map(|_| ()).map_err(|e| format!("Syntax error at {}", e.describe_in(src)))
}

/// The starting world (world.self and morphic.self), translated.
fn world_js() -> Result<String, String> {
    let w = compile(crate::world::WORLD_SOURCE, false).map_err(|e| format!("world.self: {e}"))?;
    let m = compile(crate::world::MORPHIC_SOURCE, false).map_err(|e| format!("morphic.self: {e}"))?;
    Ok(format!("({w})(SelfRT, SelfRT.lobby);\n({m})(SelfRT, SelfRT.lobby);\n"))
}

/// A complete script for a JavaScript engine such as Node: the runtime, the
/// starting world and the program; prints what the program printed and the
/// printString of its last statement.
pub fn script(src: &str, extra_world: &[&str]) -> Result<String, String> {
    let prog = compile(src, false)?;
    let mut extra = String::new();
    for (i, w) in extra_world.iter().enumerate() {
        let c = compile(w, false).map_err(|e| format!("extra file {i}: {e}"))?;
        extra.push_str(&format!("({c})(SelfRT, SelfRT.lobby);\n"));
    }
    Ok(format!(
        "{RUNTIME_JS}\nSelfRT.onPrint = (s) => {{ if (typeof process !== 'undefined') process.stdout.write(s); else console.log(s); }};\n{}{extra}SelfRT.failures.length = 0;\nconst __last = ({prog})(SelfRT, SelfRT.lobby);\n",
        world_js()?
    ))
}

/// One HTML file that runs a Self program in any browser: the runtime, the
/// starting world, the program and a renderer for its morphs.
pub fn web_app(src: &str, title: &str) -> Result<String, String> {
    let prog = compile(src, false)?;
    let world = world_js()?;
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    let t = if title.trim().is_empty() { "A Self app" } else { title.trim() };
    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="generator" content="Self World (Self translated to JavaScript)">
<title>{title}</title>
</head>
<body>
<div id="self-app"></div>
<script>
{RUNTIME_JS}
</script>
<script>
{MORPHIC_JS}
</script>
<script>
SelfMorphic.install(document.getElementById('self-app'), {jt});
{world}SelfRT.failures.length = 0;
({prog})(SelfRT, SelfRT.lobby);
SelfMorphic.start();
</script>
</body>
</html>
"#,
        title = esc(t),
        jt = js_str(t),
    ))
}
