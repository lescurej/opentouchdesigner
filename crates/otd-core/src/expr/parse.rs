use super::types::{BinOp, Expr, ExprError};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(BinOp),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Minus,
}

pub(crate) fn tokenize(src: &str) -> Result<Vec<Tok>, ExprError> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            '[' => {
                out.push(Tok::LBracket);
                i += 1;
            }
            ']' => {
                out.push(Tok::RBracket);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '+' => {
                out.push(Tok::Op(BinOp::Add));
                i += 1;
            }
            '-' => {
                out.push(Tok::Minus);
                i += 1;
            }
            '*' => {
                if chars.get(i + 1) == Some(&'*') {
                    out.push(Tok::Op(BinOp::Pow));
                    i += 2;
                } else {
                    out.push(Tok::Op(BinOp::Mul));
                    i += 1;
                }
            }
            '\'' | '"' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(ExprError("unterminated string".into()));
                }
                let s: String = chars[start..i].iter().collect();
                out.push(Tok::Str(s));
                i += 1;
            }
            '/' => {
                out.push(Tok::Op(BinOp::Div));
                i += 1;
            }
            '%' => {
                out.push(Tok::Op(BinOp::Rem));
                i += 1;
            }
            '^' => {
                out.push(Tok::Op(BinOp::Pow));
                i += 1;
            }
            '<' | '>' | '=' | '!' => {
                let two = chars.get(i + 1) == Some(&'=');
                let op = match (c, two) {
                    ('<', false) => BinOp::Lt,
                    ('<', true) => BinOp::Le,
                    ('>', false) => BinOp::Gt,
                    ('>', true) => BinOp::Ge,
                    ('=', true) => BinOp::Eq,
                    ('!', true) => BinOp::Ne,
                    _ => return Err(ExprError(format!("stray `{c}`"))),
                };
                out.push(Tok::Op(op));
                i += if two { 2 } else { 1 };
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                    let save = i;
                    i += 1;
                    if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                        i += 1;
                    }
                    if i < chars.len() && chars[i].is_ascii_digit() {
                        while i < chars.len() && chars[i].is_ascii_digit() {
                            i += 1;
                        }
                    } else {
                        i = save;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                out.push(Tok::Num(
                    s.parse()
                        .map_err(|_| ExprError(format!("bad number literal `{s}`")))?,
                ));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                out.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            other => return Err(ExprError(format!("unexpected character `{other}`"))),
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, ExprError> {
        let mut lhs = self.parse_atom()?;
        while let Some(op) = self.peek().and_then(|t| match t {
            Tok::Op(op) => Some(*op),
            Tok::Minus => Some(BinOp::Sub),
            _ => None,
        }) {
            if op.precedence() < min_prec {
                break;
            }
            self.pos += 1;
            let next_min = if op.right_assoc() {
                op.precedence()
            } else {
                op.precedence() + 1
            };
            let rhs = self.parse_expr(next_min)?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_atom(&mut self) -> Result<Expr, ExprError> {
        let atom = match self.tokens.get(self.pos).cloned() {
            Some(Tok::Minus) => {
                self.pos += 1;
                Expr::Neg(Box::new(self.parse_expr(4)?))
            }
            Some(Tok::Num(v)) => {
                self.pos += 1;
                Expr::Num(v)
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Expr::Str(s)
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                let e = self.parse_expr(0)?;
                self.expect(Tok::RParen)?;
                e
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if self.peek() == Some(&Tok::LParen) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.parse_expr(0)?);
                            if self.peek() == Some(&Tok::Comma) {
                                self.pos += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            other => {
                return Err(ExprError(format!(
                    "expected a value, found {}",
                    match other {
                        None => "end of input".to_string(),
                        Some(t) => format!("{t:?}"),
                    }
                )));
            }
        };
        self.parse_suffix(atom)
    }

    fn parse_suffix(&mut self, mut atom: Expr) -> Result<Expr, ExprError> {
        while self.peek() == Some(&Tok::LBracket) {
            self.pos += 1;
            let key = self.parse_expr(0)?;
            self.expect(Tok::RBracket)?;
            atom = Expr::Subscript(Box::new(atom), Box::new(key));
        }
        Ok(atom)
    }

    fn expect(&mut self, t: Tok) -> Result<(), ExprError> {
        if self.peek() == Some(&t) {
            self.pos += 1;
            Ok(())
        } else {
            Err(ExprError(format!("expected {t:?}")))
        }
    }
}

pub(crate) fn parse_tokens(tokens: Vec<Tok>) -> Result<Expr, ExprError> {
    let mut p = Parser { tokens, pos: 0 };
    let e = p.parse_expr(0)?;
    if p.pos != p.tokens.len() {
        return Err(ExprError(format!(
            "unexpected trailing input at token {}",
            p.pos
        )));
    }
    Ok(e)
}

pub(crate) fn parse_src(src: &str) -> Result<Expr, ExprError> {
    let tokens = tokenize(src)?;
    parse_tokens(tokens)
}