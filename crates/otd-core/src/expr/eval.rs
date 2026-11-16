use super::builtins::{bool_f64, eval_call, eval_var};
use super::context::EvalContext;
use super::types::{BinOp, Expr, ExprError};

pub(crate) fn expr_string(e: &Expr) -> Result<&str, ExprError> {
    match e {
        Expr::Str(s) => Ok(s.as_str()),
        _ => Err(ExprError("expected a string literal".into())),
    }
}

pub(crate) fn resolve_op_path(op: &str, ctx: &EvalContext) -> String {
    let op = op.trim();
    if op.starts_with('/') {
        return op.to_string();
    }
    let Some(path) = ctx.path else {
        return format!("/{op}");
    };
    let base = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    if base.is_empty() {
        format!("/{op}")
    } else {
        format!("{base}/{op}")
    }
}

pub(crate) fn eval_subscript(base: &Expr, key: &Expr, ctx: &EvalContext) -> Result<f64, ExprError> {
    let Expr::Call(name, args) = base else {
        return Err(ExprError("subscript is only supported on op(...)".into()));
    };
    if name != "op" || args.len() != 1 {
        return Err(ExprError("subscript is only supported on op(...)".into()));
    }
    let op = resolve_op_path(expr_string(&args[0])?, ctx);
    let chan = expr_string(key)?;
    let Some(net) = ctx.channels else {
        return Err(ExprError("op()['chan'] needs a cooked network".into()));
    };
    net.channel(&op, chan)
        .map(f64::from)
        .ok_or_else(|| ExprError(format!("no channel `{chan}` on `{op}`")))
}

pub(crate) fn eval_call_expr(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<f64, ExprError> {
    if name == "ch" {
        if args.len() != 2 {
            return Err(ExprError("ch() takes 2 arguments".into()));
        }
        let op = resolve_op_path(expr_string(&args[0])?, ctx);
        let chan = expr_string(&args[1])?;
        let Some(net) = ctx.channels else {
            return Err(ExprError("ch() needs a cooked network".into()));
        };
        return net
            .channel(&op, chan)
            .map(f64::from)
            .ok_or_else(|| ExprError(format!("no channel `{chan}` on `{op}`")));
    }
    if name == "op" {
        return Err(ExprError("op(...) must be followed by ['channel']".into()));
    }
    if name == "par" {
        if args.len() != 2 {
            return Err(ExprError("par() takes 2 arguments".into()));
        }
        let op = resolve_op_path(expr_string(&args[0])?, ctx);
        let param = expr_string(&args[1])?;
        let Some(net) = ctx.channels else {
            return Err(ExprError("par() needs a cooked network".into()));
        };
        return net
            .param_value(&op, param)
            .map(|v| v.as_f64())
            .ok_or_else(|| ExprError(format!("no parameter `{param}` on `{op}`")));
    }
    let a: Result<Vec<f64>, ExprError> = args.iter().map(|e| e.eval(ctx)).collect();
    eval_call(name, &a?)
}

impl Expr {
    pub fn eval(&self, ctx: &EvalContext) -> Result<f64, ExprError> {
        match self {
            Expr::Num(v) => Ok(*v),
            Expr::Str(_) => Err(ExprError("string literal is not a number".into())),
            Expr::Var(name) => {
                if let Some(param) = name.strip_prefix("parent.") {
                    if let (Some(net), Some(path)) = (ctx.channels, ctx.path) {
                        if let Some(v) = net.parent_param(path, param) {
                            return Ok(v.as_f64());
                        }
                    }
                    return Err(ExprError(format!(
                        "no parameter `{param}` on the parent component"
                    )));
                }
                eval_var(name, ctx)
            }
            Expr::Neg(e) => Ok(-e.eval(ctx)?),
            Expr::Bin(op, a, b) => {
                let (a, b) = (a.eval(ctx)?, b.eval(ctx)?);
                Ok(match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                    BinOp::Rem => a % b,
                    BinOp::Pow => a.powf(b),
                    BinOp::Lt => bool_f64(a < b),
                    BinOp::Gt => bool_f64(a > b),
                    BinOp::Le => bool_f64(a <= b),
                    BinOp::Ge => bool_f64(a >= b),
                    BinOp::Eq => bool_f64(a == b),
                    BinOp::Ne => bool_f64(a != b),
                })
            }
            Expr::Call(name, args) => eval_call_expr(name, args, ctx),
            Expr::Subscript(base, key) => eval_subscript(base, key, ctx),
        }
    }
}