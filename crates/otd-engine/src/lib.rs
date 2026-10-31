//! `otd-engine` — the cross-family cook.
//!
//! `otd-core` drives scheduling and knows nothing about what an operator
//! produces; `otd-gpu` and `otd-chop` each produce one family and know nothing
//! about each other. This crate is the only place the two meet, and it exists
//! for one reason: a TOP parameter in Export mode has to read a CHOP channel
//! cooked in the same frame (PLAN.md §2.3).
//!
//! The channel store is owned here rather than by the CHOP engine so that,
//! while a node is cooking, the freshly cooked channels can be read
//! immutably by the parameters of whatever cooks next.

pub mod comp;
mod cook_roots;
pub mod demo;
pub mod docs;
pub mod execute;
mod info;
pub mod lint;
mod node_message;
pub mod palette;
pub mod panel;
mod panel_layout;
mod panel_params;
pub mod panel_templates;
pub mod replicator;

pub use cook_roots::{editor_cook_roots, operators_watched_by_info_parameter};
pub use node_message::{NodeMessage, NodeMessageLevel, classify_node_message};
pub use otd_py::{LintSeverity, PythonLint};

use std::cell::RefCell;

use otd_chop::InputState;
use otd_chop::engine::{ChannelStore, ChopEngine, Network};
use otd_chop::ops::{INFO, PARAMETER};
use otd_core::{
    ChannelSource, CookContext, CookEngine, CookError, Cooker, EvalContext, Family, Graph, NodeId,
    OpRegistry,
};
use otd_dat::{DatEngine, DatStore, ScriptHost};
use otd_gpu::{GpuContext, TopEngine};
use otd_py::PyEngine;
use otd_sop::{Geometry, GeometryStore, SopEngine};

pub use otd_chop::{Channel, ChopData};
pub use otd_dat::DatData;
pub use otd_sop::Geometry as SopGeometry;

/// Every operator this build knows about, across all families.
pub fn registry() -> OpRegistry {
    let mut r = otd_gpu::ops::registry();
    for spec in otd_chop::ops::all() {
        r.register(spec.def.clone());
    }
    for spec in otd_dat::ops::all() {
        r.register(spec.def.clone());
    }
    for spec in otd_sop::ops::all() {
        r.register(spec.def.clone());
    }
    // The scene components and materials live with the renderer.
    otd_gpu::scene::register(&mut r);
    comp::register(&mut r);
    panel::register(&mut r);
    r.register(replicator::DEF);
    r
}

/// The network plus the interpreter: everything a parameter can read.
///
/// `otd-chop`'s `Network` answers channel and parameter lookups; this adds
/// the Python fallback for expressions the built-in language cannot parse.
/// The interpreter is behind a `RefCell` because compiling caches, and a
/// parameter evaluation is a `&self` call all the way down.
struct FullNetwork<'a> {
    chops: Network<'a>,
    dats: &'a DatStore,
    python: &'a RefCell<PyEngine>,
}

pub(crate) fn dat_data_at(graph: &Graph, dats: &DatStore, op_path: &str) -> Option<DatData> {
    let id = graph.find(op_path.trim())?;
    if let Some(data) = dats.get(id) {
        return Some(data.clone());
    }
    let node = graph.get(id)?;
    if node.family != Family::Dat {
        return None;
    }
    match node.op_type.as_str() {
        otd_dat::ops::TABLE => {
            let t = node.param("text")?.value.as_str();
            Some(DatData::from_delimited(&t, '\t'))
        }
        otd_dat::ops::TEXT => {
            let t = node.param("text")?.value.as_str();
            Some(DatData::text(t))
        }
        otd_dat::ops::SCRIPT => {
            let t = node.param("source")?.value.as_str();
            Some(DatData::text(t))
        }
        _ => {
            if let Some(p) = node.param("text") {
                let t = p.value.as_str();
                if t.contains('\t') || t.contains('\n') {
                    Some(DatData::from_delimited(&t, '\t'))
                } else {
                    Some(DatData::text(t))
                }
            } else {
                None
            }
        }
    }
}

fn dat_text_at(graph: &Graph, dats: &DatStore, op_path: &str) -> Option<String> {
    dat_data_at(graph, dats, op_path).map(|d| d.as_text())
}

fn dat_content_param_at(graph: &Graph, op_path: &str) -> &'static str {
    graph
        .find(op_path.trim())
        .and_then(|id| graph.get(id))
        .map(|n| {
            if n.op_type.as_str() == otd_dat::ops::SCRIPT {
                "source"
            } else {
                "text"
            }
        })
        .unwrap_or("text")
}

fn dat_rows_at(graph: &Graph, dats: &DatStore, op_path: &str) -> Option<Vec<Vec<String>>> {
    dat_data_at(graph, dats, op_path).map(|d| d.rows)
}

impl otd_core::ChannelSource for FullNetwork<'_> {
    fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
        self.chops.channel(op_path, channel)
    }
    fn channel_names(&self, op_path: &str) -> Vec<String> {
        self.chops.channel_names(op_path)
    }
    fn param_value(&self, op_path: &str, param: &str) -> Option<otd_core::Value> {
        self.chops.param_value(op_path, param)
    }
    fn parent_param(&self, node_path: &str, param: &str) -> Option<otd_core::Value> {
        self.chops.parent_param(node_path, param)
    }
    fn resolve_operator_path(&self, from_operator_path: &str, reference: &str) -> Option<String> {
        self.chops
            .resolve_operator_path(from_operator_path, reference)
    }
    fn eval_python(
        &self,
        source: &str,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Option<Result<otd_core::Value, String>> {
        // A re-entrant evaluation — Python reading a parameter that is itself
        // Python — would deadlock on the RefCell. Refusing is the honest
        // outcome, and the parameter falls back to its constant.
        let mut py = self.python.try_borrow_mut().ok()?;
        Some(py.eval(source, ctx, path))
    }
    fn dat_text(&self, op_path: &str) -> Option<String> {
        dat_text_at(self.chops.graph, self.dats, op_path)
    }
    fn dat_rows(&self, op_path: &str) -> Option<Vec<Vec<String>>> {
        dat_rows_at(self.chops.graph, self.dats, op_path)
    }
    fn dat_content_param(&self, op_path: &str) -> Option<&'static str> {
        self.chops.graph.find(op_path.trim())?;
        Some(dat_content_param_at(self.chops.graph, op_path))
    }

    fn op_type(&self, op_path: &str) -> Option<String> {
        let id = self.chops.graph.find(op_path.trim())?;
        self.chops
            .graph
            .get(id)
            .map(|n| n.op_type.clone())
    }
}

/// Lets a Script DAT run its source without `otd-dat` depending on PyO3.
struct PythonScripts<'a> {
    python: &'a RefCell<PyEngine>,
}

impl ScriptHost for PythonScripts<'_> {
    fn call(
        &self,
        source: &str,
        func: &str,
        args: &[otd_core::Value],
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?
            .call(source, func, args, ctx, path)
    }

    fn run_table(
        &self,
        source: &str,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<Vec<Vec<String>>, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a script cannot run inside another script".to_string())?
            .run_table(source, ctx, path)
    }

    fn webserver_http(
        &self,
        source: &str,
        dat_name: &str,
        job: &otd_dat::webserver::HttpJob,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<otd_dat::webserver::HttpReply, String> {
        let mut py = self
            .python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?;
        let (status, reason, body, content_type) = py.call_webserver_http(
            source,
            dat_name,
            &job.method,
            &job.uri,
            &job.pars,
            &job.client_address,
            &job.server_address,
            &job.data,
            ctx,
            path,
        )?;
        Ok(otd_dat::webserver::HttpReply {
            status,
            reason,
            body,
            content_type,
        })
    }

    fn webserver_event(
        &self,
        source: &str,
        func: &str,
        dat_name: &str,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?
            .call_webserver_event(source, func, dat_name, ctx, path)
    }

    fn webserver_ws(
        &self,
        source: &str,
        func: &str,
        dat_name: &str,
        client: &str,
        uri: Option<&str>,
        text: Option<&str>,
        binary: Option<&[u8]>,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?
            .call_webserver_ws(
                source,
                func,
                dat_name,
                client,
                uri,
                text,
                binary,
                ctx,
                path,
            )
    }
}

/// What the renderer is allowed to ask the rest of the network for.
struct SceneView<'a> {
    graph: &'a Graph,
    geometry: &'a GeometryStore,
    channels: &'a ChannelStore,
}

impl otd_gpu::scene::Scene for SceneView<'_> {
    fn geometry(&self, path: &str) -> Option<&Geometry> {
        // Resolving means a path can name a component and get the geometry
        // its Out operator produces.
        let id = self.graph.resolve_output(self.graph.find(path)?)?;
        self.geometry.get(id)
    }

    fn channels(&self, path: &str) -> Option<Vec<(String, Vec<f32>)>> {
        let id = self.graph.resolve_output(self.graph.find(path)?)?;
        let data = self.channels.get(id)?;
        Some(
            data.channels
                .iter()
                .map(|c| (c.name.clone(), c.samples.clone()))
                .collect(),
        )
    }
}

pub struct Engines {
    pub top: TopEngine,
    pub chop: ChopEngine,
    pub dat: DatEngine,
    pub dats: DatStore,
    pub sop: SopEngine,
    pub geometry: GeometryStore,
    /// Cooked channels, owned here — see the module docs.
    pub channels: ChannelStore,
    /// The embedded interpreter. Started once; a machine where it cannot
    /// start still opens projects, with Python expressions reporting why.
    pub python: RefCell<PyEngine>,
    cook_for_info: Option<*const CookEngine>,
}

impl Engines {
    pub fn new(gpu: GpuContext) -> Self {
        Engines {
            top: TopEngine::new(gpu),
            chop: ChopEngine::new(),
            dat: DatEngine::new(),
            dats: DatStore::new(),
            sop: SopEngine::new(),
            geometry: GeometryStore::new(),
            channels: ChannelStore::new(),
            python: RefCell::new(PyEngine::new()),
            cook_for_info: None,
        }
    }

    pub fn run_cook_frame(
        &mut self,
        cook: &mut CookEngine,
        graph: &Graph,
        roots: &[NodeId],
        ctx: &CookContext,
    ) -> Result<(), CookError> {
        self.cook_for_info = Some(cook as *const CookEngine);
        let mut sync_flush: Option<Box<dyn FnMut(&[String]) -> Result<(), String>>> =
            Some(Box::new({
                let engines_ptr = self as *mut Engines;
                let cook_ptr = cook as *mut CookEngine;
                let graph_ptr = graph as *const Graph;
                let ctx_ptr = ctx as *const CookContext;
                move |paths: &[String]| {
                    // SAFETY: flush runs only during `run_cook_frame`, before
                    // the outer cook returns and while these pointers stay valid.
                    let engines = unsafe { &mut *engines_ptr };
                    let cook = unsafe { &mut *cook_ptr };
                    let graph = unsafe { &*graph_ptr };
                    let ctx = unsafe { &*ctx_ptr };
                    for path in paths {
                        let Some(id) = graph.find(path) else {
                            continue;
                        };
                        cook.cook_frame(graph, &[id], ctx, engines)
                            .map_err(|e| e.to_string())?;
                    }
                    Ok(())
                }
            }));
        otd_core::set_sync_cook_flush(sync_flush.take());
        let result = cook.cook_frame(graph, roots, ctx, self);
        let _ = otd_core::flush_pending_sync_recooks();
        otd_core::set_sync_cook_flush(None);
        self.cook_for_info = None;
        result
    }

    pub fn begin_frame(&mut self) {
        self.top.begin_frame();
    }

    pub fn end_frame(&mut self) {
        self.top.end_frame();
    }

    pub fn shutdown(&mut self) {
        self.top.shutdown();
        self.chop.reset();
        self.dat.reset();
    }

    /// Mouse and keyboard state for the input CHOPs, pushed in by the editor.
    pub fn set_input_state(&mut self, state: InputState) {
        self.chop.io.input = state;
    }

    pub fn chop_data(&self, id: NodeId) -> Option<&ChopData> {
        self.channels.get(id)
    }

    /// The value a parameter would read if it exported from this channel.
    pub fn channel_value(&self, graph: &Graph, path: &str, channel: &str) -> Option<f32> {
        Network {
            graph,
            chops: &self.channels,
        }
        .channel(path, channel)
    }

    /// Evaluate a parameter the way the cook would, for the editor's display.
    pub fn eval_param(
        &self,
        graph: &Graph,
        id: NodeId,
        key: &str,
        ctx: &CookContext,
    ) -> Option<otd_core::Value> {
        let path = graph.path(id);
        let net = FullNetwork {
            chops: Network {
                graph,
                chops: &self.channels,
            },
            dats: &self.dats,
            python: &self.python,
        };
        let eval = ctx.eval_for_node(graph, id, &net, &path);
        Some(graph.get(id)?.param(key)?.eval(&eval))
    }

    /// Like [`eval_param`], but surfaces evaluation errors for the inspector.
    pub fn eval_param_checked(
        &self,
        graph: &Graph,
        id: NodeId,
        key: &str,
        ctx: &CookContext,
    ) -> Result<otd_core::Value, String> {
        let p = graph
            .get(id)
            .and_then(|n| n.param(key))
            .ok_or_else(|| "missing parameter".to_string())?;
        if let Some(err) = p.error() {
            return Err(err.to_string());
        }
        let path = graph.path(id);
        let net = FullNetwork {
            chops: Network {
                graph,
                chops: &self.channels,
            },
            dats: &self.dats,
            python: &self.python,
        };
        let eval = ctx.eval_for_node(graph, id, &net, &path);
        p.eval_checked(&eval)
    }

    /// The parameter changes callbacks asked for this frame.
    ///
    /// Drained by the host and applied between frames — see `otd_core::edit`
    /// for why the write cannot happen where it is written.
    pub fn take_edits(&mut self) -> Vec<otd_core::ParamEdit> {
        otd_core::take_queued_edits()
    }

    pub fn take_recook_requests(&mut self) -> Vec<String> {
        otd_core::take_queued_deferred_recooks()
    }

    pub fn take_udp_send_requests(&mut self) -> Vec<(String, Vec<u8>)> {
        otd_core::take_queued_udp_sends()
    }

    pub fn apply_script_udp_sends(&mut self, graph: &Graph) -> usize {
        let sends = self.take_udp_send_requests();
        self.dat.apply_udp_sends(graph, &sends)
    }

    /// Why Python is unavailable, if it is.
    pub fn python_error(&self) -> Option<String> {
        self.python.borrow().startup_error.clone()
    }

    pub fn take_script_log(&mut self) -> Vec<String> {
        self.python.borrow_mut().take_script_log()
    }

    pub fn lint_python(
        &self,
        graph: &Graph,
        me_path: &str,
        source: &str,
        filename: &str,
    ) -> Vec<otd_py::PythonLint> {
        let net = otd_chop::Network {
            graph,
            chops: &self.channels,
        };
        let mut issues = self.python.borrow().lint_script(source, filename);
        issues.extend(lint::lint_op_paths(graph, me_path, source));
        issues.extend(lint::lint_ch_paths(
            graph,
            me_path,
            source,
            |resolved| net.channel_names(resolved),
        ));
        issues.sort_by(|a, b| (a.line, a.column).cmp(&(b.line, b.column)));
        issues
    }

    pub fn exec_console(
        &mut self,
        graph: &Graph,
        ctx: &CookContext,
        source: &str,
    ) -> Result<String, String> {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }
        let root = graph.root();
        let path = graph.path(root);
        let net = FullNetwork {
            chops: Network {
                graph,
                chops: &self.channels,
            },
            dats: &self.dats,
            python: &self.python,
        };
        let eval = ctx.eval_for_node(graph, root, &net, &path);
        self.python.borrow_mut().exec_console(trimmed, &eval, &path)
    }

    /// The value a parameter would read if it were bound to this one.
    pub fn param_value(&self, graph: &Graph, path: &str, param: &str) -> Option<otd_core::Value> {
        Network {
            graph,
            chops: &self.channels,
        }
        .param_value(path, param)
    }

    /// A device or shader message for a node, for display on its body.
    pub fn node_status(&self, graph: &Graph, id: NodeId) -> Option<String> {
        if let Some(err) = self.top.shader_error(id) {
            return Some(err.to_string());
        }
        if let Some(status) = self.top.status(id) {
            return Some(status.to_string());
        }
        let path = graph.path(id);
        self.chop
            .status(&path)
            .or_else(|| self.dat.status_of(&path))
            .map(|s| s.to_string())
    }

    pub fn dat_data(&self, id: NodeId) -> Option<&DatData> {
        self.dats.get(id)
    }

    pub fn geometry_of(&self, id: NodeId) -> Option<&Geometry> {
        self.geometry.get(id)
    }

    pub fn forget(&mut self, id: NodeId) {
        self.top.forget(id);
        self.chop.forget(id);
        self.channels.remove(id);
        self.dats.remove(id);
        self.geometry.remove(id);
    }

    pub fn reset(&mut self) {
        self.top.reset();
        self.chop.reset();
        self.dat.reset();
        self.dats.clear();
        self.geometry.clear();
        self.channels.clear();
    }
}

/// Collect the inputs of `id` that come from another family.
///
/// This is the whole cross-family mechanism, and it is deliberately here: the
/// module docs say this crate exists because it is the one place the families
/// meet, and a converter operator is the second reason after parameter
/// exports. Everything upstream has already cooked — the scheduler does not
/// care what family a wire carries — so this only has to translate.
///
/// A slot is only translated when the operator *declared* it foreign, so an
/// ordinary CHOP with a CHOP on input 0 costs one integer comparison.
fn crossings(
    graph: &Graph,
    id: NodeId,
    top: &TopEngine,
    channels: &ChannelStore,
    dats: &DatStore,
    geometry: &GeometryStore,
) -> otd_core::Crossings {
    let Some(node) = graph.get(id) else {
        return Vec::new();
    };
    if node.input_families.iter().all(|f| *f == node.family) {
        return Vec::new();
    }
    node.inputs
        .iter()
        .enumerate()
        .map(|(i, _slot)| {
            let wanted = *node.input_families.get(i)?;
            if wanted == node.family {
                return None;
            }
            let src = graph.resolve_input_source(id, i)?;
            match wanted {
                Family::Dat => Some(otd_core::Crossing::Table(dats.get(src)?.rows.clone())),
                Family::Chop => {
                    let data = channels.get(src)?;
                    Some(otd_core::Crossing::Channels {
                        names: data.channels.iter().map(|c| c.name.clone()).collect(),
                        samples: data.channels.iter().map(|c| c.samples.clone()).collect(),
                        sample_rate: data.sample_rate,
                    })
                }
                Family::Sop => {
                    let geo = geometry.get(src)?;
                    // The attribute schema is the vertex layout, spelled out
                    // once here rather than duplicated in every converter.
                    let attrs = vec![
                        ("P".to_string(), 3),
                        ("N".to_string(), 3),
                        ("uv".to_string(), 2),
                        ("Cd".to_string(), 4),
                    ];
                    let mut data = Vec::with_capacity(geo.points.len() * 12);
                    for p in &geo.points {
                        data.extend_from_slice(&p.position);
                        data.extend_from_slice(&p.normal);
                        data.extend_from_slice(&p.uv);
                        data.extend_from_slice(&p.color);
                    }
                    Some(otd_core::Crossing::Points { attrs, data })
                }
                Family::Top => {
                    // Last frame's pixels: this frame's passes are still in an
                    // encoder that has not been submitted. See the TOP to CHOP
                    // operator's docs — TouchDesigner's is one frame behind for
                    // the same reason.
                    let tex = top.output(graph, src)?;
                    let (width, height, rgba) =
                        otd_gpu::read_pixels_rgba_f32(top.context(), tex).ok()?;
                    Some(otd_core::Crossing::Pixels {
                        width,
                        height,
                        rgba,
                    })
                }
                Family::Mat | Family::Comp => None,
            }
        })
        .collect()
}

impl Cooker for Engines {
    fn extra_inputs(&self, graph: &Graph, id: NodeId) -> Vec<NodeId> {
        let mut deps = self.top.extra_inputs(graph, id);
        if graph
            .get(id)
            .is_some_and(|n| n.op_type == INFO || n.op_type == PARAMETER)
        {
            let path = graph
                .node(id)
                .param("op")
                .map(|p| p.eval(&EvalContext::default()).as_str())
                .unwrap_or_default();
            if let Some(watch) = info::resolve_op_from(graph, id, &path) {
                if !deps.contains(&watch) {
                    deps.push(watch);
                }
            }
        }
        deps
    }

    fn cook(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
    ) -> Result<otd_core::CookOutput, CookError> {
        let Some(family) = graph.get(id).map(|n| n.family) else {
            return Err(CookError::NoSuchNode);
        };
        let info_cook = self.cook_for_info.map(|p| unsafe { &*p });
        let Engines {
            top,
            chop,
            dat,
            dats,
            sop,
            geometry,
            channels,
            python,
            ..
        } = self;

        match family {
            Family::Chop => {
                let mut foreign = crossings(graph, id, top, channels, dats, geometry);
                let data = {
                    let net = FullNetwork {
                        chops: Network {
                            graph,
                            chops: channels,
                        },
                        dats,
                        python,
                    };
                    let path = graph.path(id);
                    let eval = ctx.eval_for_node(graph, id, &net, &path);
                    if graph.get(id).is_some_and(|n| n.op_type == INFO) {
                        if let Some(cross) = info::operator_info_crossing(
                            top, info_cook, graph, channels, dats, geometry, id, &eval,
                        ) {
                            foreign.push(Some(cross));
                        }
                    }
                    chop.cook_node(graph, id, ctx, &eval, channels, foreign)?
                };
                channels.insert(id, data);
                Ok(otd_core::CookOutput::Updated)
            }
            Family::Sop => {
                let data = {
                    let net = FullNetwork {
                        chops: Network {
                            graph,
                            chops: channels,
                        },
                        dats,
                        python,
                    };
                    let path = graph.path(id);
                    let eval = ctx.eval_for_node(graph, id, &net, &path);
                    sop.cook_node(graph, id, ctx, &eval, geometry)?
                };
                geometry.insert(id, data);
                Ok(otd_core::CookOutput::Updated)
            }
            Family::Top => {
                let foreign = crossings(graph, id, top, channels, dats, geometry);
                let net = FullNetwork {
                    chops: Network {
                        graph,
                        chops: channels,
                    },
                    dats,
                    python,
                };
                let path = graph.path(id);
                let eval = ctx.eval_for_node(graph, id, &net, &path);
                let view = SceneView {
                    graph,
                    geometry,
                    channels,
                };
                top.cook_top(graph, id, ctx, &eval, Some(&view), &foreign)
            }
            Family::Dat => {
                let foreign = crossings(graph, id, top, channels, dats, geometry);
                let data = {
                    let net = FullNetwork {
                        chops: Network {
                            graph,
                            chops: channels,
                        },
                        dats,
                        python,
                    };
                    let path = graph.path(id);
                    let eval = ctx.eval_for_node(graph, id, &net, &path);
                    let scripts = PythonScripts { python };
                    dat.cook_node(graph, id, ctx, &eval, dats, Some(&scripts), foreign)?
                };
                dats.insert(id, data);
                Ok(otd_core::CookOutput::Updated)
            }
            // COMPs hold sub-networks; the other families are later phases.
            _ => Ok(otd_core::CookOutput::Updated),
        }
    }
}
