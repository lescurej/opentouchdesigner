use super::context::EvalContext;
use super::types::ExprError;

pub(crate) fn bool_f64(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

pub(crate) fn eval_var(name: &str, ctx: &EvalContext) -> Result<f64, ExprError> {
    if let Some((_, v)) = ctx.locals.iter().find(|(n, _)| *n == name) {
        return Ok(*v);
    }
    Ok(match name {
        "absTime" | "absTime.seconds" => ctx.abs_time,
        "absTime.frame" => (ctx.abs_time * ctx.fps.max(1.0)).floor(),
        "time" | "me.time" => ctx.time,
        "me.time.rate" => {
            if let (Some(net), Some(path)) = (ctx.channels, ctx.path) {
                net.param_value(path, "rate")
                    .map(|v| v.as_f64())
                    .filter(|r| *r > 0.0)
                    .unwrap_or(ctx.fps)
            } else {
                ctx.fps
            }
        }
        "frame" => ctx.frame as f64,
        "fps" => ctx.fps,
        "pi" => std::f64::consts::PI,
        "tau" => std::f64::consts::TAU,
        "e" => std::f64::consts::E,
        "True" | "true" => 1.0,
        "False" | "false" => 0.0,
        other => return Err(ExprError(format!("unknown name `{other}`"))),
    })
}

pub(crate) fn eval_call(name: &str, a: &[f64]) -> Result<f64, ExprError> {
    fn arity(name: &str, a: &[f64], n: usize) -> Result<(), ExprError> {
        if a.len() == n {
            Ok(())
        } else {
            Err(ExprError(format!(
                "{name}() takes {n} argument(s), got {}",
                a.len()
            )))
        }
    }
    Ok(match name {
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "abs" | "floor" | "ceil" | "round"
        | "sqrt" | "exp" | "log" | "sign" | "fract" | "radians" | "degrees" => {
            arity(name, a, 1)?;
            let x = a[0];
            match name {
                "sin" => x.sin(),
                "cos" => x.cos(),
                "tan" => x.tan(),
                "asin" => x.asin(),
                "acos" => x.acos(),
                "atan" => x.atan(),
                "abs" => x.abs(),
                "floor" => x.floor(),
                "ceil" => x.ceil(),
                "round" => x.round(),
                "sqrt" => x.sqrt(),
                "exp" => x.exp(),
                "log" => x.ln(),
                "sign" => x.signum(),
                "fract" => x - x.floor(),
                "radians" => x.to_radians(),
                _ => x.to_degrees(),
            }
        }
        "pow" | "atan2" | "min" | "max" | "step" | "mod" => {
            arity(name, a, 2)?;
            match name {
                "pow" => a[0].powf(a[1]),
                "atan2" => a[0].atan2(a[1]),
                "min" => a[0].min(a[1]),
                "max" => a[0].max(a[1]),
                "mod" => a[0].rem_euclid(a[1]),
                _ => bool_f64(a[1] >= a[0]),
            }
        }
        "clamp" => {
            arity(name, a, 3)?;
            a[0].clamp(a[1], a[2])
        }
        "lerp" | "mix" => {
            arity(name, a, 3)?;
            a[0] + (a[1] - a[0]) * a[2]
        }
        "smoothstep" => {
            arity(name, a, 3)?;
            let t = ((a[2] - a[0]) / (a[1] - a[0])).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        }
        "fit" => {
            arity(name, a, 5)?;
            let t = (a[0] - a[1]) / (a[2] - a[1]);
            a[3] + t * (a[4] - a[3])
        }
        other => return Err(ExprError(format!("unknown function `{other}`"))),
    })
}