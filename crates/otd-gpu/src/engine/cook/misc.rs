
use otd_core::{CookContext, CookError, CookOutput, EvalContext, Graph, Node, NodeId};

use crate::ops::{self};
use crate::texture::{TOP_FORMAT, TopTexture};

use super::super::dispatch::{Pass, PipelineRef};
use super::super::{
    TopEngine, FALLBACK_SIZE,
};

impl TopEngine {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn cook_ramp_top(
        &mut self,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        node: &Node,
        width: u32,
        height: u32,
        path: &str,
        params: ops::WideParams,
    ) -> Result<(), CookError> {
        let keys = crate::ramp::gather_keyframes(node, eval, path);
        let mode = crate::ramp::interp_from_node(node, eval);
        let tension = crate::ramp::tension_from_node(node, eval);
        let rev = crate::ramp::revision(&keys, mode, tension);
        let lut = crate::ramp::bake_lut(&keys, mode, tension);
        self.upload_pixels(
            id,
            crate::ramp::LUT_WIDTH,
            1,
            wgpu::TextureFormat::Rgba8Unorm,
            &lut,
            rev,
        );
        let lut_view = self.nodes[id].upload.as_ref().unwrap().view.clone();
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let uniforms = self.uniforms(width, height, ctx, params);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        self.run_pass(Pass {
            label: path,
            pipeline: PipelineRef::Builtin("rampTOP"),
            target: &out,
            uniform: &buf,
            in0: &dummy,
            in1: &lut_view,
            nearest: false,
        })
    }
    pub(in crate::engine) fn cook_chop_to_top(
        &mut self,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        node: &otd_core::Node,
        channels: Option<otd_core::ChannelView<'_>>,
        label: &str,
    ) -> Result<(), CookError> {
        let Some((_, samples, _)) = channels else {
            self.ensure_output(id, 1, 1);
            return self.clear_to_black(id, ctx, label);
        };
        let rgba = node
            .param("layout")
            .map(|p| p.eval(eval).as_str() == "rgba")
            .unwrap_or(false);

        let width = samples.iter().map(|s| s.len()).max().unwrap_or(0).max(1) as u32;
        // In RGBA mode four channels share a row, so the row count is a
        // quarter of the channel count, rounded up.
        let per_row = if rgba { 4 } else { 1 };
        let rows = samples.len().div_ceil(per_row).max(1);

        let mut texels = vec![0.0f32; rows * width as usize * 4];
        for (ci, chan) in samples.iter().enumerate() {
            let (row, comp) = if rgba {
                (ci / 4, ci % 4)
            } else {
                (ci, usize::MAX)
            };
            if row >= rows {
                break;
            }
            for x in 0..width as usize {
                // A channel shorter than the widest is held at its last
                // value rather than dropping to zero, so mixing a
                // single-sample control channel with a waveform broadcasts
                // it the way a CHOP does everywhere else.
                let v = chan
                    .get(x)
                    .copied()
                    .or_else(|| chan.last().copied())
                    .unwrap_or(0.0);
                let base = (row * width as usize + x) * 4;
                if comp == usize::MAX {
                    texels[base] = v;
                    texels[base + 1] = v;
                    texels[base + 2] = v;
                    texels[base + 3] = 1.0;
                } else {
                    texels[base + comp] = v;
                }
            }
        }

        let halves: Vec<u16> = texels.iter().copied().map(crate::f32_to_f16).collect();
        let bytes: &[u8] = bytemuck::cast_slice(&halves);
        self.upload_pixels(id, width, rows as u32, TOP_FORMAT, bytes, u64::MAX);
        self.ensure_output(id, width, rows as u32);

        let source_view = self.nodes[id].upload.as_ref().unwrap().view.clone();
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let uniforms = self.uniforms(width, rows as u32, ctx, ops::NO_PARAMS);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        self.run_pass(Pass {
            label,
            pipeline: PipelineRef::Builtin(ops::NULL),
            target: &out,
            uniform: &buf,
            in0: &source_view,
            in1: &dummy,
            // Nearest: each texel is one sample of one channel, and
            // interpolating between two channels is meaningless.
            nearest: true,
        })
    }
    pub(in crate::engine) fn clear_to_black(
        &mut self,
        id: NodeId,
        ctx: &CookContext,
        label: &str,
    ) -> Result<(), CookError> {
        let (w, h) = {
            let out = self.nodes[id].output.as_ref().unwrap();
            (out.key.width, out.key.height)
        };
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let uniforms = self.uniforms(w, h, ctx, ops::NO_PARAMS);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        self.run_pass(Pass {
            label,
            pipeline: PipelineRef::Builtin(ops::NULL),
            target: &out,
            uniform: &buf,
            in0: &dummy,
            in1: &dummy,
            nearest: false,
        })
    }
    pub(in crate::engine) fn cook_analyze(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        node: &otd_core::Node,
        path: &str,
    ) -> Result<CookOutput, CookError> {
        let Some((mut sw, mut sh)) = self.input_size(graph, id, 0) else {
            self.ensure_output(id, 1, 1);
            return self
                .clear_to_black(id, ctx, path)
                .map(|_| CookOutput::Updated);
        };
        sw = sw.max(1);
        sh = sh.max(1);
        self.ensure_output(id, 1, 1);
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let dummy = self.dummy.clone();
        let mut src = self.input_view(graph, id, 0);
        let function = ops::spec("analyzeTOP")
            .map(|spec| spec.pack_params(node, eval)[0][2])
            .unwrap_or(0.0);
        if sw == 1 && sh == 1 {
            let uniforms = self.uniforms(1, 1, ctx, ops::NO_PARAMS);
            let buf = self.write_uniform(id, 0, uniforms);
            self.run_pass(Pass {
                label: path,
                pipeline: PipelineRef::Builtin(ops::NULL),
                target: &out,
                uniform: &buf,
                in0: &src,
                in1: &dummy,
                nearest: true,
            })?;
            return Ok(CookOutput::Updated);
        }
        for (slot, _) in (0..24).enumerate() {
            if sw <= 1 && sh <= 1 {
                break;
            }
            let dw = sw.div_ceil(2);
            let dh = sh.div_ceil(2);
            let last = dw <= 1 && dh <= 1;
            let target = if last {
                out.clone()
            } else {
                let tex = self.pool.acquire(&self.ctx.device, dw.max(1), dh.max(1));
                let view = tex.view.clone();
                self.scratch.push(tex);
                view
            };
            let mut params = ops::NO_PARAMS;
            params[0] = [sw as f32, sh as f32, function, 0.0];
            let uniforms = self.uniforms(dw.max(1), dh.max(1), ctx, params);
            let buf = self.write_uniform(id, slot, uniforms);
            self.run_pass(Pass {
                label: path,
                pipeline: PipelineRef::Builtin("analyzeTOP"),
                target: &target,
                uniform: &buf,
                in0: &src,
                in1: &dummy,
                nearest: true,
            })?;
            if !last {
                src = target;
            }
            sw = dw.max(1);
            sh = dh.max(1);
            if last {
                break;
            }
        }
        Ok(CookOutput::Updated)
    }
    pub(in crate::engine) fn apply_feedback_snapshots(&mut self) {
        let jobs: Vec<(NodeId, TopTexture, wgpu::TextureView)> = self
            .feedback_snapshot
            .iter()
            .filter_map(|(id, snap)| {
                let out = self.nodes.get(id)?.output.as_ref()?;
                if snap.key.width != out.key.width || snap.key.height != out.key.height {
                    return None;
                }
                Some((id, snap.clone(), out.view.clone()))
            })
            .collect();
        for (id, snap, out_view) in jobs {
            let uniforms = self.uniforms(
                snap.key.width,
                snap.key.height,
                &CookContext::default(),
                ops::NO_PARAMS,
            );
            let buf = self.write_uniform(id, 0, uniforms);
            let dummy = self.dummy.clone();
            let _ = self.run_pass(Pass {
                label: "feedback apply",
                pipeline: PipelineRef::Builtin(ops::NULL),
                target: &out_view,
                uniform: &buf,
                in0: &snap.view,
                in1: &dummy,
                nearest: false,
            });
        }
    }
    pub(in crate::engine) fn hold_for_next_frame(&mut self, id: NodeId, target: NodeId) {
        let Some(src) = self
            .nodes
            .get(target)
            .and_then(|n| n.output.as_ref())
            .cloned()
        else {
            return;
        };
        self.ensure_output(id, src.key.width, src.key.height);
        let snap = self
            .feedback_snapshot
            .get(id)
            .filter(|t| t.key.width == src.key.width && t.key.height == src.key.height)
            .cloned()
            .unwrap_or_else(|| {
                self.pool
                    .acquire(&self.ctx.device, src.key.width, src.key.height)
            });
        let uniforms = self.uniforms(
            src.key.width,
            src.key.height,
            &CookContext::default(),
            ops::NO_PARAMS,
        );
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        let _ = self.run_pass(Pass {
            label: "feedback hold",
            pipeline: PipelineRef::Builtin(ops::NULL),
            target: &snap.view,
            uniform: &buf,
            in0: &src.view,
            in1: &dummy,
            nearest: false,
        });
        self.feedback_snapshot.insert(id, snap);
    }
    pub(in crate::engine) fn blit_from(
        &mut self,
        graph: &Graph,
        id: NodeId,
        source: Option<NodeId>,
        ctx: &CookContext,
        label: &str,
    ) -> Result<(), CookError> {
        let src = source
            .filter(|s| *s != id)
            .and_then(|s| self.output(graph, s))
            .cloned();
        let Some(src) = src else {
            // Nothing to read yet: keep a black frame so downstream operators
            // still have something to sample.
            self.ensure_output(id, FALLBACK_SIZE.0, FALLBACK_SIZE.1);
            return Ok(());
        };
        self.ensure_output(id, src.key.width, src.key.height);
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let uniforms = self.uniforms(src.key.width, src.key.height, ctx, ops::NO_PARAMS);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        self.run_pass(Pass {
            label,
            pipeline: PipelineRef::Builtin(ops::NULL),
            target: &out,
            uniform: &buf,
            in0: &src.view,
            in1: &dummy,
            nearest: false,
        })
    }
}
