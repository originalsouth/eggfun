//! Hand-rolled tokenizer + Pratt parser for the calculator REPL.
//!
//! Grammar (highest precedence last in the table is bound tightest):
//!
//!   or-expr   :=  xor-expr ( '|' xor-expr )*
//!   xor-expr  :=  and-expr ( 'xor' and-expr )*
//!   and-expr  :=  not-expr ( '&' not-expr )*
//!   not-expr  :=  '!' not-expr | add-expr
//!   add-expr  :=  mul-expr ( ('+'|'-') mul-expr )*
//!   mul-expr  :=  unary    ( ('*'|'/') unary    )*
//!   unary     :=  '-' unary | pow
//!   pow       :=  atom ( '^' unary )?              // right-assoc
//!   atom      :=  number | ident ( '(' or-expr ')' )? | '(' or-expr ')'
//!
//! Boolean keyword forms (`and`, `or`, `not`, `xor`) are also accepted as
//! aliases for `&`, `|`, `!`, `xor`. `true` / `false` are literals.

use anyhow::{anyhow, bail, Result};
use egg::{Id, RecExpr};
use ordered_float::OrderedFloat;

use crate::lang::Calc;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Amp,
    Pipe,
    Bang,
    Eof,
}

fn tokenize(src: &str) -> Result<Vec<Tok>> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '+' => { out.push(Tok::Plus);   i += 1; }
            '-' => { out.push(Tok::Minus);  i += 1; }
            '*' => { out.push(Tok::Star);   i += 1; }
            '/' => { out.push(Tok::Slash);  i += 1; }
            '^' => { out.push(Tok::Caret);  i += 1; }
            '(' => { out.push(Tok::LParen); i += 1; }
            ')' => { out.push(Tok::RParen); i += 1; }
            '&' => {
                // accept `&&` as `&`
                i += 1;
                if i < bytes.len() && bytes[i] as char == '&' { i += 1; }
                out.push(Tok::Amp);
            }
            '|' => {
                i += 1;
                if i < bytes.len() && bytes[i] as char == '|' { i += 1; }
                out.push(Tok::Pipe);
            }
            '!' => { out.push(Tok::Bang);   i += 1; }
            '0'..='9' | '.' => {
                let start = i;
                let mut seen_dot = false;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_digit() {
                        i += 1;
                    } else if ch == '.' && !seen_dot {
                        seen_dot = true;
                        i += 1;
                    } else if (ch == 'e' || ch == 'E')
                        && i + 1 < bytes.len()
                    {
                        // optional sign after exponent marker
                        i += 1;
                        if matches!(bytes[i] as char, '+' | '-') { i += 1; }
                    } else {
                        break;
                    }
                }
                let s = std::str::from_utf8(&bytes[start..i]).unwrap();
                let n: f64 = s
                    .parse()
                    .map_err(|e| anyhow!("bad number `{}`: {}", s, e))?;
                out.push(Tok::Num(n));
            }
            ch if ch.is_alphabetic() || ch == '_' => {
                let start = i;
                while i < bytes.len() {
                    let c2 = bytes[i] as char;
                    if c2.is_alphanumeric() || c2 == '_' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let id = std::str::from_utf8(&bytes[start..i]).unwrap().to_string();
                out.push(Tok::Ident(id));
            }
            _ => bail!("unexpected character `{}`", c),
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    expr: RecExpr<Calc>,
}

impl Parser {
    fn new(toks: Vec<Tok>) -> Self {
        Self { toks, pos: 0, expr: RecExpr::default() }
    }
    fn peek(&self) -> &Tok { &self.toks[self.pos] }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        self.pos += 1;
        t
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == t { self.pos += 1; true } else { false }
    }
    fn add(&mut self, node: Calc) -> Id { self.expr.add(node) }

    fn ident_is_kw(s: &str, kw: &str) -> bool { s.eq_ignore_ascii_case(kw) }

    fn parse_or(&mut self) -> Result<Id> {
        let mut lhs = self.parse_xor()?;
        loop {
            let is_pipe = matches!(self.peek(), Tok::Pipe);
            let is_kw_or = matches!(self.peek(), Tok::Ident(s) if Self::ident_is_kw(s, "or"));
            if !(is_pipe || is_kw_or) { break; }
            self.bump();
            let rhs = self.parse_xor()?;
            lhs = self.add(Calc::Or([lhs, rhs]));
        }
        Ok(lhs)
    }

    fn parse_xor(&mut self) -> Result<Id> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), Tok::Ident(s) if Self::ident_is_kw(s, "xor")) {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = self.add(Calc::Xor([lhs, rhs]));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Id> {
        let mut lhs = self.parse_not()?;
        loop {
            let is_amp = matches!(self.peek(), Tok::Amp);
            let is_kw_and = matches!(self.peek(), Tok::Ident(s) if Self::ident_is_kw(s, "and"));
            if !(is_amp || is_kw_and) { break; }
            self.bump();
            let rhs = self.parse_not()?;
            lhs = self.add(Calc::And([lhs, rhs]));
        }
        Ok(lhs)
    }

    fn parse_not(&mut self) -> Result<Id> {
        let is_bang = matches!(self.peek(), Tok::Bang);
        let is_kw_not = matches!(self.peek(), Tok::Ident(s) if Self::ident_is_kw(s, "not"));
        if is_bang || is_kw_not {
            self.bump();
            let rhs = self.parse_not()?;
            Ok(self.add(Calc::Not([rhs])))
        } else {
            self.parse_add()
        }
    }

    fn parse_add(&mut self) -> Result<Id> {
        let mut lhs = self.parse_mul()?;
        loop {
            match self.peek() {
                Tok::Plus  => { self.bump(); let r = self.parse_mul()?; lhs = self.add(Calc::Add([lhs, r])); }
                Tok::Minus => { self.bump(); let r = self.parse_mul()?; lhs = self.add(Calc::Sub([lhs, r])); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Id> {
        let mut lhs = self.parse_unary()?;
        loop {
            match self.peek() {
                Tok::Star  => { self.bump(); let r = self.parse_unary()?; lhs = self.add(Calc::Mul([lhs, r])); }
                Tok::Slash => { self.bump(); let r = self.parse_unary()?; lhs = self.add(Calc::Div([lhs, r])); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Id> {
        if matches!(self.peek(), Tok::Minus) {
            self.bump();
            let rhs = self.parse_unary()?;
            Ok(self.add(Calc::Neg([rhs])))
        } else {
            self.parse_pow()
        }
    }

    fn parse_pow(&mut self) -> Result<Id> {
        let lhs = self.parse_atom()?;
        if matches!(self.peek(), Tok::Caret) {
            self.bump();
            // right-associative — recurse into unary so `2^-3` parses
            let rhs = self.parse_unary()?;
            Ok(self.add(Calc::Pow([lhs, rhs])))
        } else {
            Ok(lhs)
        }
    }

    fn parse_atom(&mut self) -> Result<Id> {
        match self.bump() {
            Tok::Num(n) => Ok(self.add(Calc::Num(OrderedFloat(n)))),
            Tok::LParen => {
                let inner = self.parse_or()?;
                if !self.eat(&Tok::RParen) {
                    bail!("expected `)`");
                }
                Ok(inner)
            }
            Tok::Ident(name) => {
                // function call?
                if matches!(self.peek(), Tok::LParen) {
                    self.bump();
                    let arg = self.parse_or()?;
                    if !self.eat(&Tok::RParen) {
                        bail!("expected `)` after `{}(...`", name);
                    }
                    self.build_call(&name, arg)
                } else {
                    Ok(self.build_ident(&name))
                }
            }
            t => bail!("unexpected token {:?}", t),
        }
    }

    fn build_ident(&mut self, name: &str) -> Id {
        match name.to_ascii_lowercase().as_str() {
            "true"  => self.add(Calc::True),
            "false" => self.add(Calc::False),
            _ => self.add(Calc::Symbol(name.into())),
        }
    }

    fn build_call(&mut self, name: &str, arg: Id) -> Result<Id> {
        let node = match name.to_ascii_lowercase().as_str() {
            "sin"  => Calc::Sin([arg]),
            "cos"  => Calc::Cos([arg]),
            "tan"  => Calc::Tan([arg]),
            "log"  => Calc::Log([arg]),
            "ln"   => Calc::Ln([arg]),
            "exp"  => Calc::Exp([arg]),
            "sqrt" => Calc::Sqrt([arg]),
            "abs"  => Calc::Abs([arg]),
            "not"  => Calc::Not([arg]),
            _ => bail!("unknown function `{}`", name),
        };
        Ok(self.add(node))
    }
}

pub fn parse(src: &str) -> Result<RecExpr<Calc>> {
    let toks = tokenize(src)?;
    let mut p = Parser::new(toks);
    let _root = p.parse_or()?;
    if !matches!(p.peek(), Tok::Eof) {
        bail!("trailing input: {:?}", p.peek());
    }
    Ok(p.expr)
}
