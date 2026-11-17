//! Cross-family cook implementation for [`crate::Engines`].


use otd_chop::engine::{ChannelStore, Network};
use otd_chop::ops::{INFO, PARAMETER};
use otd_core::{
    CookContext, CookError, Cooker, EvalContext, Family, Graph, NodeId,
};
use otd_dat::DatStore;
use otd_gpu::TopEngine;
use otd_sop::GeometryStore;


use crate::info;

use crate::engines::Engines;
use crate::geotext_comp;
use crate::network::FullNetwork;
use crate::scene_view::SceneView;
use crate::script_host::PythonScripts;

fn crossings(
    graph: &Graph,
    id: NodeId,
    top: &mut TopEngine,
    channels: &ChannelStore,
    dats: &DatStore,
    geometry: &GeometryStore,
) -> otd_core::Crossings {
    if let Some(cross) =
        info::crossing_from_converter_param(graph, id, top, channels, dats, geometry)
    {
        return vec![Some(cross)];
    }
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
                        samples: data.channels.iter().map(|c| c.copy_samples()).collect(),
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
                    top.want_chop_pixel_readback(src);
                    let (width, height, rgba) = top.chop_pixel_cache(src)?;
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
        if graph
            .get(id)
            .is_some_and(|n| n.op_type == otd_chop::io::AUDIO_MOVIE)
        {
            let path = graph.path(id);
            let eval = EvalContext {
                path: Some(&path),
                ..EvalContext::default()
            };
            let top_path = graph
                .node(id)
                .param("top")
                .map(|p| p.eval(&eval).as_str())
                .unwrap_or_default();
            if let Some(watch) = info::resolve_op_from(graph, id, &top_path) {
                if !deps.contains(&watch) {
                    deps.push(watch);
                }
            }
        }
        if graph.get(id).is_some_and(|n| n.op_type == "selectCHOP") {
            let paths = graph
                .node(id)
                .param("chop")
                .map(|p| p.eval(&EvalContext::default()).as_str())
                .unwrap_or_default();
            for src in info::resolve_chop_output_sources(graph, id, &paths) {
                if !deps.contains(&src) {
                    deps.push(src);
                }
            }
        }
        if graph
            .get(id)
            .is_some_and(|n| matches!(n.op_type.as_str(), "chopexecuteDAT" | "parameterexecuteDAT"))
        {
            let key = if graph.node(id).op_type == "chopexecuteDAT" {
                "chop"
            } else {
                "op"
            };
            let path = graph
                .node(id)
                .param(key)
                .map(|p| p.eval(&EvalContext::default()).as_str())
                .unwrap_or_default();
            if let Some(watch) = info::resolve_op_from(graph, id, &path) {
                if !deps.contains(&watch) {
                    deps.push(watch);
                }
            }
        }
        for src in info::converter_param_dependencies(graph, id) {
            if !deps.contains(&src) {
                deps.push(src);
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
        let info_cook = otd_core::cook_engine();
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
        let panel_snapshot = chop.io.panel_interaction.clone();

        match family {
            Family::Chop => {
                if crate::audio_partition::skip_frame_chop_cook(
                    &self.audio_partition,
                    self.audio_rt.is_armed(),
                    graph,
                    id,
                ) {
                    if let Some(data) = self.audio_rt.tap_chop(id) {
                        channels.insert(id, data);
                        return Ok(otd_core::CookOutput::Updated);
                    }
                    return Ok(otd_core::CookOutput::Unchanged);
                }
                let mut foreign = crossings(graph, id, top, channels, dats, geometry);
                let data = {
                    let net = FullNetwork {
                        chops: Network::with_panel(graph, channels, &panel_snapshot),
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
                    if graph
                        .get(id)
                        .is_some_and(|n| n.op_type == otd_chop::io::AUDIO_MOVIE)
                    {
                        if let Some(cross) =
                            info::audiomovie_playback_crossing(top, graph, id, &eval)
                        {
                            foreign.push(Some(cross));
                        }
                        if let Some(cross) =
                            info::audiomovie_soundtrack_crossing(top, graph, id, &eval)
                        {
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
                        chops: Network::with_panel(graph, channels, &panel_snapshot),
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
                    chops: Network::with_panel(graph, channels, &panel_snapshot),
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
                let keyboard_keys = chop.io.input.keys.clone();
                let data = {
                    let net = FullNetwork {
                        chops: Network::with_panel(graph, channels, &panel_snapshot),
                        dats,
                        python,
                    };
                    let path = graph.path(id);
                    let eval = ctx.eval_for_node(graph, id, &net, &path);
                    let scripts = PythonScripts { python };
                    let chop_table_src = info::choptodat_chop_table_src(graph, id, channels, &eval);
                    dat.cook_node(
                        graph,
                        id,
                        ctx,
                        &eval,
                        dats,
                        Some(&scripts),
                        foreign,
                        Some(&keyboard_keys),
                        chop_table_src,
                    )?
                };
                dats.insert(id, data);
                Ok(otd_core::CookOutput::Updated)
            }
            Family::Comp
                if graph
                    .get(id)
                    .is_some_and(|n| geotext_comp::is_geotext_type(&n.op_type)) =>
            {
                let net = FullNetwork {
                    chops: Network::with_panel(graph, channels, &panel_snapshot),
                    dats,
                    python,
                };
                let path = graph.path(id);
                let eval = ctx.eval_for_node(graph, id, &net, &path);
                let node = graph.node(id);
                let text = node
                    .param("text")
                    .map(|p| p.eval(&eval).as_str())
                    .unwrap_or_default();
                let scale = node
                    .param("scale")
                    .map(|p| p.eval(&eval).as_f64())
                    .unwrap_or(1.0) as f32;
                geometry.insert(id, otd_sop::geotext::cook_geometry(&text, scale));
                Ok(otd_core::CookOutput::Updated)
            }
            // COMPs hold sub-networks; the other families are later phases.
            _ => Ok(otd_core::CookOutput::Updated),
        }
    }
}
