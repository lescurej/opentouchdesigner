
use otd_core::{CookContext, CookError, EvalContext, Graph, NodeId};

use crate::ops::{self};
use crate::record::{self, Recorder};

use super::super::dispatch::{Pass, PipelineRef};
use super::super::dispatch::text_top_body;
use super::super::{
    PendingRecord, TopEngine, FALLBACK_SIZE,
};

impl TopEngine {
    pub(in crate::engine) fn cook_movie_out_top(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        node: &otd_core::Node,
        label: &str,
    ) -> Result<(), CookError> {
        let input_size = self.input_size(graph, id, 0).unwrap_or(FALLBACK_SIZE);
        let (width, height) = ops::movie_output_dimensions(node, eval, input_size);
        self.ensure_output(id, width, height);
        let in0 = self.input_view(graph, id, 0);
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let nearest = ops::movie_sampler_nearest(node, eval);
        let use_fit = (width, height) != input_size;
        let params = if use_fit {
            let mut p = ops::NO_PARAMS;
            p[0][0] = ops::movie_fit_mode(node, eval);
            p
        } else {
            ops::NO_PARAMS
        };
        let uniforms = self.uniforms(width, height, ctx, params);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        let pipeline = if use_fit {
            PipelineRef::Builtin("fitTOP")
        } else {
            PipelineRef::Builtin(ops::NULL)
        };
        self.run_pass(Pass {
            label,
            pipeline,
            target: &out,
            uniform: &buf,
            in0: &in0,
            in1: &dummy,
            nearest,
        })?;
        self.cook_movie_out(id, node, eval);
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn cook_text(
        &mut self,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        node: &otd_core::Node,
        width: u32,
        height: u32,
        label: &str,
    ) -> Result<(), CookError> {
        let text = |key: &str| {
            node.param(key)
                .map(|p| p.eval(eval).as_str())
                .unwrap_or_default()
        };
        let number = |key: &str, fallback: f32| {
            node.param(key)
                .map(|p| p.eval(eval).as_f32())
                .unwrap_or(fallback)
        };
        let index = |key: &str| {
            node.param(key)
                .map(|p| p.menu_index(&p.eval(eval).as_str()))
                .unwrap_or(0)
        };

        let font_path = text("font");
        let body = text_top_body(eval, node, label);
        let layout = crate::text::Layout {
            text: body,
            size: number("size", 48.0).max(1.0),
            line_spacing: number("linespacing", 1.2),
            horizontal: crate::text::Align::from_index(index("halign")),
            vertical: crate::text::Align::from_index(index("valign")),
            width,
            height,
            wrap: node
                .param("wrap")
                .map(|p| p.eval(eval).as_bool())
                .unwrap_or(true),
        };

        // Everything the raster depends on. The colour is deliberately absent:
        // it is applied in the shader, so changing it must not re-rasterise.
        let key = format!(
            "{font_path}|{}|{}|{}|{}|{}|{width}x{height}|{}",
            layout.text,
            layout.size,
            layout.line_spacing,
            index("halign"),
            index("valign"),
            layout.wrap
        );

        if self.text_keys.get(id).map(|k| k.as_str()) != Some(key.as_str()) {
            match self.fonts.get(&font_path) {
                Ok(font) => {
                    let raster = crate::text::rasterise(font, &layout);
                    self.upload_pixels(
                        id,
                        raster.width,
                        raster.height,
                        wgpu::TextureFormat::Rgba8Unorm,
                        &raster.pixels,
                        u64::MAX,
                    );
                    self.text_keys.insert(id, key);
                    self.set_node_status(id, None);
                }
                Err(e) => {
                    // No font is a message on the node and a blank picture, not
                    // a failed cook: losing the whole render because a caption
                    // cannot find a face is the wrong trade.
                    self.set_node_status(id, Some(e));
                    self.text_keys.remove(id);
                    return self.clear_to_black(id, ctx, label);
                }
            }
        }

        let Some(source_view) = self.nodes[id].upload.as_ref().map(|u| u.view.clone()) else {
            return self.clear_to_black(id, ctx, label);
        };
        let params = ops::spec(&node.op_type).unwrap().pack_params(node, eval);
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let uniforms = self.uniforms(width, height, ctx, params);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        self.run_pass(Pass {
            label,
            pipeline: PipelineRef::Builtin(ops::TEXT),
            target: &out,
            uniform: &buf,
            in0: &source_view,
            in1: &dummy,
            nearest: false,
        })
    }
    pub(in crate::engine) fn cook_movie_out(&mut self, id: NodeId, node: &otd_core::Node, eval: &EvalContext) {
        let text = |key: &str| {
            node.param(key)
                .map(|p| p.eval(eval).as_str())
                .unwrap_or_default()
        };
        let record = node
            .param("record")
            .map(|p| p.eval(eval).as_bool())
            .unwrap_or(false);
        let pause = node
            .param("pause")
            .map(|p| p.eval(eval).as_bool())
            .unwrap_or(false);
        let addframe = node
            .param("addframe")
            .map(|p| p.eval(eval).as_bool())
            .unwrap_or(false);
        let edge = self.movie_out_edge(id);
        let record_pulse = record && !edge.record;
        let add_pulse = addframe && !edge.addframe;
        edge.record = record;
        edge.addframe = addframe;

        if !record {
            if self.recorders.remove(id).is_some() {
                self.set_node_status(id, None);
            }
            return;
        }

        let Some((width, height)) = self
            .nodes
            .get(id)
            .and_then(|n| n.output.as_ref().map(|t| (t.key.width, t.key.height)))
        else {
            return;
        };

        let mut out_type = text("type");
        if out_type.is_empty() {
            out_type = "movie".into();
        }
        let fps = node
            .param("fps")
            .map(|p| p.eval(eval).as_f32() as f64)
            .unwrap_or(60.0);
        if let Some(limit) = ops::movie_out_length_limit_frames(node, eval, fps) {
            let written = self.movie_out_frames_written(id);
            if written >= limit {
                self.recorders.remove(id);
                self.set_node_status(id, Some(format!("limit reached ({written} frames)")));
                return;
            }
        }

        let capture = match out_type.as_str() {
            "image" => record_pulse || add_pulse,
            "imagesequence" => {
                if pause {
                    add_pulse
                } else {
                    true
                }
            }
            "stopframemovie" => add_pulse,
            _ => {
                if pause {
                    add_pulse
                } else {
                    true
                }
            }
        };
        if !capture {
            if self.recorders.contains_key(id) {
                let written = self.movie_out_frames_written(id);
                self.set_node_status(id, Some(format!("recording (paused) — {written} frames")));
            }
            return;
        }

        let image_type = text("imagefiletype");
        let image_type = if image_type.is_empty() {
            "png".to_string()
        } else {
            image_type
        };
        let stream = matches!(out_type.as_str(), "movie" | "stopframemovie");

        if stream {
            let path = text("file");
            let codec = match text("codec").as_str() {
                "h265" => "libx265",
                "prores" => "prores_ks",
                _ => "libx264",
            };
            let quality = node
                .param("quality")
                .map(|p| p.eval(eval).as_i64())
                .unwrap_or(75)
                .clamp(0, 100) as u32;
            let key = format!("{path}|{width}x{height}|{fps:.3}|{codec}|{quality}|{out_type}");
            if self.recorders.get(id).map(|r| r.key.as_str()) != Some(key.as_str()) {
                self.recorders.remove(id);
                match Recorder::start(key, &path, width, height, fps, quality, codec) {
                    Ok(r) => {
                        self.recorders.insert(id, r);
                        self.set_node_status(id, None);
                    }
                    Err(e) => {
                        self.set_node_status(id, Some(e));
                        return;
                    }
                }
            }
            if let Some(problem) = self.recorders.get(id).and_then(|r| r.problem()) {
                self.set_node_status(id, Some(problem));
            } else {
                let written = self.movie_out_frames_written(id);
                self.set_node_status(id, Some(format!("recording — {written} frames")));
            }
            self.pending_records.push(PendingRecord::Stream(id));
            return;
        }

        let base_path = text("file");
        let leading = node
            .param("leadingzerosdigits")
            .map(|p| p.eval(eval).as_i64())
            .unwrap_or(4)
            .clamp(1, 12) as u32;
        let start_n = node
            .param("n")
            .map(|p| p.eval(eval).as_i64())
            .unwrap_or(1)
            .max(0) as u32;
        if !self.movie_out_seq.contains_key(id) {
            self.movie_out_seq.insert(id, start_n);
        }
        let index = *self.movie_out_seq.get(id).unwrap();
        let unique = node
            .param("uniquesuff")
            .map(|p| p.eval(eval).as_bool())
            .unwrap_or(false);
        let path = if out_type.as_str() == "image" {
            let mut p = base_path.clone();
            if unique {
                p = record::sequence_path(&p, index, leading, &image_type);
            }
            p
        } else {
            record::sequence_path(&base_path, index, leading, &image_type)
        };
        self.pending_records.push(PendingRecord::Still {
            id,
            path,
            image_type,
        });
        let written = self.movie_out_frames_written(id);
        self.set_node_status(id, Some(format!("writing — {} frames queued", written + 1)));
    }
}
