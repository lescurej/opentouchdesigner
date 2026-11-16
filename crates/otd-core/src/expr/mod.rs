//! A tiny numeric expression language for parameters in Expression mode.
//!
//! PLAN.md commits to embedded CPython for the real expression engine
//! (Phase 3). That's a heavy dependency to carry through Phase 0/1, and the
//! Phase 1 demo needs *something* live behind the Expression mode, so this
//! module implements a self-contained arithmetic evaluator with the same
//! variable names Python mode will expose (`absTime`, `me.time`, `frame`).
//!
//! When PyO3 lands, this stays as the fallback used by headless builds
//! compiled without the `python` feature — expressions written against it
//! remain valid Python expressions, so projects keep loading.

mod builtins;
mod context;
mod eval;
mod parse;
mod types;

pub use context::{ChannelSource, EvalContext};
pub use types::{BinOp, Expr, ExprError};

/// Variable names that make an expression re-cook every frame.
const TIME_VARS: [&str; 5] = ["absTime", "time", "frame", "me.time", "absTime.seconds"];

impl Expr {
    pub fn parse(src: &str) -> Result<Expr, ExprError> {
        parse::parse_src(src)
    }

    pub fn is_time_dependent(&self) -> bool {
        match self {
            Expr::Num(_) | Expr::Str(_) => false,
            Expr::Var(name) => TIME_VARS.contains(&name.as_str()),
            Expr::Neg(e) => e.is_time_dependent(),
            Expr::Bin(_, a, b) => a.is_time_dependent() || b.is_time_dependent(),
            Expr::Call(name, args) => {
                name == "ch"
                    || name == "par"
                    || name == "op"
                    || args.iter().any(|a| a.is_time_dependent())
            }
            Expr::Subscript(a, b) => a.is_time_dependent() || b.is_time_dependent(),
        }
    }

    pub fn referenced_paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_referenced_paths(&mut out);
        out
    }

    fn collect_referenced_paths(&self, out: &mut Vec<String>) {
        match self {
            Expr::Subscript(a, b) => {
                a.collect_referenced_paths(out);
                b.collect_referenced_paths(out);
            }
            Expr::Call(name, args) if matches!(name.as_str(), "ch" | "par" | "op") => {
                if let Some(Expr::Str(p)) = args.first() {
                    let p = p.trim();
                    if !p.is_empty() && !out.iter().any(|x| x == p) {
                        out.push(p.to_string());
                    }
                }
                for a in args {
                    a.collect_referenced_paths(out);
                }
            }
            Expr::Neg(e) => e.collect_referenced_paths(out),
            Expr::Bin(_, a, b) => {
                a.collect_referenced_paths(out);
                b.collect_referenced_paths(out);
            }
            Expr::Call(_, args) => {
                for a in args {
                    a.collect_referenced_paths(out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "../tests/expr__tests.rs"]
mod tests;