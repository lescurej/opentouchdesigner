
use otd_core::{CookContext, CookError, CookOutput, EvalContext, Graph, NodeId};

use crate::ops::{self};
use crate::scene::Scene;

use super::super::dispatch::{video_in_idle, video_in_kind, Pass, PipelineRef};
use super::super::movie_params::{
    movie_cue_frame, movie_decode_tuning, movie_effective_fps, movie_length_frames,
    movie_map_index, movie_open_start_index, movie_preread_frames, movie_specify_index,
    movie_timecode_index, movie_trim_range,
};
use super::super::{
    NodeGpu, TopEngine, Upload,
    VideoInKind,
};

impl TopEngine {
    pub(in crate::engine) fn cook_video(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        scene: Option<&dyn Scene>,
        label: &str,
    ) -> Result<CookOutput, CookError> {
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let kind = video_in_kind(&node.op_type).expect("cook_video on a video TOP");
        let live = kind != VideoInKind::File;
        let get = |key: &str| node.param(key).map(|p| p.eval(eval));
        let fallback = ops::generator_size(node, eval);

        // ---- What is being asked for, as one string. A change to it is
        // what reopens the source, so nothing else has to track staleness.
        let (key, file) = match kind {
            VideoInKind::Camera => {
                let device = get("device").map(|v| v.as_str()).unwrap_or_default();
                let fps = get("fps")
                    .map(|v| v.as_f64())
                    .unwrap_or(30.0)
                    .clamp(1.0, 240.0);
                (
                    format!("camera:{device}:{}x{}:{fps}", fallback.0, fallback.1),
                    device.to_string(),
                )
            }
            VideoInKind::Screen => {
                let screen = get("screen").map(|v| v.as_i64()).unwrap_or(0).max(0) as u32;
                let fps = get("fps")
                    .map(|v| v.as_f64())
                    .unwrap_or(30.0)
                    .clamp(1.0, 120.0);
                (
                    format!("screen:{screen}:{}x{}:{fps}", fallback.0, fallback.1),
                    screen.to_string(),
                )
            }
            VideoInKind::Stream => {
                let url = get("url").map(|v| v.as_str()).unwrap_or_default();
                (format!("stream:{url}"), url.to_string())
            }
            VideoInKind::File => {
                let file = get("file").map(|v| v.as_str()).unwrap_or_default();
                let resolved = otd_core::media::resolve_media_path(graph, &file)
                    .display()
                    .to_string();
                (format!("file:{resolved}"), resolved)
            }
        };

        let active = get("active").map(|v| v.as_bool()).unwrap_or(true);
        let playmode_early = if live {
            String::new()
        } else {
            get("playmode")
                .map(|v| v.as_str())
                .unwrap_or_else(|| "sequential".into())
        };
        if kind == VideoInKind::File {
            let always = get("alwaysloadinitial")
                .map(|v| v.as_bool())
                .unwrap_or(true);
            let playing = get("play").map(|v| v.as_bool()).unwrap_or(true);
            if !always
                && playmode_early == "sequential"
                && !playing
                && self.videos.get(id).is_none()
            {
                self.ensure_output(id, fallback.0, fallback.1);
                return self
                    .clear_to_black(id, ctx, label)
                    .map(|_| CookOutput::Updated);
            }
        }
        if !active || video_in_idle(kind, &file) {
            // Nothing asked for: release the device and show black at the
            // fallback size, so a downstream chain still has a shape.
            self.release_video(id);
            self.set_node_status(id, None);
            self.ensure_output(id, fallback.0, fallback.1);
            return self
                .clear_to_black(id, ctx, label)
                .map(|_| CookOutput::Updated);
        }

        if kind == VideoInKind::File {
            let reload_now = get("reload").map(|v| v.as_bool()).unwrap_or(false);
            let was = self.movie_ui.get(id).map(|e| e.reload).unwrap_or(false);
            if reload_now && !was {
                self.release_video(id);
            }
            self.movie_edge(id).reload = reload_now;
        }

        // ---- Open, if this is a different source from the one running.
        if self.videos.get(id).map(|s| s.key.as_str()) != Some(key.as_str()) {
            self.release_video(id);
            let opened = match kind {
                VideoInKind::Camera => {
                    let fps = get("fps")
                        .map(|v| v.as_f64())
                        .unwrap_or(30.0)
                        .clamp(1.0, 240.0);
                    crate::video::Source::camera(key.clone(), &file, fallback.0, fallback.1, fps)
                }
                VideoInKind::Screen => {
                    let screen = file.parse::<u32>().unwrap_or(0);
                    let fps = get("fps")
                        .map(|v| v.as_f64())
                        .unwrap_or(30.0)
                        .clamp(1.0, 120.0);
                    crate::video::Source::screen(key.clone(), screen, fallback.0, fallback.1, fps)
                }
                VideoInKind::Stream => crate::video::Source::stream(key.clone(), &file),
                VideoInKind::File => {
                    if crate::video::is_still(&file) {
                        Ok(crate::video::Source::still(
                            key.clone(),
                            std::path::Path::new(&file),
                        ))
                    } else {
                        match crate::video::probe(&file) {
                            Some(info) => {
                                let start = movie_open_start_index(node, eval, &info).max(0);
                                match crate::video::Source::file(key.clone(), &file, info, start) {
                                    Ok(mut source) => {
                                        let (hw, pre, cpus) = movie_decode_tuning(node, eval);
                                        source.set_decode_tuning(
                                            hw,
                                            movie_preread_frames(&playmode_early, pre),
                                            cpus,
                                        );
                                        Ok(source)
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            None if !std::path::Path::new(&file).exists() => {
                                Err(format!("no file at {file}"))
                            }
                            None if !crate::video::tools_installed() => Err(format!(
                                "could not read {file} — {}",
                                crate::video::missing_ffmpeg()
                            )),
                            None => Err(format!(
                                "could not read {file} — not a video ffmpeg can decode"
                            )),
                        }
                    }
                }
            };
            match opened {
                Ok(source) => {
                    self.set_node_status(id, None);
                    self.videos.insert(id, source);
                }
                Err(e) => {
                    self.set_node_status(id, Some(e));
                    self.ensure_output(id, fallback.0, fallback.1);
                    return self
                        .clear_to_black(id, ctx, label)
                        .map(|_| CookOutput::Updated);
                }
            }
        }

        if kind == VideoInKind::File {
            if let Some(source) = self.videos.get_mut(id) {
                let (hw, pre, cpus) = movie_decode_tuning(node, eval);
                source.set_decode_tuning(hw, movie_preread_frames(&playmode_early, pre), cpus);
            }
            if self.videos.contains_key(id) {
                self.attach_soundtrack(id, &file);
            }
        }

        let speed = get("speed").map(|v| v.as_f64()).unwrap_or(1.0);
        let interp = get("interp").map(|v| v.as_bool()).unwrap_or(false);
        let playmode = playmode_early.clone();
        let extend_left = get("extendleft")
            .map(|v| v.as_str())
            .unwrap_or_else(|| "hold".into());
        let extend_right = get("extendright")
            .map(|v| v.as_str())
            .unwrap_or_else(|| "cycle".into());
        let step = get("stepsize").map(|v| v.as_i64()).unwrap_or(1).max(1);

        // Cue (held): stay on Cue Point. Cue Pulse (rising edge): jump once,
        // then keep playing. Releasing Cue leaves the playhead on that point
        // so the next frame runs forward from there.
        let mut hold_at_cue = false;
        let mut jump_to_cue = false;
        let mut cue_index: Option<i64> = None;
        let mut reset_sequential_playback = false;
        if kind == VideoInKind::File {
            let fps = self
                .videos
                .get(id)
                .map(|s| movie_effective_fps(node, eval, &s.info))
                .unwrap_or_else(|| {
                    node.param("rate")
                        .map(|p| p.eval(eval).as_f64())
                        .unwrap_or(60.0)
                        .max(1e-6)
                });
            let length = self
                .videos
                .get(id)
                .map(|s| movie_length_frames(&s.info, fps))
                .unwrap_or(1);
            cue_index = Some(movie_cue_frame(node, eval, fps, length));

            let cue_now = get("cue").map(|v| v.as_bool()).unwrap_or(false);
            let cue_pulse = get("cuepulse").map(|v| v.as_bool()).unwrap_or(false);
            let play_now = get("play").map(|v| v.as_bool()).unwrap_or(true);
            hold_at_cue = cue_now;

            let edge = self.movie_edge(id);
            let pulse_rise = cue_pulse && !edge.cue_pulse;
            let cue_release = edge.cue && !cue_now;
            jump_to_cue = hold_at_cue || pulse_rise || cue_release;
            edge.cue = cue_now;
            edge.cue_pulse = cue_pulse;
            edge.play = play_now;
            if edge.last_playmode.is_empty() {
                edge.last_playmode = playmode.clone();
            } else if edge.last_playmode != playmode {
                reset_sequential_playback = playmode == "sequential";
                edge.last_playmode = playmode.clone();
            }
        }
        if reset_sequential_playback {
            if let Some(s) = self.videos.get_mut(id) {
                s.reset_sequential_playback();
            }
        }

        let playback_active = if live {
            false
        } else {
            let playing = get("play").map(|v| v.as_bool()).unwrap_or(true);
            playing && !hold_at_cue
        };
        if kind == VideoInKind::File {
            if let Some(source) = self.videos.get_mut(id) {
                source.set_snap_index(playmode.as_str() == "specify");
                source.set_movie_playing(playback_active);
                source.playback_speed = speed;
            }
        }

        let (wanted, ended_message) = {
            let source = &mut self.videos[id];
            if live || source.info.fps <= 0.0 {
                (0, false)
            } else {
                let fps = movie_effective_fps(node, eval, &source.info);
                let length = (source.info.duration * fps).round().max(1.0) as i64;
                let (trim_start, trim_end) = movie_trim_range(node, eval, fps, length);

                let raw = if jump_to_cue {
                    let idx = cue_index.unwrap_or(0);
                    if playmode == "sequential" {
                        source.cue_sequential_to(idx, ctx.abs_time);
                    } else {
                        source.seek_playhead_to(idx);
                    }
                    idx
                } else {
                    match playmode.as_str() {
                        "specify" => {
                            let idx = movie_specify_index(node, eval, fps, length);
                            source.seek_playhead_to(idx);
                            idx
                        }
                        "sequential" => {
                            let dt = source.sequential_wall_dt(ctx.abs_time, ctx.dt.max(0.0), 0.25);
                            source.wanted_index_sequential(
                                eval.time,
                                dt,
                                speed,
                                playback_active,
                                interp,
                                fps,
                            )
                        }
                        "timecodeop" => {
                            let locked = source.wanted_index_locked(eval.time, speed, fps, interp);
                            let idx = movie_timecode_index(node, eval, scene, fps, length, locked);
                            source.seek_playhead_to(idx);
                            idx
                        }
                        "locked" => source.wanted_index_locked(eval.time, speed, fps, interp),
                        _ => source.wanted_index_locked(eval.time, speed, fps, interp),
                    }
                };

                let stepped = if step > 1 { (raw / step) * step } else { raw };
                movie_map_index(stepped, trim_start, trim_end, &extend_left, &extend_right)
            }
        };

        if kind == VideoInKind::File {
            if let Some(source) = self.videos.get_mut(id) {
                let fps = if source.info.fps > 0.0 {
                    movie_effective_fps(node, eval, &source.info)
                } else {
                    1.0
                };
                let length = if source.info.fps > 0.0 {
                    (source.info.duration * fps).round().max(1.0) as i64
                } else {
                    1
                };
                source.set_playback_report(wanted, length, fps, ended_message);
            }
        }

        let path_for_seek = file.clone();
        let frame = self.videos[id].advance(&path_for_seek, wanted).cloned();
        let revision = self.videos[id].revision;

        let Some(frame) = frame else {
            // Still decoding the first frame — one or two frames of a fresh
            // patch, not an error — unless the worker reported why not.
            let problem = self.videos[id].problem();
            self.set_node_status(id, problem);
            self.ensure_output(id, fallback.0, fallback.1);
            return self
                .clear_to_black(id, ctx, label)
                .map(|_| CookOutput::Updated);
        };

        // A frame arrived, so whatever was last complained about is over.
        // Status was only ever written on the no-frame path, which meant it
        // latched: a camera that took longer than the grace period to start —
        // which is exactly what happens while the permission prompt is on
        // screen — kept saying it had produced no frames while visibly
        // producing them.
        self.set_node_status(id, self.videos[id].problem());

        // `once` past the end shows black rather than freezing, so a clip
        // that has finished looks finished.
        let out_size = ops::movie_output_dimensions(node, eval, (frame.width, frame.height));
        if ended_message {
            self.ensure_output(id, out_size.0, out_size.1);
            return self
                .clear_to_black(id, ctx, label)
                .map(|_| CookOutput::Updated);
        }

        let same_frame = self.nodes[id]
            .upload
            .as_ref()
            .is_some_and(|u| u.revision == revision)
            && self.nodes[id].output.is_some();
        if same_frame {
            return Ok(CookOutput::Unchanged);
        }

        let update = get("updateimage").map(|v| v.as_bool()).unwrap_or(true);
        if update || self.nodes[id].upload.is_none() {
            self.upload_frame(id, &frame, revision);
        }

        if self.nodes[id].upload.is_none() {
            self.ensure_output(id, out_size.0, out_size.1);
            return self
                .clear_to_black(id, ctx, label)
                .map(|_| CookOutput::Updated);
        }

        self.ensure_output(id, out_size.0, out_size.1);
        let source_view = self.nodes[id].upload.as_ref().unwrap().view.clone();
        let out = self.nodes[id].output.as_ref().unwrap().view.clone();
        let nearest = ops::movie_sampler_nearest(node, eval);
        let use_fit = out_size != (frame.width, frame.height);
        let params = if use_fit {
            let mut p = ops::NO_PARAMS;
            p[0][0] = ops::movie_fit_mode(node, eval);
            p
        } else {
            ops::NO_PARAMS
        };
        let uniforms = self.uniforms(out_size.0, out_size.1, ctx, params);
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
            in0: &source_view,
            in1: &dummy,
            nearest,
        })
        .map(|_| CookOutput::Updated)
    }
    pub(super) fn upload_frame(&mut self, id: NodeId, frame: &crate::video::Frame, revision: u64) {
        self.upload_pixels(
            id,
            frame.width,
            frame.height,
            wgpu::TextureFormat::Rgba8Unorm,
            &frame.pixels,
            revision,
        );
    }
    pub(super) fn upload_pixels(
        &mut self,
        id: NodeId,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        bytes: &[u8],
        revision: u64,
    ) {
        let (width, height) = (width.max(1), height.max(1));
        if !self.nodes.contains_key(id) {
            self.nodes.insert(id, NodeGpu::default());
        }
        let reusable = self
            .nodes
            .get(id)
            .and_then(|entry| {
                entry
                    .upload
                    .as_ref()
                    .map(|u| u.width == width && u.height == height && u.format == format)
            })
            .unwrap_or(false);
        if reusable
            && revision != u64::MAX
            && self.nodes[id]
                .upload
                .as_ref()
                .is_some_and(|u| u.revision == revision)
        {
            return; // Nothing new to send.
        }
        if !reusable {
            let texture = self.ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("otd upload"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.nodes[id].upload = Some(Upload {
                texture,
                view,
                width,
                height,
                format,
                revision: u64::MAX,
            });
        }
        let bytes_per_texel = format.block_copy_size(None).unwrap_or(4);
        let upload = self.nodes[id].upload.as_mut().unwrap();
        upload.revision = revision;
        self.ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &upload.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * bytes_per_texel),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}
