use indexmap::IndexMap;

use crate::param::Param;

use super::{Connector, Family};

/// A static description of an operator type. Registered once at startup;
/// `Graph::create` stamps out nodes from it.
#[derive(Clone)]
pub struct OpDef {
    pub type_name: &'static str,
    pub label: &'static str,
    pub family: Family,
    pub inputs: &'static [&'static str],
    /// The family each input accepts, when it is not this operator's own.
    ///
    /// Almost every operator wires within one family, which is the rule that
    /// keeps the graph legible (PLAN.md §2.1) — so the common case is `&[]`,
    /// meaning "all mine". The exceptions are the **converter operators**,
    /// which are the sanctioned way to cross a family boundary: a CHOP to TOP
    /// takes a CHOP input and produces a texture. Declaring the exception per
    /// input rather than per operator lets a converter also take inputs of its
    /// own family.
    pub input_families: &'static [Family],
    pub summary: &'static str,
    pub time_dependent: bool,
    pub params: fn() -> IndexMap<String, Param>,
    /// Set for the In and Out operators that give a component its shape.
    pub connector: Connector,
}

impl OpDef {
    /// What each input accepts, one entry per declared input. Falls back to
    /// this operator's own family for any input the definition did not
    /// override, so a converter only has to name the inputs that cross.
    pub fn accepted_families(&self) -> Vec<Family> {
        (0..self.inputs.len())
            .map(|i| self.input_families.get(i).copied().unwrap_or(self.family))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct OpRegistry {
    defs: IndexMap<String, OpDef>,
}

impl OpRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: OpDef) {
        self.defs.insert(def.type_name.to_string(), def);
    }

    pub fn get(&self, type_name: &str) -> Option<&OpDef> {
        self.defs.get(type_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &OpDef> {
        self.defs.values()
    }

    /// Operators of one family, for the Tab create-dialog.
    pub fn of_family(&self, family: Family) -> impl Iterator<Item = &OpDef> {
        self.defs.values().filter(move |d| d.family == family)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("no such node")]
    NoSuchNode,
    #[error("input index {0} out of range")]
    BadInputIndex(usize),
    #[error("cannot wire a {0} output into a {1} input")]
    FamilyMismatch(&'static str, &'static str),
    #[error("that connection would create a cycle; use a Feedback operator instead")]
    WouldCycle,
    #[error("unknown operator type `{0}`")]
    UnknownOpType(String),
    #[error("no node at path `{0}`")]
    BadPath(String),
    #[error("no parameter `{0}` on that operator")]
    NoSuchParam(String),
}
