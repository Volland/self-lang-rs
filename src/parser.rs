//! Parser for Self: object literals `( | slots | code )`, blocks
//! `[ | :args. locals | code ]`, unary / binary / keyword messages with
//! Self's precedence rules, resends, `^` returns and slot annotations.

use crate::lexer::{tokenize, tokenize_with, Tok, Token};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    SelfRef,
    /// An object literal without code (a data object).
    Object(Rc<ObjLit>),
    /// An object literal with code appearing as an expression: a
    /// parenthesised sub-expression, possibly with local slots.
    Paren(Rc<ObjLit>),
    Block(Rc<ObjLit>),
    /// A method object stored in a constant slot.
    Method(Rc<ObjLit>),
    Send {
        recv: Option<Box<Expr>>,
        sel: Rc<str>,
        args: Vec<Expr>,
        /// `None` for a normal send, `Some("resend")` for an undirected
        /// resend, `Some(parentName)` for a directed resend.
        resend: Option<Rc<str>>,
    },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Return(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    Constant,
    Assignable,
    Argument,
}

#[derive(Debug, Clone)]
pub struct SlotDesc {
    pub name: Rc<str>,
    pub kind: SlotKind,
    pub parent: bool,
    pub init: Option<Expr>,
    pub category: Rc<str>,
    pub comment: Option<Rc<str>>,
}

#[derive(Debug, Clone, Default)]
pub struct ObjLit {
    pub slots: Vec<SlotDesc>,
    /// `None` = data object (no code). `Some` = method / block code.
    pub code: Option<Vec<Stmt>>,
    pub source: Rc<str>,
    pub comment: Option<Rc<str>>,
    /// How many leading argument slots came from the slot header
    /// (`at: i Put: x = ( ... )`) rather than from the body.
    pub inline_args: usize,
}

impl ObjLit {
    pub fn arg_names(&self) -> Vec<Rc<str>> {
        self.slots
            .iter()
            .filter(|s| s.kind == SlotKind::Argument)
            .map(|s| s.name.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub msg: String,
    pub pos: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl ParseError {
    /// The message with a line and column, e.g. "line 3, column 7: expected ')'".
    pub fn describe_in(&self, src: &str) -> String {
        let (l, c) = line_col(src, self.pos);
        format!("line {l}, column {c}: {}", self.msg)
    }
}

pub struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
    /// Where each top-level statement starts (character offsets).
    starts: Vec<usize>,
}

type PResult<T> = Result<T, ParseError>;

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> PResult<Self> {
        let toks = tokenize(src).map_err(|e| ParseError { msg: e.msg, pos: e.pos })?;
        Ok(Parser { src, toks, pos: 0, starts: vec![] })
    }

    /// A parser for whole source files (line breaks end statements).
    pub fn new_file(src: &'a str) -> PResult<Self> {
        let toks = tokenize_with(src, true).map_err(|e| ParseError { msg: e.msg, pos: e.pos })?;
        Ok(Parser { src, toks, pos: 0, starts: vec![] })
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn here(&self) -> usize {
        self.toks[self.pos].start
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn err<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(ParseError { msg: msg.into(), pos: self.here() })
    }
    fn expect(&mut self, t: Tok, what: &str) -> PResult<()> {
        if *self.peek() == t {
            self.next();
            Ok(())
        } else {
            self.err(format!("expected {what}, found {}", describe(self.peek())))
        }
    }

    /// Parse a whole "do it" text: optional `| locals |` then statements.
    pub fn parse_doit(&mut self) -> PResult<ObjLit> {
        let start = self.here();
        let mut lit = ObjLit::default();
        if *self.peek() == Tok::Pipe {
            self.next();
            lit.slots = self.parse_slot_list(Tok::Pipe, "".into())?;
            self.expect(Tok::Pipe, "'|'")?;
        }
        lit.code = Some(self.parse_code(&[Tok::Eof])?);
        if *self.peek() != Tok::Eof {
            return self.err(format!("unexpected {}", describe(self.peek())));
        }
        lit.source = self.src[start..].into();
        Ok(lit)
    }

    /// Parse a bare slot list such as `x <- 3. foo = ( ... )` (used by the
    /// outliner's "add slot" editor).
    pub fn parse_slots_only(&mut self) -> PResult<Vec<SlotDesc>> {
        let slots = self.parse_slot_list(Tok::Eof, "".into())?;
        if *self.peek() != Tok::Eof {
            return self.err(format!("unexpected {}", describe(self.peek())));
        }
        Ok(slots)
    }

    fn parse_code(&mut self, terminators: &[Tok]) -> PResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            while *self.peek() == Tok::Dot {
                self.next();
            }
            if terminators.contains(self.peek()) {
                break;
            }
            if terminators == [Tok::Eof] {
                self.starts.push(self.here());
            }
            if *self.peek() == Tok::Caret {
                self.next();
                let e = self.parse_expr()?;
                stmts.push(Stmt::Return(e));
                while *self.peek() == Tok::Dot {
                    self.next();
                }
                if !terminators.contains(self.peek()) {
                    return self.err("a '^' return must be the last statement");
                }
                break;
            }
            let e = self.parse_expr()?;
            stmts.push(Stmt::Expr(e));
            if *self.peek() == Tok::Dot {
                continue;
            }
            if terminators.contains(self.peek()) {
                break;
            }
            return self.err(format!("expected '.' or end of code, found {}", describe(self.peek())));
        }
        Ok(stmts)
    }

    fn parse_slot_list(&mut self, end: Tok, category: Rc<str>) -> PResult<Vec<SlotDesc>> {
        let mut slots = Vec::new();
        loop {
            while *self.peek() == Tok::Dot {
                self.next();
            }
            if *self.peek() == end || *self.peek() == Tok::Eof {
                break;
            }
            if *self.peek() == Tok::LBrace {
                self.next();
                // `{} = 'comment'` object annotation
                if *self.peek() == Tok::RBrace {
                    self.next();
                    if matches!(self.peek(), Tok::Op(o) if o == "=") {
                        self.next();
                        if let Tok::Str(s) = self.next() {
                            slots.push(SlotDesc {
                                name: "{}".into(),
                                kind: SlotKind::Constant,
                                parent: false,
                                init: Some(Expr::Str(s.into())),
                                category: category.clone(),
                                comment: None,
                            });
                            continue;
                        }
                        return self.err("expected a string after '{} ='");
                    }
                    continue;
                }
                let mut cat = category.clone();
                if let Tok::Str(s) = self.peek().clone() {
                    self.next();
                    cat = parse_category(&s).unwrap_or(cat);
                }
                let inner = self.parse_slot_list(Tok::RBrace, cat)?;
                self.expect(Tok::RBrace, "'}'")?;
                slots.extend(inner);
                continue;
            }
            slots.extend(self.parse_slot(category.clone())?);
            match self.peek() {
                Tok::Dot => continue,
                t if *t == end => break,
                Tok::RBrace => break,
                _ => {
                    return self.err(format!("expected '.' between slots, found {}", describe(self.peek())))
                }
            }
        }
        Ok(slots)
    }

    fn parse_slot(&mut self, category: Rc<str>) -> PResult<Vec<SlotDesc>> {
        let mk = |name: &str, kind, parent, init| SlotDesc {
            name: name.into(),
            kind,
            parent,
            init,
            category: category.clone(),
            comment: None,
        };
        match self.peek().clone() {
            Tok::Arg(name) => {
                self.next();
                Ok(vec![mk(&name, SlotKind::Argument, false, None)])
            }
            Tok::Ident(name) => {
                self.next();
                let mut parent = false;
                if matches!(self.peek(), Tok::Op(o) if o == "*") {
                    self.next();
                    parent = true;
                }
                if matches!(self.peek(), Tok::Op(o) if o == "*=") {
                    self.next();
                    let init = self.parse_slot_init(&[])?;
                    return Ok(vec![mk(&name, SlotKind::Constant, true, Some(init))]);
                }
                match self.peek().clone() {
                    Tok::Op(o) if o == "=" => {
                        self.next();
                        let init = self.parse_slot_init(&[])?;
                        Ok(vec![mk(&name, SlotKind::Constant, parent, Some(init))])
                    }
                    Tok::Arrow => {
                        self.next();
                        let init = self.parse_expr()?;
                        Ok(vec![mk(&name, SlotKind::Assignable, parent, Some(init))])
                    }
                    _ => Ok(vec![mk(&name, SlotKind::Assignable, parent, None)]),
                }
            }
            Tok::Op(op) => {
                // binary method slot: `+ arg = ( ... )` or `+ = (| :arg | ...)`
                self.next();
                let mut args = vec![];
                if let Tok::Ident(a) = self.peek().clone() {
                    self.next();
                    args.push(a);
                }
                if !matches!(self.peek(), Tok::Op(o) if o == "=") {
                    return self.err("expected '=' in binary slot");
                }
                self.next();
                let init = self.parse_slot_init(&args)?;
                Ok(vec![mk(&op, SlotKind::Constant, false, Some(init))])
            }
            Tok::SmallKw(first) => {
                self.next();
                let mut sel = first;
                let mut args = vec![];
                if let Tok::Ident(a) = self.peek().clone() {
                    self.next();
                    args.push(a);
                    while let Tok::CapKw(k) = self.peek().clone() {
                        self.next();
                        sel.push_str(&k);
                        match self.next() {
                            Tok::Ident(a) => args.push(a),
                            _ => return self.err("expected an argument name after keyword"),
                        }
                    }
                } else {
                    while let Tok::CapKw(k) = self.peek().clone() {
                        self.next();
                        sel.push_str(&k);
                    }
                }
                if !matches!(self.peek(), Tok::Op(o) if o == "=") {
                    return self.err("expected '=' in keyword slot");
                }
                self.next();
                let init = self.parse_slot_init(&args)?;
                Ok(vec![mk(&sel, SlotKind::Constant, false, Some(init))])
            }
            other => self.err(format!("expected a slot, found {}", describe(&other))),
        }
    }

    /// The value of a constant slot. A parenthesised object with code
    /// becomes a method (with the given inline argument names prepended).
    fn parse_slot_init(&mut self, inline_args: &[String]) -> PResult<Expr> {
        if *self.peek() == Tok::LParen {
            let save = self.pos;
            let lit = self.parse_object_lit(Tok::LParen, Tok::RParen)?;
            if lit.code.is_some() {
                let mut lit = lit;
                let mut slots: Vec<SlotDesc> = inline_args
                    .iter()
                    .map(|a| SlotDesc {
                        name: a.as_str().into(),
                        kind: SlotKind::Argument,
                        parent: false,
                        init: None,
                        category: "".into(),
                        comment: None,
                    })
                    .collect();
                slots.extend(lit.slots.drain(..));
                lit.slots = slots;
                lit.inline_args = inline_args.len();
                return Ok(Expr::Method(Rc::new(lit)));
            }
            if !inline_args.is_empty() {
                return self.err("a slot with arguments needs a method body");
            }
            // data object: continue parsing as an expression (it may receive messages)
            self.pos = save;
        } else if !inline_args.is_empty() {
            return self.err("a slot with arguments needs a method body '( ... )'");
        }
        self.parse_expr()
    }

    fn parse_object_lit(&mut self, open: Tok, close: Tok) -> PResult<ObjLit> {
        let start = self.here();
        self.expect(open, "an opening bracket")?;
        let mut lit = ObjLit::default();
        if *self.peek() == Tok::Pipe {
            self.next();
            lit.slots = self.parse_slot_list(Tok::Pipe, "".into())?;
            self.expect(Tok::Pipe, "'|' closing the slot list")?;
        } else if matches!(self.peek(), Tok::Op(o) if o == "||") {
            self.next();
        }
        if let Some(pos) = lit.slots.iter().position(|s| &*s.name == "{}") {
            let s = lit.slots.remove(pos);
            if let Some(Expr::Str(c)) = s.init {
                lit.comment = Some(c);
            }
        }
        let is_block = close == Tok::RBrack;
        if *self.peek() == close {
            lit.code = if is_block { Some(vec![]) } else { None };
        } else {
            lit.code = Some(self.parse_code(&[close.clone()])?);
        }
        let end = self.toks[self.pos].end;
        self.expect(close, "a closing bracket")?;
        lit.source = self.src[start..end].into();
        Ok(lit)
    }

    pub fn parse_expr(&mut self) -> PResult<Expr> {
        let recv = self.parse_binary_expr()?;
        self.parse_keyword_tail(recv)
    }

    fn parse_keyword_tail(&mut self, recv: Option<Expr>) -> PResult<Expr> {
        if let Tok::SmallKw(k) = self.peek().clone() {
            self.next();
            let mut sel = k;
            let mut args = vec![self.parse_keyword_arg()?];
            while let Tok::CapKw(k) = self.peek().clone() {
                self.next();
                sel.push_str(&k);
                args.push(self.parse_keyword_arg()?);
            }
            return Ok(Expr::Send { recv: recv.map(Box::new), sel: sel.into(), args, resend: None });
        }
        match recv {
            Some(e) => Ok(e),
            None => self.err(format!("expected an expression, found {}", describe(self.peek()))),
        }
    }

    /// Keyword arguments are binary expressions; a following small keyword
    /// starts a new (right-associative) keyword message.
    fn parse_keyword_arg(&mut self) -> PResult<Expr> {
        // an implicit-receiver keyword message may itself be an argument:
        // `r: copySize: 3` means `r: (copySize: 3)`
        if matches!(self.peek(), Tok::SmallKw(_)) {
            return self.parse_keyword_tail(None);
        }
        let e = self.parse_binary_expr()?;
        if e.is_none() {
            return self.err(format!("expected an argument, found {}", describe(self.peek())));
        }
        if matches!(self.peek(), Tok::SmallKw(_)) {
            return self.parse_keyword_tail(e);
        }
        Ok(e.unwrap())
    }

    /// Returns `None` if no receiver is present (implicit receiver keyword send).
    fn parse_binary_expr(&mut self) -> PResult<Option<Expr>> {
        let mut left = self.parse_unary_expr()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) => o.clone(),
                _ => break,
            };
            self.next();
            let right = match self.parse_unary_expr()? {
                Some(r) => r,
                // `a + and: b With: c` — an implicit-receiver keyword message as the argument
                None if matches!(self.peek(), Tok::SmallKw(_)) => self.parse_keyword_tail(None)?,
                None => return self.err(format!("expected an argument for '{op}'")),
            };
            left = Some(Expr::Send { recv: left.map(Box::new), sel: op.into(), args: vec![right], resend: None });
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> PResult<Option<Expr>> {
        let mut e = match self.peek().clone() {
            Tok::Resend(target) => {
                self.next();
                let t: Rc<str> = target.into();
                match self.peek().clone() {
                    Tok::Ident(n) => {
                        self.next();
                        Some(Expr::Send { recv: None, sel: n.into(), args: vec![], resend: Some(t) })
                    }
                    Tok::Op(op) => {
                        self.next();
                        let arg = match self.parse_unary_expr()? {
                            Some(a) => a,
                            None => return self.err("expected an argument for the resent operator"),
                        };
                        Some(Expr::Send { recv: None, sel: op.into(), args: vec![arg], resend: Some(t) })
                    }
                    Tok::SmallKw(_) => {
                        let ex = self.parse_keyword_tail(None)?;
                        if let Expr::Send { recv, sel, args, .. } = ex {
                            Some(Expr::Send { recv, sel, args, resend: Some(t) })
                        } else {
                            unreachable!()
                        }
                    }
                    _ => return self.err("expected a message after resend"),
                }
            }
            _ => self.parse_primary()?,
        };
        loop {
            match self.peek().clone() {
                Tok::Ident(name) => {
                    self.next();
                    e = Some(Expr::Send { recv: e.map(Box::new), sel: name.into(), args: vec![], resend: None });
                }
                _ => break,
            }
            // a resend name may only start an expression
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> PResult<Option<Expr>> {
        let e = match self.peek().clone() {
            Tok::Int(v) => {
                self.next();
                Expr::Int(v)
            }
            Tok::Float(v) => {
                self.next();
                Expr::Float(v)
            }
            Tok::Str(s) => {
                self.next();
                Expr::Str(s.into())
            }
            Tok::Ident(n) if n == "self" => {
                self.next();
                Expr::SelfRef
            }
            Tok::Ident(_) => return Ok(None),
            Tok::LParen => {
                let lit = self.parse_object_lit(Tok::LParen, Tok::RParen)?;
                if lit.code.is_some() {
                    Expr::Paren(Rc::new(lit))
                } else {
                    Expr::Object(Rc::new(lit))
                }
            }
            Tok::LBrack => {
                let lit = self.parse_object_lit(Tok::LBrack, Tok::RBrack)?;
                Expr::Block(Rc::new(lit))
            }
            _ => return Ok(None),
        };
        Ok(Some(e))
    }
}

fn parse_category(annotation: &str) -> Option<Rc<str>> {
    for part in annotation.split('\u{7f}') {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("Category:") {
            return Some(rest.trim().into());
        }
    }
    let t = annotation.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.into())
    }
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("'{s}'"),
        Tok::SmallKw(s) | Tok::CapKw(s) => format!("keyword '{s}'"),
        Tok::Arg(s) => format!("argument ':{s}'"),
        Tok::Resend(s) => format!("'{s}.'"),
        Tok::Op(s) => format!("operator '{s}'"),
        Tok::Int(v) => format!("number {v}"),
        Tok::Float(v) => format!("number {v}"),
        Tok::Str(_) => "a string".into(),
        Tok::LParen => "'('".into(),
        Tok::RParen => "')'".into(),
        Tok::LBrack => "'['".into(),
        Tok::RBrack => "']'".into(),
        Tok::LBrace => "'{'".into(),
        Tok::RBrace => "'}'".into(),
        Tok::Pipe => "'|'".into(),
        Tok::Dot => "'.'".into(),
        Tok::Caret => "'^'".into(),
        Tok::Arrow => "'<-'".into(),
        Tok::Eof => "end of text".into(),
    }
}

/// Is the selector a keyword selector made of capitalised parts after the first?
pub fn arity_of(sel: &str) -> usize {
    if sel.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
        sel.matches(':').count()
    } else {
        1
    }
}

pub fn parse_doit(src: &str) -> Result<ObjLit, ParseError> {
    Parser::new(src)?.parse_doit()
}

/// Parse a source file; also returns where each top-level statement starts.
pub fn parse_file(src: &str) -> Result<(ObjLit, Vec<usize>), ParseError> {
    let mut p = Parser::new_file(src)?;
    let lit = p.parse_doit()?;
    Ok((lit, p.starts))
}

/// 1-based line and column of a character offset.
pub fn line_col(src: &str, pos: usize) -> (usize, usize) {
    let pos = pos.min(src.len());
    let before = &src[..(0..=pos).rev().find(|i| src.is_char_boundary(*i)).unwrap_or(0)];
    let line = before.matches('\n').count() + 1;
    let col = before.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0) + 1;
    (line, col)
}

pub fn parse_slots(src: &str) -> Result<Vec<SlotDesc>, ParseError> {
    Parser::new(src)?.parse_slots_only()
}

