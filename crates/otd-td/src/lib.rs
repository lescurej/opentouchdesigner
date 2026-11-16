//! TouchDesigner `.toe` / `.tox` decode (native → `toeexpand` → guidance).
//!
//! Step 1 of TD import: expand the container and report which operator types OTD
//! does not implement yet. Full graph import comes later.

mod analyze;
mod container;
mod cparm_import;
mod dat_text;
mod expansion;
mod gnode_import;
mod graph_import;
mod parm_import;
mod tea;
mod toeexpand;

pub use analyze::{
    ImportReport, MissingOperator, MissingParameter, TdNode, analyze_expansion, format_report_text,
    parse_node_type, td_to_otd_guess,
};
pub use container::{
    ContainerError, DecodeMeta, assess_expansion, container_offset, decode_container,
    is_toe_container,
};
pub use expansion::{load_dir, read_touchdesigner_file};
pub use graph_import::{
    ImportGraphError, ImportGraphResult, expansion_filesystem_root, import_into_graph,
};
pub use toeexpand::{ToeExpandError, expansion_dir, find_toeexpand, run_toeexpand, toeexpand_tool};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use otd_core::OpRegistry;

#[derive(Debug, Clone)]
pub enum DecodeSource {
    Native(DecodeMeta),
    ToeExpand { tool: PathBuf, dir: PathBuf },
}

#[derive(Debug, Clone)]
pub struct DecodedExpansion {
    pub source: DecodeSource,
    pub files: BTreeMap<String, Vec<u8>>,
    pub decode_ms: u64,
}

#[derive(Debug, Clone)]
pub struct FullImportReport {
    pub decode: DecodedExpansion,
    pub parity: ImportReport,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FailedImport {
    pub notes: Vec<String>,
}

/// Native decode, then `toeexpand`, then a human-readable failure note.
pub fn decode_with_fallback(path: &Path) -> Result<DecodedExpansion, FailedImport> {
    let mut notes = Vec::new();
    let bytes = match read_touchdesigner_file(path) {
        Ok(b) => b,
        Err(e) => {
            return Err(FailedImport {
                notes: vec![format!("could not read {}: {e}", path.display())],
            });
        }
    };

    let t0 = Instant::now();
    if is_toe_container(&bytes) {
        match decode_container(&bytes) {
            Ok((files, meta)) => match assess_expansion(&files) {
                Ok(()) => {
                    return Ok(DecodedExpansion {
                        source: DecodeSource::Native(meta),
                        files,
                        decode_ms: t0.elapsed().as_millis() as u64,
                    });
                }
                Err(reason) => {
                    notes.push(format!(
                        "native decode expanded but failed sanity check: {reason}"
                    ));
                }
            },
            Err(e) => notes.push(format!("native decode failed: {e}")),
        }
    } else {
        notes.push("file does not look like a TD container at known offsets".into());
    }

    match run_toeexpand(path) {
        Ok(dir) => {
            let tool = toeexpand_tool().unwrap_or_else(|| PathBuf::from("toeexpand"));
            match load_dir(&dir) {
                Ok(files) => match assess_expansion(&files) {
                    Ok(()) => Ok(DecodedExpansion {
                        source: DecodeSource::ToeExpand {
                            tool,
                            dir: dir.clone(),
                        },
                        files,
                        decode_ms: t0.elapsed().as_millis() as u64,
                    }),
                    Err(reason) => Err(FailedImport {
                        notes: {
                            notes.push(format!("toeexpand output failed sanity check: {reason}"));
                            notes
                        },
                    }),
                },
                Err(e) => Err(FailedImport {
                    notes: {
                        notes.push(format!("could not read {}: {e}", dir.display()));
                        notes
                    },
                }),
            }
        }
        Err(e) => Err(FailedImport {
            notes: {
                notes.push(format!("{e}"));
                notes.push(
                    "Install TouchDesigner and ensure toeexpand is on PATH, or use WebToe to \
                     inspect the file. Full import into .otd is not available yet."
                        .into(),
                );
                notes
            },
        }),
    }
}

pub fn inspect_touchdesigner(
    path: &Path,
    registry: &OpRegistry,
) -> Result<FullImportReport, FailedImport> {
    let decode = decode_with_fallback(path)?;
    let mut notes = Vec::new();
    let source_label = match &decode.source {
        DecodeSource::Native(meta) => format!(
            "native Rust decoder ({} segment(s), kinds {:?}, {} ms)",
            meta.segments, meta.kinds, decode.decode_ms
        ),
        DecodeSource::ToeExpand { tool, .. } => {
            format!("toeexpand ({}, {} ms)", tool.display(), decode.decode_ms)
        }
    };
    let parity = analyze_expansion(&decode.files, registry);
    if !parity.has_gaps() {
        notes.push(
            "All operator types and parameters are supported — import opens the base directly."
                .into(),
        );
    } else if parity.missing.is_empty() {
        notes.push("Some parameters have no matching OpenTouchDesigner parameter.".into());
    } else if parity.missing_params.is_empty() {
        notes.push(
            "Some operator types are missing in OTD — import keeps them as placeholders; implement ops or add TYPE_ALIASES."
                .into(),
        );
    } else {
        notes.push("Some operator types and parameters are missing in OTD.".into());
    }
    let _ = source_label;
    Ok(FullImportReport {
        decode,
        parity,
        notes,
    })
}

pub fn inspect_touchdesigner_text(
    path: &Path,
    registry: &OpRegistry,
) -> Result<String, FailedImport> {
    let report = inspect_touchdesigner(path, registry)?;
    let source = match &report.decode.source {
        DecodeSource::Native(meta) => format!(
            "native Rust decoder ({} segment(s), kinds {:?}, {} ms)",
            meta.segments, meta.kinds, report.decode.decode_ms
        ),
        DecodeSource::ToeExpand { tool, .. } => format!(
            "toeexpand ({}, {} ms)",
            tool.display(),
            report.decode.decode_ms
        ),
    };
    Ok(format_report_text(
        &path.display().to_string(),
        &source,
        &report.parity,
        &report.notes,
    ))
}
