//! Tokenizer for Self source text, following the lexical rules of the
//! Self Language Reference (identifiers, keywords, operators, numbers with
//! radix, strings with escapes, "double quoted" comments).

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    /// `foo:` — a keyword starting with a small letter (or `_`).
    SmallKw(String),
    /// `Foo:` — a continuation keyword.
    CapKw(String),
    /// `:x` — an argument slot name.
    Arg(String),
    /// `resend.` or `parentName.` directly followed by a message.
    Resend(String),
    Op(String),
    Int(i64),
    Float(f64),
    Str(String),
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Pipe,
    Dot,
    Caret,
    Arrow,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub msg: String,
    pub pos: usize,
}

fn is_op_char(c: char) -> bool {
    matches!(
        c,
        '!' | '@' | '#' | '$' | '%' | '^' | '&' | '*' | '-' | '+' | '=' | '~' | '/' | '?' | '<'
            | '>' | ',' | ';' | '|' | '\\' | '`'
    )
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    tokenize_with(src, false)
}

/// In file mode a line break outside any brackets ends the statement, as in
/// the original Self file reader (unless an argument is still expected).
pub fn tokenize_with(src: &str, file_mode: bool) -> Result<Vec<Token>, LexError> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let n = chars.len();
    let byte_at = |i: usize| if i < n { chars[i].0 } else { src.len() };
    let ch = |i: usize| if i < n { chars[i].1 } else { '\0' };
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0;
    let mut depth: i32 = 0;

    let prev_is_operand = |out: &Vec<Token>| {
        matches!(
            out.last().map(|t| &t.tok),
            Some(Tok::Ident(_))
                | Some(Tok::Int(_))
                | Some(Tok::Float(_))
                | Some(Tok::Str(_))
                | Some(Tok::RParen)
                | Some(Tok::RBrack)
        )
    };

    while i < n {
        let c = ch(i);
        let start = byte_at(i);
        if c.is_whitespace() {
            if file_mode && (c == '\n' || c == '\r') && depth <= 0 {
                let ends = match out.last().map(|t| &t.tok) {
                    None => false,
                    Some(Tok::Dot | Tok::SmallKw(_) | Tok::CapKw(_) | Tok::Op(_) | Tok::Arrow | Tok::Pipe | Tok::Caret
                        | Tok::Resend(_) | Tok::Arg(_)) => false,
                    Some(_) => true,
                };
                if ends {
                    out.push(Token { tok: Tok::Dot, start, end: start });
                }
            }
            i += 1;
            continue;
        }
        if c == '"' {
            i += 1;
            while i < n && ch(i) != '"' {
                i += 1;
            }
            if i >= n {
                return Err(LexError { msg: "unterminated comment".into(), pos: start });
            }
            i += 1;
            continue;
        }
        let push = |out: &mut Vec<Token>, tok: Tok, j: usize| {
            out.push(Token { tok, start, end: byte_at(j) });
        };
        match c {
            '(' => { depth += 1; i += 1; push(&mut out, Tok::LParen, i); continue; }
            ')' => { depth -= 1; i += 1; push(&mut out, Tok::RParen, i); continue; }
            '[' => { depth += 1; i += 1; push(&mut out, Tok::LBrack, i); continue; }
            ']' => { depth -= 1; i += 1; push(&mut out, Tok::RBrack, i); continue; }
            '{' => { depth += 1; i += 1; push(&mut out, Tok::LBrace, i); continue; }
            '}' => { depth -= 1; i += 1; push(&mut out, Tok::RBrace, i); continue; }
            '.' => { i += 1; push(&mut out, Tok::Dot, i); continue; }
            _ => {}
        }
        if c == '\'' {
            i += 1;
            let mut s = String::new();
            loop {
                if i >= n {
                    return Err(LexError { msg: "unterminated string".into(), pos: start });
                }
                let d = ch(i);
                if d == '\'' {
                    i += 1;
                    break;
                }
                if d == '\\' {
                    i += 1;
                    let e = ch(i);
                    i += 1;
                    match e {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        'b' => s.push('\u{8}'),
                        'f' => s.push('\u{c}'),
                        'v' => s.push('\u{b}'),
                        'a' => s.push('\u{7}'),
                        '0' => s.push('\0'),
                        'x' => {
                            let h: String = [ch(i), ch(i + 1)].iter().collect();
                            i += 2;
                            let v = u32::from_str_radix(&h, 16).unwrap_or(63);
                            s.push(char::from_u32(v).unwrap_or('?'));
                        }
                        'd' | 'o' => {
                            let h: String = [ch(i), ch(i + 1), ch(i + 2)].iter().collect();
                            i += 3;
                            let v = u32::from_str_radix(&h, if e == 'd' { 10 } else { 8 }).unwrap_or(63);
                            s.push(char::from_u32(v).unwrap_or('?'));
                        }
                        other => s.push(other),
                    }
                    continue;
                }
                s.push(d);
                i += 1;
            }
            push(&mut out, Tok::Str(s), i);
            continue;
        }
        // numbers (with optional leading minus when not following an operand)
        let neg_number = c == '-' && ch(i + 1).is_ascii_digit() && !prev_is_operand(&out);
        if c.is_ascii_digit() || neg_number {
            let mut j = i;
            let negative = neg_number;
            if negative {
                j += 1;
            }
            let ds = j;
            while ch(j).is_ascii_digit() {
                j += 1;
            }
            let dec: String = chars[ds..j].iter().map(|x| x.1).collect();
            // radix
            if (ch(j) == 'r' || ch(j) == 'R') && is_ident_char(ch(j + 1)) {
                let base: u32 = dec.parse().unwrap_or(10);
                let mut k = j + 1;
                let ks = k;
                while ch(k).is_ascii_alphanumeric() {
                    k += 1;
                }
                let digits: String = chars[ks..k].iter().map(|x| x.1).collect();
                if !(2..=36).contains(&base) {
                    return Err(LexError { msg: "bad number base".into(), pos: start });
                }
                let v = i64::from_str_radix(&digits, base)
                    .map_err(|_| LexError { msg: "bad digits for base".into(), pos: start })?;
                push(&mut out, Tok::Int(if negative { -v } else { v }), k);
                i = k;
                continue;
            }
            let mut is_float = false;
            let mut text = dec.clone();
            if ch(j) == '.' && ch(j + 1).is_ascii_digit() {
                is_float = true;
                j += 1;
                text.push('.');
                while ch(j).is_ascii_digit() {
                    text.push(ch(j));
                    j += 1;
                }
            }
            if (ch(j) == 'e' || ch(j) == 'E')
                && (ch(j + 1).is_ascii_digit()
                    || ((ch(j + 1) == '-' || ch(j + 1) == '+') && ch(j + 2).is_ascii_digit()))
            {
                is_float = true;
                text.push('e');
                j += 1;
                if ch(j) == '-' || ch(j) == '+' {
                    text.push(ch(j));
                    j += 1;
                }
                while ch(j).is_ascii_digit() {
                    text.push(ch(j));
                    j += 1;
                }
            }
            if is_float {
                let v: f64 = text.parse().unwrap_or(0.0);
                push(&mut out, Tok::Float(if negative { -v } else { v }), j);
            } else {
                let v: i64 = text
                    .parse()
                    .map_err(|_| LexError { msg: "integer too large".into(), pos: start })?;
                push(&mut out, Tok::Int(if negative { -v } else { v }), j);
            }
            i = j;
            continue;
        }
        if c == ':' {
            if ch(i + 1).is_ascii_lowercase() || ch(i + 1) == '_' {
                let mut j = i + 1;
                while is_ident_char(ch(j)) {
                    j += 1;
                }
                let name: String = chars[i + 1..j].iter().map(|x| x.1).collect();
                push(&mut out, Tok::Arg(name), j);
                i = j;
                continue;
            }
            return Err(LexError { msg: "unexpected ':'".into(), pos: start });
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let mut j = i;
            while is_ident_char(ch(j)) {
                j += 1;
            }
            let name: String = chars[i..j].iter().map(|x| x.1).collect();
            let small = !c.is_ascii_uppercase();
            if ch(j) == ':' && ch(j + 1) != '=' {
                let kw = format!("{name}:");
                push(&mut out, if small { Tok::SmallKw(kw) } else { Tok::CapKw(kw) }, j + 1);
                i = j + 1;
                continue;
            }
            // resend: `resend.foo` / `parent.foo:` with no whitespace
            if small
                && ch(j) == '.'
                && (ch(j + 1).is_ascii_lowercase() || ch(j + 1) == '_' || (is_op_char(ch(j + 1)) && ch(j + 1) != '|'))
            {
                push(&mut out, Tok::Resend(name), j + 1);
                i = j + 1;
                continue;
            }
            if !small {
                return Err(LexError {
                    msg: format!("identifiers must start with a small letter: {name}"),
                    pos: start,
                });
            }
            push(&mut out, Tok::Ident(name), j);
            i = j;
            continue;
        }
        if is_op_char(c) {
            let mut j = i;
            while is_op_char(ch(j)) {
                j += 1;
            }
            let op: String = chars[i..j].iter().map(|x| x.1).collect();
            let tok = match op.as_str() {
                "|" => Tok::Pipe,
                "^" => Tok::Caret,
                "<-" => Tok::Arrow,
                _ => Tok::Op(op),
            };
            push(&mut out, tok, j);
            i = j;
            continue;
        }
        return Err(LexError { msg: format!("unexpected character '{c}'"), pos: start });
    }
    out.push(Token { tok: Tok::Eof, start: src.len(), end: src.len() });
    Ok(out)
}
