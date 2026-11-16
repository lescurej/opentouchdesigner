//! Compare a TD expansion against the OpenTouchDesigner operator registry.

use std::collections::{BTreeMap, BTreeSet};

use otd_core::OpRegistry;

#[derive(Debug, Clone)]
pub struct TdNode {
    pub path: String,
    pub td_type: String,
    pub otd_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MissingOperator {
    /// TouchDesigner `FAMILY:typename`, e.g. `TOP:geotext`.
    pub td_type: String,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MissingParameter {
    /// OpenTouchDesigner operator the parameter was written on.
    pub otd_op: String,
    /// TouchDesigner parameter name, before aliasing.
    pub td_key: String,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub nodes_total: usize,
    pub supported: usize,
    pub missing: Vec<MissingOperator>,
    pub missing_params: Vec<MissingParameter>,
    pub nodes: Vec<TdNode>,
}

impl ImportReport {
    /// True when the file names an operator or a parameter OpenTouchDesigner
    /// does not have. A clean report can be imported without a dialog.
    pub fn has_gaps(&self) -> bool {
        !self.missing.is_empty() || !self.missing_params.is_empty()
    }
}

/// TouchDesigner writes these on `.parm` files as editor or external-tox
/// bookkeeping. They are not operator parameters, so they are not gaps.
fn is_editor_parm(key: &str) -> bool {
    matches!(
        key,
        "pageindex"
            | "enableexternaltox"
            | "externaltox"
            | "externaltoxreadonly"
            | "reloadtoxonstart"
            | "savebackup"
    )
}

/// Known TD tokens that do not follow `{name}{FAMILY}` in OTD.
const TYPE_ALIASES: &[(&str, &str)] = &[
    ("COMP:geo", "geometryCOMP"),
    ("COMP:cam", "cameraCOMP"),
    ("COMP:light", "lightCOMP"),
    ("COMP:ambient", "ambientCOMP"),
    ("COMP:base", "baseCOMP"),
    ("COMP:container", "baseCOMP"),
    ("COMP:null", "baseCOMP"),
    ("TOP:moviefilein", "movieFileInTOP"),
    ("TOP:videodevin", "videoDeviceInTOP"),
    ("TOP:render", "renderTOP"),
    ("TOP:geotext", "textTOP"),
    ("CHOP:audiomovie", "audioMovieCHOP"),
    ("CHOP:audiodevicein", "audioDeviceInCHOP"),
    ("CHOP:dmxin", "dmxInCHOP"),
    ("CHOP:dmxout", "dmxOutCHOP"),
    ("CHOP:artnet", "artnetCHOP"),
    ("CHOP:par", "parameterCHOP"),
    ("CHOP:reorder", "reorderCHOP"),
    ("DAT:chopexec", "chopexecuteDAT"),
    ("SOP:filein", "fileinSOP"),
    ("SOP:importselect", "importselectSOP"),
    ("SOP:material", "materialSOP"),
    ("COMP:geotext", "geotextCOMP"),
    ("DAT:parexec", "parameterexecuteDAT"),
    ("TOP:res", "resolutionTOP"),
    ("CHOP:topto", "toptochopCHOP"),
    ("DAT:datexec", "datExecuteDAT"),
    ("SOP:point", "pointSOP"),
    ("DAT:text", "textDAT"),
    ("CHOP:fan", "fanCHOP"),
    ("CHOP:null", "nullCHOP"),
    ("CHOP:in", "inCHOP"),
    ("CHOP:out", "outCHOP"),
    ("CHOP:rename", "renameCHOP"),
    ("CHOP:select", "selectCHOP"),
    ("CHOP:switch", "switchCHOP"),
    ("CHOP:constant", "constantCHOP"),
    ("MAT:phong", "phongMAT"),
    ("MAT:constant", "constantMAT"),
    ("TOP:thresh", "thresholdTOP"),
    ("DAT:info", "infoDAT"),
];

pub fn parse_node_type(n_file: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(n_file).ok()?;
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("DAT:")
            && !rest.is_empty()
        {
            return Some(format!("DAT:{rest}"));
        }
        if let Some(rest) = l.strip_prefix("TOP:")
            && !rest.is_empty()
        {
            return Some(format!("TOP:{rest}"));
        }
        if let Some(rest) = l.strip_prefix("CHOP:")
            && !rest.is_empty()
        {
            return Some(format!("CHOP:{rest}"));
        }
        if let Some(rest) = l.strip_prefix("SOP:")
            && !rest.is_empty()
        {
            return Some(format!("SOP:{rest}"));
        }
        if let Some(rest) = l.strip_prefix("MAT:")
            && !rest.is_empty()
        {
            return Some(format!("MAT:{rest}"));
        }
        if let Some(rest) = l.strip_prefix("COMP:")
            && !rest.is_empty()
        {
            return Some(format!("COMP:{rest}"));
        }
        if let Some(rest) = l.strip_prefix("POP:")
            && !rest.is_empty()
        {
            return Some(format!("POP:{rest}"));
        }
    }
    None
}

pub fn td_to_otd_guess(td_type: &str) -> String {
    let Some((family, ty)) = td_type.split_once(':') else {
        return td_type.to_string();
    };
    for (from, to) in TYPE_ALIASES {
        if *from == td_type {
            return (*to).to_string();
        }
    }
    format!("{ty}{family}")
}

pub fn analyze_expansion(files: &BTreeMap<String, Vec<u8>>, registry: &OpRegistry) -> ImportReport {
    let mut nodes = Vec::new();
    let mut by_td: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (path, bytes) in files {
        if !path.ends_with(".n") {
            continue;
        }
        let Some(td_type) = parse_node_type(bytes) else {
            continue;
        };
        let otd_guess = td_to_otd_guess(&td_type);
        let supported = registry.get(&otd_guess).is_some();
        let otd_type = supported.then_some(otd_guess);
        nodes.push(TdNode {
            path: path.clone(),
            td_type: td_type.clone(),
            otd_type,
        });
        by_td.entry(td_type).or_default().push(path.clone());
    }

    let mut missing = Vec::new();
    let mut supported = 0usize;
    for node in &nodes {
        if node.otd_type.is_some() {
            supported += 1;
        }
    }
    for (td_type, paths) in &by_td {
        let otd = td_to_otd_guess(td_type);
        if registry.get(&otd).is_some() {
            continue;
        }
        let examples: Vec<String> = paths.iter().take(5).cloned().collect();
        missing.push(MissingOperator {
            td_type: td_type.clone(),
            count: paths.len(),
            examples,
        });
    }
    missing.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.td_type.cmp(&b.td_type))
    });

    let missing_params = missing_parameters(files, &nodes, registry);

    ImportReport {
        nodes_total: nodes.len(),
        supported,
        missing,
        missing_params,
        nodes,
    }
}

fn missing_parameters(
    files: &BTreeMap<String, Vec<u8>>,
    nodes: &[TdNode],
    registry: &OpRegistry,
) -> Vec<MissingParameter> {
    use std::collections::BTreeMap as Map;

    let mut hits: Map<(String, String), Vec<String>> = Map::new();
    for node in nodes {
        let Some(otd_op) = node.otd_type.as_deref() else {
            continue;
        };
        let Some(def) = registry.get(otd_op) else {
            continue;
        };
        let known: std::collections::BTreeSet<String> = (def.params)().into_keys().collect();
        let stem = node.path.strip_suffix(".n").unwrap_or(node.path.as_str());
        let Some(parm_bytes) = files.get(&format!("{stem}.parm")) else {
            continue;
        };
        let custom: std::collections::BTreeSet<String> = files
            .get(&format!("{stem}.cparm"))
            .map(|bytes| {
                crate::cparm_import::custom_param_names(&String::from_utf8_lossy(bytes))
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default();
        let text = String::from_utf8_lossy(parm_bytes);
        for parm in crate::parm_import::parse_parm_block(&text) {
            let td_key = parm.key.as_str();
            if is_editor_parm(td_key) || custom.contains(td_key) {
                continue;
            }
            let covered = crate::parm_import::parm_target_keys(otd_op, td_key)
                .iter()
                .any(|key| known.contains(key));
            if covered {
                continue;
            }
            hits.entry((otd_op.to_string(), td_key.to_string()))
                .or_default()
                .push(stem.to_string());
        }
    }

    let mut missing: Vec<MissingParameter> = hits
        .into_iter()
        .map(|((otd_op, td_key), paths)| MissingParameter {
            otd_op,
            td_key,
            count: paths.len(),
            examples: paths.into_iter().take(5).collect(),
        })
        .collect();
    missing.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.otd_op.cmp(&b.otd_op))
            .then_with(|| a.td_key.cmp(&b.td_key))
    });
    missing
}

pub fn format_report_text(
    file_label: &str,
    source: &str,
    report: &ImportReport,
    notes: &[String],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("TouchDesigner import report — {file_label}\n"));
    out.push_str(&format!("Decoded via: {source}\n\n"));
    out.push_str(&format!(
        "Nodes: {} total, {} supported in OpenTouchDesigner, {} missing operator type(s), {} missing parameter(s)\n\n",
        report.nodes_total,
        report.supported,
        report.missing.len(),
        report.missing_params.len()
    ));
    if report.missing.is_empty() {
        out.push_str("All operator types in this file exist in OTD.\n");
    } else {
        out.push_str("Missing operators (by TouchDesigner type):\n");
        for m in &report.missing {
            out.push_str(&format!("  • {} ×{}", m.td_type, m.count));
            if !m.examples.is_empty() {
                out.push_str(&format!(" — e.g. {}", m.examples.join(", ")));
            }
            out.push('\n');
        }
        out.push_str("\nThese are parity gaps: implement the operator or add a TYPE_ALIASES mapping in otd-td.\n");
    }
    if report.missing_params.is_empty() {
        out.push_str("All imported parameters match an OpenTouchDesigner parameter.\n");
    } else {
        out.push_str(
            "\nMissing parameters (no matching OTD parameter).\n\
             Only values written in the file are listed — TouchDesigner omits parameters left at their default:\n",
        );
        for m in &report.missing_params {
            out.push_str(&format!("  • {}.{} ×{}", m.otd_op, m.td_key, m.count));
            if !m.examples.is_empty() {
                out.push_str(&format!(" — e.g. {}", m.examples.join(", ")));
            }
            out.push('\n');
        }
    }
    if !notes.is_empty() {
        out.push_str("\nNotes:\n");
        for n in notes {
            out.push_str(&format!("  • {n}\n"));
        }
    }
    let unique: BTreeSet<_> = report
        .missing
        .iter()
        .map(|m| td_to_otd_guess(&m.td_type))
        .collect();
    if !unique.is_empty() {
        out.push_str("\nGuessed OTD names (for tracking):\n");
        for name in unique {
            out.push_str(&format!("  • {name}\n"));
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/analyze__tests.rs"]
mod tests;
