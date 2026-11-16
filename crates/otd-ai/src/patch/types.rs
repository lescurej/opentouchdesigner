//! Turning a sentence into operators.
//!
//! Two decisions carry this module.
//!
//! **The model's knowledge of the operators comes from the registry**, not
//! from a hand-written list and not from whatever it remembers about
//! TouchDesigner. [`catalogue`] is generated from the same `OpRegistry` the
//! editor builds its menus from, so an operator added tomorrow is one the
//! assistant can use tomorrow, and one that was renamed cannot be suggested
//! under its old name. The docs generator already proved the pattern; this is
//! the same trick pointed at a different reader.
//!
//! **Nothing a model says touches the graph until it has been checked.** The
//! reply is JSON, it is validated against the real registry — every operator
//! type, every parameter name, every connection endpoint — and a plan with an
//! operator that does not exist is refused whole rather than applied in part.
//! A patch that is half-built is worse than one that was not built, because
//! the user now has to work out which half.

use std::collections::BTreeMap;

use otd_core::Value;

/// What the assistant proposes: nodes to create, wires to run between them,
/// and what to look at afterwards.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub notes: String,
    pub nodes: Vec<PlannedNode>,
    pub connections: Vec<PlannedWire>,
    /// Operators already in the network to retune. Separate from `nodes`
    /// because they are not being created: naming an existing operator under
    /// `nodes` used to get you a renamed duplicate beside the original and the
    /// original left exactly as it was, which is the opposite of what "make
    /// this slower" means.
    pub sets: Vec<PlannedSet>,
    /// Operators already in the network to remove, by name. Only direct
    /// children of the network being edited — see [`apply`].
    pub deletes: Vec<String>,
    /// Name of the node to set as the viewer, if any.
    pub viewer: Option<String>,
    /// Trouble noticed while reading the reply, to be reported alongside
    /// whatever `apply` finds. Parse-time problems used to have nowhere to go
    /// but an error, so anything not worth refusing the whole plan for was
    /// simply dropped.
    pub warnings: Vec<String>,
}

impl Plan {
    /// Move every node the plan creates, so a recipe authored around the
    /// origin can land in clear canvas.
    pub fn offset(&mut self, dx: f32, dy: f32) {
        for node in &mut self.nodes {
            node.pos[0] += dx;
            node.pos[1] += dy;
        }
    }

    /// The box the plan's nodes occupy: `(min, max)`, or nothing if it
    /// creates none.
    pub fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
        let first = self.nodes.first()?.pos;
        Some(self.nodes.iter().fold((first, first), |(lo, hi), n| {
            (
                [lo[0].min(n.pos[0]), lo[1].min(n.pos[1])],
                [hi[0].max(n.pos[0]), hi[1].max(n.pos[1])],
            )
        }))
    }
}

/// A change to an operator that already exists.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedSet {
    pub node: String,
    /// Left as raw JSON: coercing a value needs the parameter's declared type,
    /// and that comes off the node in the graph, which the parser cannot see.
    /// [`apply`] does it once it has both.
    pub params: BTreeMap<String, serde_json::Value>,
    pub expressions: BTreeMap<String, String>,
    pub exports: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedNode {
    pub name: String,
    pub op: String,
    pub pos: [f32; 2],
    /// Constant parameter values, by key.
    pub params: BTreeMap<String, Value>,
    /// Parameters to put into Expression mode, by key.
    pub expressions: BTreeMap<String, String>,
    /// Parameters driven by CHOP channels, by key: `node:channel` → `ch()` expression.
    pub exports: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedWire {
    pub from: String,
    pub to: String,
    pub input: usize,
}

/// What a plan did, or would do.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Applied {
    pub created: Vec<String>,
    /// Parameters retuned on operators that already existed, as `node.key`.
    pub changed: Vec<String>,
    /// Operators removed, by the name they had.
    pub removed: Vec<String>,
    pub wired: usize,
    /// Things that were skipped, with the reason. Reported rather than
    /// swallowed: a parameter the model invented is worth knowing about.
    pub warnings: Vec<String>,
}
