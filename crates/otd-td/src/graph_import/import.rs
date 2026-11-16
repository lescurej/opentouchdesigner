use std::collections::BTreeMap;
use std::path::Path;

use otd_core::graph::{Family, Graph, NodeId, OpRegistry};
use otd_core::missing::{finish_missing_substitution, resolve_op_def};
use otd_core::value::Value;

use crate::analyze::{parse_node_type, td_to_otd_guess};
use crate::parm_import::{apply_parsed_parms, parse_parm_block};

use super::parse::*;
use super::types::{ImportGraphError, ImportGraphResult};

/// Import one expanded component tree as a new COMP under `parent`.
pub fn import_into_graph(
    graph: &mut Graph,
    parent: NodeId,
    files: &BTreeMap<String, Vec<u8>>,
    base_dir: &Path,
    registry: &OpRegistry,
    component_name: &str,
) -> Result<ImportGraphResult, ImportGraphError> {
    let prefix = detect_prefix(files).ok_or(ImportGraphError::NoRootPrefix)?;
    let root_key = format!("{prefix}.n");
    let root_bytes = files
        .get(&root_key)
        .ok_or_else(|| ImportGraphError::NoRootDescriptor(prefix.clone()))?;
    let td_root =
        parse_node_type(root_bytes).ok_or_else(|| ImportGraphError::BadRootType("?".into()))?;
    let root_op = td_to_otd_guess(&td_root);
    let resolved = resolve_op_def(registry, &root_op)
        .ok_or_else(|| ImportGraphError::Registry(root_op.clone()))?;

    let root = graph
        .create(parent, resolved.def, Some(component_name))
        .map_err(|e| ImportGraphError::Registry(e.to_string()))?;
    finish_missing_substitution(graph, root, &resolved);

    let cparm_key = format!("{prefix}.cparm");
    if let Some(cparm) = files.get(&cparm_key) {
        let text = String::from_utf8_lossy(cparm);
        crate::cparm_import::apply_cparm_block(graph, root, &text);
    }

    let mut by_td_path: BTreeMap<String, NodeId> = BTreeMap::new();
    by_td_path.insert(prefix.clone(), root);

    let mut node_paths: Vec<String> = files
        .keys()
        .filter(|k| k.ends_with(".n") && **k != root_key && k.starts_with(&format!("{prefix}/")))
        .map(|k| k.strip_suffix(".n").unwrap().to_string())
        .collect();
    node_paths.sort_by_key(|p| p.matches('/').count());

    let mut warnings = Vec::new();
    let mut nodes = 0usize;
    let mut pending_docks: Vec<(NodeId, String, bool)> = Vec::new();

    for td_path in &node_paths {
        let Some((parent_td, name)) = split_td_path(td_path) else {
            continue;
        };
        let Some(&parent_id) = by_td_path.get(parent_td) else {
            warnings.push(format!("skip `{td_path}` — parent `{parent_td}` missing"));
            continue;
        };
        let key = format!("{td_path}.n");
        let bytes = files.get(&key).expect("key from list");
        let Some(td_type) = parse_node_type(bytes) else {
            warnings.push(format!("skip `{td_path}` — no operator type"));
            continue;
        };
        let otd_op = td_to_otd_guess(&td_type);
        let Some(resolved) = resolve_op_def(registry, &otd_op) else {
            warnings.push(format!(
                "skip `{td_path}` — no registry entry for `{otd_op}`"
            ));
            continue;
        };
        let id = graph
            .create(parent_id, resolved.def, Some(name))
            .map_err(|e| ImportGraphError::Registry(e.to_string()))?;
        finish_missing_substitution(graph, id, &resolved);
        nodes += 1;

        apply_n_file(graph, id, bytes);
        if let Some(dock) = parse_dock(&String::from_utf8_lossy(bytes)) {
            pending_docks.push((id, dock, parse_dock_shown(&String::from_utf8_lossy(bytes))));
        }
        if graph.node(id).family == Family::Comp {
            if let Some(cparm) = files.get(&format!("{td_path}.cparm")) {
                let text = String::from_utf8_lossy(cparm);
                crate::cparm_import::apply_cparm_block(graph, id, &text);
            }
        }
        let otd_op = graph.node(id).op_type.clone();
        if let Some(parm) = files.get(&format!("{td_path}.parm")) {
            let text = String::from_utf8_lossy(parm);
            apply_parsed_parms(graph, id, &otd_op, &parse_parm_block(&text), &mut warnings);
        }
        if let Some(text) = dat_text_sidecar(files, td_path) {
            if graph.node(id).params.contains_key("text") {
                let _ = graph.set_param(id, "text", Value::Str(text));
            }
        }
        if let Some(sidecar) = data_sidecar_rel(files, td_path, name) {
            if graph.node(id).params.contains_key("file") {
                let path = base_dir.join(&sidecar);
                let file = if path.is_file() {
                    path.to_string_lossy().into_owned()
                } else {
                    sidecar
                };
                let _ = graph.set_param(id, "file", Value::Str(file));
            }
        }
        let gnode_key = format!("{td_path}.gnode");
        if let Some(gnode) = files.get(&gnode_key) {
            let text = String::from_utf8_lossy(gnode);
            crate::gnode_import::apply_gnode_transform(graph, id, &text);
        }
        by_td_path.insert(td_path.clone(), id);
    }

    attach_docks(graph, &pending_docks);
    place_docked(graph);
    copy_glsl_pixel_sources(graph);

    graph.set_base_dir(Some(base_dir.to_path_buf()));

    let mut wires = 0usize;
    for td_path in &node_paths {
        let Some(&dst) = by_td_path.get(td_path) else {
            continue;
        };
        let key = format!("{td_path}.n");
        let Some(bytes) = files.get(&key) else {
            continue;
        };
        let text = String::from_utf8_lossy(bytes);
        let Some((parent_td, _)) = split_td_path(td_path) else {
            continue;
        };
        for (input_index, src_ref) in parse_inputs(&text) {
            let Some(src) = resolve_input_ref(graph, &prefix, parent_td, &src_ref, &by_td_path)
            else {
                warnings.push(format!(
                    "wire `{td_path}` input {input_index}: no `{src_ref}`"
                ));
                continue;
            };
            match graph.connect(src, dst, input_index) {
                Ok(()) => wires += 1,
                Err(e) => warnings.push(format!("wire `{td_path}` ← `{src_ref}`: {e}")),
            }
        }
    }

    for (td_path, &comp_id) in &by_td_path {
        if graph.node(comp_id).family != Family::Comp {
            continue;
        }
        let key = format!("{td_path}.network");
        let Some(bytes) = files.get(&key) else {
            continue;
        };
        let text = String::from_utf8_lossy(bytes);
        let parent_td = td_path
            .rsplit_once('/')
            .map(|(p, _)| p)
            .unwrap_or(prefix.as_str());
        for (input_index, src_ref) in parse_compinputs(&text) {
            let Some(src) = resolve_input_ref(graph, &prefix, parent_td, &src_ref, &by_td_path)
            else {
                warnings.push(format!(
                    "compinput `{td_path}` input {input_index}: no `{src_ref}`"
                ));
                continue;
            };
            match graph.connect(src, comp_id, input_index) {
                Ok(()) => wires += 1,
                Err(e) => warnings.push(format!("compinput `{td_path}` ← `{src_ref}`: {e}")),
            }
        }
    }

    Ok(ImportGraphResult {
        root,
        nodes: nodes + 1,
        wires,
        warnings,
    })
}
