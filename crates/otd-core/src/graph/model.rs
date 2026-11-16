//! The graph model: an arena of nodes, typed by family, arranged in a
//! component hierarchy. No GPU, no UI, no Python — everything in here is
//! plain data so the cook engine can be unit tested (PLAN.md §3, "a clean
//! core").

use indexmap::IndexMap;
use slotmap::SlotMap;
use std::collections::HashSet;

use crate::param::Param;

slotmap::new_key_type! {
    /// Stable handle to a node. Generational, so a stale id from a deleted
    /// node resolves to `None` rather than to whatever took its slot.
    pub struct NodeId;
}

/// The typed wire families. Same-family wiring only; crossing families is an
/// explicit converter operator or a parameter reference (PLAN.md §2.1).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Family {
    /// GPU textures.
    Top,
    /// Channels of samples.
    Chop,
    /// Geometry.
    Sop,
    /// Text and tables.
    Dat,
    /// Materials.
    Mat,
    /// Containers.
    Comp,
}

impl Family {
    pub fn suffix(self) -> &'static str {
        match self {
            Family::Top => "TOP",
            Family::Chop => "CHOP",
            Family::Sop => "SOP",
            Family::Dat => "DAT",
            Family::Mat => "MAT",
            Family::Comp => "COMP",
        }
    }

    /// Wire tint, matching the family colours artists already have muscle
    /// memory for. sRGB, 0-255.
    pub fn color(self) -> [u8; 3] {
        match self {
            Family::Top => [92, 68, 128],
            Family::Chop => [58, 128, 78],
            Family::Sop => [52, 108, 118],
            Family::Dat => [128, 58, 98],
            Family::Mat => [128, 102, 52],
            Family::Comp => [78, 82, 92],
        }
    }

    pub fn all() -> &'static [Family] {
        &[
            Family::Top,
            Family::Chop,
            Family::Sop,
            Family::Dat,
            Family::Mat,
            Family::Comp,
        ]
    }
}

/// Whether an operator surfaces as a connector on its parent component.
///
/// PLAN.md §2.4: "In/Out ops inside a component surface as typed connectors
/// on the node." That is the whole encapsulation mechanism — a component's
/// shape is defined by what is inside it, not declared separately.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Connector {
    #[default]
    None,
    In,
    Out,
}

/// Per-node toggles that live on the node body in the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeFlags {
    /// Viewer flag: show the operator viewer on the node body (vs the family icon).
    /// Serialized as `display` in `.otd` for older projects.
    #[serde(rename = "display")]
    pub viewer: bool,
    /// Show this operator on the network background (auto-arranged viewer grid).
    #[serde(default)]
    pub bg_display: bool,
    /// Viewer Active: the node body is for inspecting/interacting, not moving.
    #[serde(default)]
    pub viewer_active: bool,
    /// Pass input 0 straight through without cooking this operator.
    pub bypass: bool,
    /// This node is a cook root even if nothing downstream wants it.
    pub render: bool,
    /// Always cook (TouchDesigner cook flag), even off-screen.
    pub cook: bool,
    /// Edits, wiring and moves are blocked (TouchDesigner lock).
    pub lock: bool,
    /// Do not overwrite this operator when its clone master re-syncs.
    #[serde(default)]
    pub clone_immune: bool,
}

impl Default for NodeFlags {
    fn default() -> Self {
        NodeFlags {
            viewer: true,
            bg_display: false,
            viewer_active: false,
            bypass: false,
            render: false,
            cook: false,
            lock: false,
            clone_immune: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub op_type: String,
    pub family: Family,
    pub params: IndexMap<String, Param>,
    /// One slot per declared input; `None` means unconnected.
    pub inputs: Vec<Option<NodeId>>,
    /// Which output connector on the source operator feeds each input.
    pub input_source_out: Vec<usize>,
    pub input_labels: Vec<String>,
    /// The family each input accepts. Usually the node's own family, but a
    /// component's connectors take theirs from its In operators, which is how
    /// one component can accept a TOP and a CHOP.
    pub input_families: Vec<Family>,
    pub connector: Connector,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    /// Editor position. Part of the project file — node layout is authored
    /// information, not incidental UI state.
    pub pos: [f32; 2],
    pub size: Option<[f32; 2]>,
    pub flags: NodeFlags,
    /// Operators that must re-cook every frame regardless of parameter
    /// changes (a movie player, a device input).
    pub intrinsically_time_dependent: bool,
    /// This component's contents come from a `.otdc` file rather than from
    /// the project. The project stores the reference; the file stores the
    /// network (PLAN.md §5, Phase 3).
    pub external: Option<String>,
    /// This component tracks another one in the same project: its contents
    /// are replaced by copies of the master's whenever the master changes.
    pub clone_of: Option<String>,
    /// The master's subtree revision at the last sync, so an unchanged
    /// master costs nothing.
    pub clone_synced_at: u64,
    /// Bumped on every mutation. The cook engine compares against the value
    /// it last cooked at — a revision counter rather than a dirty bit, so a
    /// node edited twice between cooks still only cooks once (PLAN.md §3).
    pub(crate) revision: u64,
    /// When this node is a Callbacks DAT text node, the operator that owns it.
    pub callback_owner: Option<NodeId>,
    pub docked_to: Option<NodeId>,
    pub dock_shown: bool,
}

impl Node {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn param(&self, key: &str) -> Option<&Param> {
        self.params.get(key)
    }

    pub fn param_flexible(&self, key: &str) -> Option<&Param> {
        if let Some(p) = self.params.get(key) {
            return Some(p);
        }
        let key_lc = key.to_ascii_lowercase();
        self.params
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key) || k.to_ascii_lowercase() == key_lc)
            .map(|(_, p)| p)
    }

    /// True when any parameter is animated by an expression.
    pub fn has_time_dependent_param(&self) -> bool {
        self.params.values().any(|p| p.is_time_dependent())
    }

    /// Operator paths this node's parameters read from — Export and Bind
    /// sources. The cook engine turns these into dependencies so the source
    /// cooks first and its animation propagates here.
    pub fn param_sources(&self) -> impl Iterator<Item = &str> {
        self.params.values().flat_map(|p| p.referenced_ops())
    }

    /// True when some parameter reads a custom parameter on the enclosing
    /// component, which makes this node depend on that component's settings.
    pub fn references_parent(&self) -> bool {
        self.params.values().any(|p| {
            p.mode == crate::param::ParamMode::Expression && p.expression.contains("parent.")
        })
    }

    /// The author-defined parameters on this node, in declaration order.
    pub fn custom_params(&self) -> impl Iterator<Item = (&String, &Param)> {
        self.params.iter().filter(|(_, p)| p.custom)
    }
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub(crate) nodes: SlotMap<NodeId, Node>,
    pub(crate) root: NodeId,
    /// The directory the project file lives in, when it came from one.
    ///
    /// External component references are stored exactly as authored, which
    /// means a bundle can hold `components/meter.otdc` and stay portable. That
    /// only works if something remembers what the path is relative *to* — the
    /// process's working directory is the wrong answer, and is what makes a
    /// project open by hand and fail from a service.
    pub(crate) base_dir: Option<std::path::PathBuf>,
    pub(crate) audio_topology_gen: u64,
    pub load_warnings: Vec<String>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(Node {
            name: "root".into(),
            op_type: "baseCOMP".into(),
            family: Family::Comp,
            params: IndexMap::new(),
            inputs: Vec::new(),
            input_source_out: Vec::new(),
            input_labels: Vec::new(),
            input_families: Vec::new(),
            connector: Connector::None,
            parent: None,
            children: Vec::new(),
            pos: [0.0, 0.0],
            size: None,
            flags: NodeFlags::default(),
            intrinsically_time_dependent: false,
            external: None,
            clone_of: None,
            clone_synced_at: 0,
            revision: 0,
            callback_owner: None,
            docked_to: None,
            dock_shown: true,
        });
        Graph {
            nodes,
            root,
            base_dir: None,
            audio_topology_gen: 0,
            load_warnings: Vec::new(),
        }
    }

    pub fn audio_topology_generation(&self) -> u64 {
        self.audio_topology_gen
    }

    /// The directory external component references resolve against.
    pub fn base_dir(&self) -> Option<&std::path::Path> {
        self.base_dir.as_deref()
    }

    pub fn set_base_dir(&mut self, dir: Option<std::path::PathBuf>) {
        self.base_dir = dir;
    }

    pub fn refresh_clone_paths_after_rename(&mut self, old_path: &str, new_path: &str) {
        for id in self.walk() {
            let node = &mut self.nodes[id];
            let Some(clone) = node.clone_of.as_deref() else {
                continue;
            };
            if clone == old_path {
                node.clone_of = Some(new_path.to_string());
            } else if let Some(suffix) = clone.strip_prefix(&format!("{old_path}/")) {
                node.clone_of = Some(format!("{new_path}/{suffix}"));
            }
        }
    }

    /// An external reference as an openable path.
    pub fn resolve_external(&self, file: &str) -> std::path::PathBuf {
        match &self.base_dir {
            Some(dir) if std::path::Path::new(file).is_relative() => dir.join(file),
            _ => std::path::PathBuf::from(file),
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    /// Mutable access. Bumps the node's revision, so the next cook sees it as
    /// dirty — conservative by design: an extra cook is cheap, a missed one
    /// shows up as a frozen viewer.
    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        let n = &mut self.nodes[id];
        n.revision += 1;
        n
    }

    pub(crate) fn get_mut_raw(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Read-write access that does *not* dirty the node. For things the cook
    /// does not depend on, like dragging a node around the canvas.
    pub fn node_mut_quiet(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id]
    }

    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes.iter()
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id].children
    }

    pub fn owned_callback_dat(&self, owner: NodeId) -> Option<NodeId> {
        self.nodes.iter().find_map(|(id, n)| {
            (n.callback_owner == Some(owner)).then_some(id)
        })
    }

    pub fn owned_callback_dats(&self, owners: &[NodeId]) -> Vec<NodeId> {
        let owners: HashSet<NodeId> = owners.iter().copied().collect();
        self.nodes
            .iter()
            .filter_map(|(id, n)| {
                n.callback_owner
                    .filter(|owner| owners.contains(owner))
                    .map(|_| id)
            })
            .collect()
    }

    pub fn callback_dat_links(&self) -> impl Iterator<Item = (NodeId, NodeId)> + '_ {
        self.nodes.iter().filter_map(|(id, n)| {
            n.callback_owner.map(|owner| (owner, id))
        })
    }
}
