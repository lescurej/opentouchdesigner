use std::fmt;

/// Read-only access to the rest of the network, for parameters that are
/// driven by something other than their own value.
///
/// Kept as a trait so `otd-core` still knows nothing about CHOPs: the CHOP
/// engine implements it, and a build with no CHOPs simply passes `None`.
pub trait ChannelSource {
    /// The current value of `channel` on the CHOP at `op_path` (`ch()`).
    fn channel(&self, op_path: &str, channel: &str) -> Option<f32>;

    /// The current value of `param` on the operator at `op_path` (`par()`).
    fn param_value(&self, op_path: &str, param: &str) -> Option<crate::value::Value>;

    /// Every channel name a CHOP is presenting.
    ///
    /// Reading one value needs only a name; *watching* a CHOP needs to know
    /// what the names are, which is what a CHOP Execute DAT does. Defaulted to
    /// empty so a build with no CHOPs is unaffected.
    fn channel_names(&self, _op_path: &str) -> Vec<String> {
        Vec::new()
    }

    /// A parameter on the parent component of `node_path` (`parent.label`, etc.).
    fn parent_param(&self, _node_path: &str, _param: &str) -> Option<crate::value::Value> {
        None
    }

    /// Turn a reference-parameter path into an absolute operator path, from
    /// the operator at `from_operator_path`. Default: only absolute paths.
    fn resolve_operator_path(&self, _from_operator_path: &str, reference: &str) -> Option<String> {
        reference.starts_with('/').then(|| reference.to_string())
    }

    /// Evaluate an expression the built-in language could not parse.
    ///
    /// `None` means there is no Python in this build, in which case the
    /// parameter keeps its constant. Implemented in `otd-engine`, so
    /// `otd-core` stays free of the interpreter.
    fn eval_python(
        &self,
        _source: &str,
        _ctx: &EvalContext,
        _path: &str,
    ) -> Option<Result<crate::value::Value, String>> {
        None
    }

    /// Text contents of a DAT at `op_path`, like TouchDesigner `op('x').text`.
    fn dat_text(&self, _op_path: &str) -> Option<String> {
        None
    }

    /// Table contents as rows of cell strings (empty cells are `""`).
    fn dat_rows(&self, _op_path: &str) -> Option<Vec<Vec<String>>> {
        None
    }

    /// Parameter name holding editable DAT body (`text` or `source` on Script DAT).
    fn dat_content_param(&self, _op_path: &str) -> Option<&'static str> {
        None
    }

    /// Operator type name at `op_path` (e.g. `webserverDAT`).
    fn op_type(&self, _op_path: &str) -> Option<String> {
        None
    }
}

/// Everything a parameter is allowed to see. Deliberately narrow: parameters
/// must not be able to reach into the graph and mutate it mid-cook.
#[derive(Clone, Copy, Default)]
pub struct EvalContext<'a> {
    /// Frame number since the project started.
    pub frame: i64,
    /// Component-local time in seconds (per-component local time, PLAN.md §2.2).
    pub time: f64,
    /// Wall-clock seconds since the project started, independent of local time.
    pub abs_time: f64,
    /// Nominal frames per second of the timeline.
    pub fps: f64,
    /// Present once CHOPs are cooking; `None` in a pure-TOP build or before
    /// the first frame.
    pub channels: Option<&'a dyn ChannelSource>,
    /// The path of the operator being evaluated, so `parent.x` can be
    /// resolved relative to it.
    pub path: Option<&'a str>,
    /// Names the *operator* supplies, looked up before anything else.
    ///
    /// This is how an Expression CHOP gives its expression the sample it is
    /// working on. Deliberately a slice rather than a map: there are one or
    /// two of them, the lookup happens once per sample, and an allocation per
    /// sample at audio rate would be the whole cost of the operator.
    pub locals: &'a [(&'a str, f64)],
}

impl fmt::Debug for EvalContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvalContext")
            .field("frame", &self.frame)
            .field("time", &self.time)
            .field("abs_time", &self.abs_time)
            .field("fps", &self.fps)
            .field("channels", &self.channels.is_some())
            .finish()
    }
}