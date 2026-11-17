//! Engines aggregate and frame helpers.

use std::cell::RefCell;

use otd_chop::InputState;
use otd_chop::engine::{ChannelStore, ChopEngine, Network};
use otd_core::{
    ChannelSource, CookContext, CookEngine, CookError, Graph, NodeId,
};
use otd_dat::{DatEngine, DatStore};
use otd_gpu::{GpuContext, TopEngine};
use otd_py::PyEngine;
use otd_sop::{Geometry, GeometryStore, SopEngine};

use otd_chop::ChopData;
use otd_dat::DatData;

use crate::audio_partition;
use crate::audio_runtime;

use crate::lint;
use crate::network::FullNetwork;
use crate::project_runtime::{self, FrameTiming};

pub struct Engines {
    pub last_frame_timing: FrameTiming,
    pub audio_partition: audio_partition::AudioPartition,
    pub audio_rt: audio_runtime::AudioRuntime,
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
}

impl Engines {
    pub fn new(gpu: GpuContext) -> Self {
        Engines {
            last_frame_timing: FrameTiming {
                interval_ms: 1000.0 / otd_core::project::DEFAULT_PROJECT_FPS,
                ..Default::default()
            },
            audio_partition: audio_partition::AudioPartition::default(),
            audio_rt: audio_runtime::AudioRuntime::new(),
            top: TopEngine::new(gpu),
            chop: ChopEngine::new(),
            dat: DatEngine::new(),
            dats: DatStore::new(),
            sop: SopEngine::new(),
            geometry: GeometryStore::new(),
            channels: ChannelStore::new(),
            python: RefCell::new(PyEngine::new()),
        }
    }

    pub fn run_cook_frame(
        &mut self,
        cook: &mut CookEngine,
        graph: &Graph,
        roots: &[NodeId],
        ctx: &CookContext,
    ) -> Result<(), CookError> {
        let engines_ptr = self as *mut Engines;
        let cook_ptr = cook as *mut CookEngine;
        let graph_ptr = graph as *const Graph;
        let ctx_ptr = ctx as *const CookContext;
        let session = otd_core::CookSession::new(
            cook,
            Box::new(move |paths: &[String]| {
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
            }),
        );
        otd_core::run_with_cook_session(session, || {
            let result = cook.cook_frame(graph, roots, ctx, self);
            let _ = otd_core::flush_pending_sync_recooks();
            result
        })
    }

    pub fn set_defer_playback_submit(&mut self, defer: bool) {
        self.top.set_defer_submit(defer);
    }

    pub fn flush_deferred_gpu_submit(&mut self) -> f64 {
        self.top.flush_deferred_submit()
    }

    pub fn finish_thumb_blit_frame(&mut self) {
        self.top.end_frame_immediate();
    }

    pub fn begin_frame(&mut self) {
        self.top.poll_chop_pixel_readbacks();
        self.top.begin_frame();
    }

    pub fn run_egui_display_blits(
        &mut self,
        graph: &Graph,
        time: &CookContext,
        blits: &[project_runtime::EguiDisplayBlit],
    ) {
        for blit in blits {
            let Some(src) = self.top.output(graph, blit.top_id).cloned() else {
                continue;
            };
            let _ = self
                .top
                .blit_to_rgba8_unorm(graph, &src, &blit.dst, time);
        }
    }

    pub fn end_frame(&mut self, graph: &Graph) {
        self.chop.end_frame(&self.channels);
        self.top.end_frame();
        self.top.schedule_chop_pixel_readbacks(graph);
    }

    pub fn poll_async_gpu(&mut self) {
        self.top.poll_chop_pixel_readbacks();
    }

    pub fn shutdown(&mut self) {
        self.audio_rt.stop();
        self.top.shutdown();
        self.chop.reset();
        self.dat.reset();
    }

    /// Mouse and keyboard state for the input CHOPs, pushed in by the editor.
    pub fn set_input_state(&mut self, state: InputState) {
        self.chop.io.input = state;
    }

    /// Output-queue depth from the editor settings. Not part of the patch.
    pub fn set_audio_latency_ms(&mut self, ms: u32) {
        self.chop.io.set_audio_latency_ms(ms);
    }

    pub fn set_perform_panel_input(&mut self, state: otd_chop::PerformPanelInput) {
        self.chop.io.perform_panel = state;
    }

    pub fn chop_data(&self, id: NodeId) -> Option<&ChopData> {
        self.channels.get(id)
    }

    fn chop_network<'a>(&'a self, graph: &'a Graph) -> Network<'a> {
        Network::with_panel(graph, &self.channels, &self.chop.io.panel_interaction)
    }

    /// The value a parameter would read if it exported from this channel.
    pub fn channel_value(&self, graph: &Graph, path: &str, channel: &str) -> Option<f32> {
        self.chop_network(graph).channel(path, channel)
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
            chops: self.chop_network(graph),
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
            chops: self.chop_network(graph),
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
        let net = self.chop_network(graph);
        let mut issues = self.python.borrow().lint_script(source, filename);
        issues.extend(lint::lint_op_paths(graph, me_path, source));
        issues.extend(lint::lint_ch_paths(graph, me_path, source, |resolved| {
            net.channel_names(resolved)
        }));
        issues.sort_by_key(|issue| (issue.line, issue.column));
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
            chops: self.chop_network(graph),
            dats: &self.dats,
            python: &self.python,
        };
        let eval = ctx.eval_for_node(graph, root, &net, &path);
        self.python.borrow_mut().exec_console(trimmed, &eval, &path)
    }

    /// The value a parameter would read if it were bound to this one.
    pub fn param_value(&self, graph: &Graph, path: &str, param: &str) -> Option<otd_core::Value> {
        self.chop_network(graph).param_value(path, param)
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

    #[allow(clippy::too_many_arguments)]
    pub fn render_sop_preview(
        &mut self,
        id: NodeId,
        geo: &Geometry,
        orbit: &otd_gpu::OrbitCamera,
        wireframe: bool,
        orthographic: bool,
        show_axes: bool,
        geo_stamp: u64,
    ) -> Result<bool, CookError> {
        self.top.render_sop_preview(
            id,
            geo,
            orbit,
            wireframe,
            orthographic,
            show_axes,
            geo_stamp,
        )
    }

    pub fn forget(&mut self, id: NodeId) {
        self.top.forget(id);
        self.chop.forget(id);
        self.channels.remove(id);
        self.dats.remove(id);
        self.geometry.remove(id);
    }

    /// Drop runtime caches for removed nodes and reopen slots for nodes undo restored.
    pub fn sync_with_graph(&mut self, graph: &Graph) {
        self.top.sync_with_graph(graph);
        self.chop.sync_with_graph(graph);
        self.channels.retain_graph(graph);
        self.dats.retain_graph(graph);
        self.geometry.retain_graph(graph);
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
