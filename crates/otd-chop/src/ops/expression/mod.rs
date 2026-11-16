use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use crate::ops::helpers::script_param;

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "expr" => Param::str("v").with_label("Expression"),
    }
}

/// An arbitrary expression applied to every sample.
///
/// `v` is the sample, `i` its index, `c` the channel index and `n` the number
/// of samples; `time`, `frame` and the rest of the evaluator's vocabulary are
/// in scope too, so `v * sin(time)` is a whole operator.
///
/// The expression is parsed **once per cook**, not once per sample. That is
/// the difference between an operator you can put on an audio-rate channel and
/// one you cannot: parsing 512 times a frame would cost more than the
/// arithmetic by two orders of magnitude.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let source = script_param(c, "expr");
    let Ok(expr) = otd_core::Expr::parse(&source) else {
        // A half-typed expression holds the input rather than emptying the
        // chain — the same rule the GLSL TOP follows for a shader mid-edit.
        return input;
    };

    let n = input.num_samples() as f64;
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let locals = [
                    ("v", *v as f64),
                    ("i", i as f64),
                    ("c", ci as f64),
                    ("n", n),
                ];
                let eval = otd_core::EvalContext {
                    locals: &locals,
                    ..*c.eval
                };
                expr.eval(&eval).map(|r| r as f32).unwrap_or(*v)
            })
            .collect();
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "expressionCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Expression",
                &["in"],
                "Apply an expression to every sample. `v` is the sample.",
                params,
                cook,
            )
}
