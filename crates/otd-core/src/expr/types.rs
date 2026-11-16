use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Str(String),
    Var(String),
    Neg(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Subscript(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

impl BinOp {
    pub(crate) fn precedence(self) -> u8 {
        match self {
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge | BinOp::Eq | BinOp::Ne => 1,
            BinOp::Add | BinOp::Sub => 2,
            BinOp::Mul | BinOp::Div | BinOp::Rem => 3,
            BinOp::Pow => 4,
        }
    }
    pub(crate) fn right_assoc(self) -> bool {
        self == BinOp::Pow
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprError(pub String);

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ExprError {}