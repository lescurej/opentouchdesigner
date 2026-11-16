use otd_core::{CookContext, CookError, CookOutput, Cooker, Crossings, EvalContext, Graph, Node, NodeId};

use crate::ops::{self, Sizing};
use crate::scene::Scene;

use super::{TopEngine, VideoInKind, FALLBACK_SIZE};

pub(super) fn video_in_kind(op_type: &str) -> Option<VideoInKind> {
    match op_type {
        ops::MOVIE_IN => Some(VideoInKind::File),
        ops::VIDEO_DEVICE_IN => Some(VideoInKind::Camera),
        ops::SCREEN_GRAB => Some(VideoInKind::Screen),
        ops::VIDEO_STREAM_IN => Some(VideoInKind::Stream),
        _ => None,
    }
}

pub(super) fn video_in_idle(kind: VideoInKind, source: &str) -> bool {
    match kind {
        VideoInKind::File => ops::nothing_to_open(false, source),
        VideoInKind::Stream => source.trim().is_empty(),
        VideoInKind::Camera | VideoInKind::Screen => false,
    }
}

/// TouchDesigner order: DAT cell (edited by the field) → Field COMP → Text parameter.
pub(super) fn text_top_body(eval: &EvalContext, node: &Node, top_path: &str) -> String {
    let field_ref = node
        .param("field")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default();
    let dat_ref = node
        .param("dat")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default();
    if let Some(net) = eval.channels {
        if !dat_ref.trim().is_empty() {
            if let Some(path) = net.resolve_operator_path(top_path, dat_ref.trim()) {
                if let Some(t) = net.dat_text(&path) {
                    return t;
                }
            }
        }
        if !field_ref.trim().is_empty() {
            if let Some(path) = net.resolve_operator_path(top_path, field_ref.trim()) {
                if let Some(v) = net.param_value(&path, "text") {
                    return v.as_str();
                }
            }
        }
    }
    node.param("text")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default()
}
pub(super) enum PipelineRef<'a> {
    Builtin(&'a str),
    Key(&'a str),
}

pub(super) struct Pass<'a> {
    pub(super) label: &'a str,
    pub(super) pipeline: PipelineRef<'a>,
    pub(super) target: &'a wgpu::TextureView,
    pub(super) uniform: &'a wgpu::Buffer,
    pub(super) in0: &'a wgpu::TextureView,
    pub(super) in1: &'a wgpu::TextureView,
    pub(super) nearest: bool,
}

/// The TOP a parameter-named operator points at.
fn referenced_target(graph: &Graph, id: NodeId, key: &str) -> Option<NodeId> {
    let node = graph.get(id)?;
    let path = node.param(key)?.eval(&EvalContext::default()).as_str();
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    graph.find_from(id, path).filter(|t| *t != id)
}

impl TopEngine {
    /// Cook one TOP with an explicit evaluation context.
    ///
    /// The context is passed in rather than derived from `ctx` because a
    /// parameter in Export mode has to resolve against the CHOP channels
    /// cooked this frame, and only the cross-family router knows about those.
    pub fn cook_node(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        scene: Option<&dyn Scene>,
        foreign: &Crossings,
    ) -> Result<(), CookError> {
        self.cook_top(graph, id, ctx, eval, scene, foreign)
            .map(|_| ())
    }
}

impl Cooker for TopEngine {
    fn extra_inputs(&self, graph: &Graph, id: NodeId) -> Vec<NodeId> {
        let Some(node) = graph.get(id) else {
            return Vec::new();
        };
        // Select reads the current frame, so its target must cook first.
        // Feedback deliberately does not appear here.
        if node.op_type == ops::SELECT {
            return referenced_target(graph, id, "top").into_iter().collect();
        }
        // A Render TOP discovers the Geometry components it draws rather than
        // being wired to them, so it has to declare them. With no Geometry
        // parameter it draws the whole project, and depends on all of it.
        if node.op_type == ops::RENDER {
            let root = node
                .param("geometry")
                .map(|p| p.eval(&EvalContext::default()).as_str())
                .unwrap_or_default();
            let start = if root.trim().is_empty() {
                Some(graph.root())
            } else {
                graph.find(root.trim())
            };
            let Some(start) = start else {
                return Vec::new();
            };
            let mut found = Vec::new();
            let mut stack = vec![start];
            while let Some(cur) = stack.pop() {
                let Some(n) = graph.get(cur) else { continue };
                if n.op_type == crate::scene::GEOMETRY {
                    found.push(cur);
                    continue;
                }
                stack.extend(n.children.iter().copied());
            }
            return found;
        }
        Vec::new()
    }

    fn cook(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
    ) -> Result<CookOutput, CookError> {
        // No CHOPs in sight: parameters resolve against time alone, and
        // there is no geometry to draw.
        let eval = ctx.eval_ctx();
        self.cook_top(graph, id, ctx, &eval, None, &Crossings::new())
    }
}

impl TopEngine {
    pub fn cook_top(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        scene: Option<&dyn Scene>,
        foreign: &Crossings,
    ) -> Result<CookOutput, CookError> {
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let path = graph.path(id);
        // Parameters are evaluated knowing where they live, so `parent.x`
        // resolves against the right component.
        let eval = &EvalContext {
            path: Some(&path),
            ..*eval
        };

        // COMPs hold sub-networks; they have no texture of their own yet.
        if node.family != otd_core::Family::Top {
            return Ok(CookOutput::Updated);
        }

        let spec = ops::spec(&node.op_type)
            .ok_or_else(|| CookError::op(&path, format!("unknown TOP `{}`", node.op_type)))?;
        // ---- An In operator presents whatever is wired to its component from
        // outside. Nothing else in the engine has to know about components.
        if node.connector == otd_core::Connector::In {
            let source = graph.connector_source(id);
            return self
                .blit_from(graph, id, source, ctx, &path)
                .map(|_| CookOutput::Updated);
        }

        // ---- Feedback holds the target's picture from the END of the last
        // frame, and is refreshed after everything else has cooked.
        //
        // It used to blit the target here, at its own cook, on the reasoning
        // that the target "has not re-cooked yet". That is true only when the
        // target is DOWNSTREAM — the ordinary loop, `fb -> decay -> out` with
        // `target = out`. Point a Feedback TOP at something upstream of it and
        // the target cooks first, the blit copies THIS frame, and the delay
        // silently disappears.
        //
        // Which breaks the most useful camera technique there is: `source`
        // differenced against `feedback(target = source)` is how you get
        // movement, and it came out identically black — a correct-looking
        // patch, eleven operators, no warning, no picture. So the delay is now
        // a property of the operator rather than of the cook order: the copy
        // is deferred to `end_frame`, where every node has already run.
        if node.op_type == ops::FEEDBACK {
            let target = referenced_target(graph, id, "target")
                .and_then(|t| otd_core::cook::resolve_bypass(graph, t));
            let Some(target) = target else {
                // Nothing to read: a black frame, so downstream still samples
                // something rather than failing.
                self.ensure_output(id, FALLBACK_SIZE.0, FALLBACK_SIZE.1);
                return Ok(CookOutput::Updated);
            };
            // Match the target's size now so operators sized from this one are
            // right on the first frame; the content arrives at end_frame.
            let size = self
                .nodes
                .get(target)
                .and_then(|n| n.output.as_ref())
                .map(|t| (t.key.width, t.key.height))
                .unwrap_or(FALLBACK_SIZE);
            self.ensure_output(id, size.0, size.1);
            self.pending_feedback.push((id, target));
            return Ok(CookOutput::Updated);
        }
        if node.op_type == "analyzeTOP" {
            return self.cook_analyze(graph, id, ctx, eval, node, &path);
        }
        // ---- Select reads the same way, but its target has already cooked.
        if node.op_type == ops::SELECT {
            let target = referenced_target(graph, id, "top");
            return self
                .blit_from(graph, id, target, ctx, &path)
                .map(|_| CookOutput::Updated);
        }
        // ---- CHOP to TOP: the pixels come from a wire of another family.
        if node.op_type == ops::CHOP_TO_TOP {
            let channels = foreign
                .first()
                .and_then(|c| c.as_ref())
                .and_then(|c| c.as_channels());
            return self
                .cook_chop_to_top(id, ctx, eval, node, channels, &path)
                .map(|_| CookOutput::Updated);
        }

        if matches!(node.op_type.as_str(), ops::SYPHON_IN | ops::SPOUT_IN) {
            let fallback = ops::generator_size(node, eval);
            self.release_video(id);
            self.set_node_status(id, ops::io_stub_status(&node.op_type).map(str::to_string));
            self.ensure_output(id, fallback.0, fallback.1);
            return self
                .clear_to_black(id, ctx, &path)
                .map(|_| CookOutput::Updated);
        }

        // ---- Video: the pixels come from a decoder thread, not a shader.
        if video_in_kind(&node.op_type).is_some() {
            return self.cook_video(graph, id, ctx, eval, scene, &path);
        }

        if node.op_type == ops::MOVIE_OUT {
            return self
                .cook_movie_out_top(graph, id, ctx, eval, node, &path)
                .map(|_| CookOutput::Updated);
        }

        // ---- Cache holds whatever it last saw while Active is off.
        if node.op_type == ops::CACHE {
            let active = node
                .param("active")
                .map(|p| p.eval(eval).as_bool())
                .unwrap_or(true);
            if !active && self.nodes.get(id).is_some_and(|n| n.output.is_some()) {
                return Ok(CookOutput::Unchanged);
            }
        }

        // ---- Resolution
        let (width, height) = match spec.sizing {
            Sizing::Params => ops::generator_size(node, eval),
            Sizing::Input0 if node.op_type == "switchTOP" => {
                let mode = node
                    .param("outputresolution")
                    .map(|p| p.eval(eval).as_str())
                    .unwrap_or_else(|| "useinput".into());
                if matches!(mode.as_str(), "custom" | "fit" | "limit") {
                    ops::filter_output_dimensions(node, eval, |idx| self.input_size(graph, id, idx))
                        .unwrap_or(FALLBACK_SIZE)
                } else {
                    let index = node
                        .param("index")
                        .map(|p| p.eval(eval).as_f64().floor().max(0.0) as usize)
                        .unwrap_or(0);
                    self.input_size(graph, id, index)
                        .or_else(|| self.input_size(graph, id, 0))
                        .unwrap_or(FALLBACK_SIZE)
                }
            }
            Sizing::Input0 => {
                ops::filter_output_dimensions(node, eval, |idx| self.input_size(graph, id, idx))
                    .unwrap_or(FALLBACK_SIZE)
            }
            Sizing::Input0OrParams => self
                .input_size(graph, id, 0)
                .unwrap_or_else(|| ops::generator_size(node, eval)),
            // Handled above; a Referenced operator never reaches here.
            Sizing::Referenced => FALLBACK_SIZE,
        };
        self.ensure_output(id, width, height);

        if node.op_type == "rampTOP" {
            let params = spec.pack_params(node, eval);
            return self
                .cook_ramp_top(id, ctx, eval, node, width, height, &path, params)
                .map(|_| CookOutput::Updated);
        }

        if let Some(msg) = ops::io_stub_status(&node.op_type) {
            self.set_node_status(id, Some(msg.to_string()));
        }

        // ---- Text is rasterised on the CPU and uploaded, so the shader only
        // has to tint the coverage. That split is what makes Colour a live
        // parameter: changing it is a uniform write, not a re-lay-out.
        if node.op_type == ops::TEXT {
            self.cook_text(id, ctx, eval, node, width, height, &path)?;
            return Ok(CookOutput::Updated);
        }

        // ---- The 3D pipeline is a different beast: vertex buffers, a depth
        // attachment, a cull mode. It gets its own module.
        if node.op_type == ops::RENDER {
            return self
                .cook_render(graph, id, ctx, eval, scene, width, height)
                .map(|_| CookOutput::Updated);
        }

        // ---- Shader: built in, or compiled from a parameter.
        let mut pipeline_key: Option<String> = None;
        if spec.dynamic_shader {
            let (source, is_glsl) = ops::shader_source(node, eval);
            match self.user_pipeline(&source, is_glsl) {
                Ok(key) => {
                    let entry = self.ensure_node_mut(id)?;
                    entry.shader_error = None;
                    entry.shader_key = Some(key.clone());
                    pipeline_key = Some(key);
                }
                Err(e) => {
                    let entry = self.ensure_node_mut(id)?;
                    entry.shader_error = Some(e);
                    // Hold the last shader that worked, so an edit in progress
                    // does not black out a running patch.
                    pipeline_key = entry.shader_key.clone();
                    if pipeline_key.is_none() {
                        return Ok(CookOutput::Updated);
                    }
                }
            }
        }

        let params = spec.pack_params(node, eval);
        let base = self.uniforms(width, height, ctx, params);
        let nearest = node
            .param("filter")
            .map(|p| p.eval(eval).as_str() == "nearest")
            .unwrap_or(false)
            || ops::movie_sampler_nearest(node, eval);

        let in0 = self.input_view(graph, id, 0);
        let in1 = self.input_view(graph, id, 1);
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let type_name = spec.def.type_name;

        if spec.two_pass {
            // Horizontal into scratch, vertical into the output.
            let scratch = self.pool.acquire(&self.ctx.device, width, height);
            let scratch_view = scratch.view.clone();
            self.scratch.push(scratch);
            let dummy = self.dummy.clone();

            let mut first = base;
            first.params[0][1] = 0.0;
            let buf0 = self.write_uniform(id, 0, first);
            self.run_pass(Pass {
                label: &path,
                pipeline: PipelineRef::Builtin(type_name),
                target: &scratch_view,
                uniform: &buf0,
                in0: &in0,
                in1: &dummy,
                nearest,
            })?;

            let mut second = base;
            second.params[0][1] = 1.0;
            let buf1 = self.write_uniform(id, 1, second);
            self.run_pass(Pass {
                label: &path,
                pipeline: PipelineRef::Builtin(type_name),
                target: &out,
                uniform: &buf1,
                in0: &scratch_view,
                in1: &dummy,
                nearest,
            })
        } else {
            let buf = self.write_uniform(id, 0, base);
            let pipeline = match &pipeline_key {
                Some(key) => PipelineRef::Key(key.as_str()),
                None => PipelineRef::Builtin(type_name),
            };
            self.table_override = pipeline_key
                .as_ref()
                .and_then(|key| self.const_tables.get(key).cloned());
            let ran = self.run_pass(Pass {
                label: &path,
                pipeline,
                target: &out,
                uniform: &buf,
                in0: &in0,
                in1: &in1,
                nearest,
            });
            self.table_override = None;
            ran
        }
        .map(|_| CookOutput::Updated)
    }
}

