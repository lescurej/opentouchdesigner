//! Exporting a project as a self-contained folder.
//!
//! A project is not one file. It references `.otdc` component files, and those
//! live wherever the author happened to keep them — a scratch folder, another
//! project's directory, a Dropbox path with their name in it. That is right
//! for authoring, where the whole point of a shared component is that two
//! projects use one copy of it, and wrong for a show machine, where anything
//! not in the folder you copied across is a file that will be missing at 8pm.
//!
//! An export copies every referenced component in beside the project and
//! rewrites the references to be relative to it. The result opens the same way
//! from any directory on any machine — which is what [`Project::open`] and the
//! graph's base directory exist to make true.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::graph::{Graph, OpRegistry};
use crate::project::{Project, ProjectEditor, ProjectError, ProjectSettings};

/// Where components go inside a bundle.
pub const COMPONENT_DIR: &str = "components";

/// Where movies, images and audio go inside a bundle.
pub const MEDIA_DIR: &str = "media";

/// Project timing and the authoring state [`export`] would otherwise drop.
///
/// [`Project::from_graph`] rebuilds a project from the live graph and does not
/// carry perform settings or the network editor camera.
pub struct ExportOptions<'a> {
    pub fps: f64,
    pub sample_rate: f64,
    pub settings: &'a ProjectSettings,
    pub editor: &'a ProjectEditor,
}

/// What an export produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub project: PathBuf,
    /// The component files copied in, in the order they were found.
    pub components: Vec<PathBuf>,
    /// The movies, images and audio copied in.
    pub media: Vec<PathBuf>,
    /// References that could not be read, as `(node path, file, reason)`.
    ///
    /// A missing component is reported rather than thrown: an artist who is
    /// mid-reorganisation should get a list of what to fix, not one error and
    /// no bundle.
    pub missing: Vec<(String, String, String)>,
}

/// Write `graph` and everything it references into `dir`.
///
/// The project lands at `dir/<name>.otd` and components at
/// `dir/components/<file>.otdc`. Two components with the same file name from
/// different folders are kept apart by numbering, because collapsing them
/// would silently point one instance at the other's network.
pub fn export(
    graph: &Graph,
    registry: &OpRegistry,
    fps: f64,
    sample_rate: f64,
    dir: impl AsRef<Path>,
    name: &str,
) -> Result<Bundle, ProjectError> {
    export_with(
        graph,
        registry,
        &ExportOptions {
            fps,
            sample_rate,
            settings: &ProjectSettings::default(),
            editor: &ProjectEditor::default(),
        },
        dir,
        name,
    )
}

/// Like [`export`], keeping the project's perform settings and the network
/// editor camera.
pub fn export_with(
    graph: &Graph,
    registry: &OpRegistry,
    options: &ExportOptions<'_>,
    dir: impl AsRef<Path>,
    name: &str,
) -> Result<Bundle, ProjectError> {
    let dir = dir.as_ref();
    std::fs::create_dir_all(dir)?;

    let mut copied: BTreeMap<PathBuf, String> = BTreeMap::new();
    let mut used_names: Vec<String> = Vec::new();
    let mut components = Vec::new();
    let mut missing = Vec::new();

    // The rewritten references are collected by node path and applied to the
    // *file* rather than to the graph. Exporting must not disturb the project
    // the artist is still working in, where those references are deliberately
    // pointing at shared files outside the folder.
    let mut rewritten: BTreeMap<String, String> = BTreeMap::new();

    for id in graph.walk() {
        let Some(file) = graph.node(id).external.clone() else {
            continue;
        };
        let source = graph.resolve_external(&file);

        // The same component used twice is one file in the bundle, and both
        // instances point at it.
        if let Some(existing) = copied.get(&source) {
            rewritten.insert(graph.path(id), existing.clone());
            continue;
        }

        let stem = Path::new(&file)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "component".into());
        let mut unique = stem.clone();
        let mut n = 1;
        while used_names.contains(&unique) {
            unique = format!("{stem}{n}");
            n += 1;
        }
        used_names.push(unique.clone());

        let relative = format!("{COMPONENT_DIR}/{unique}.otdc");
        let target = dir.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::copy(&source, &target) {
            Ok(_) => {
                copied.insert(source, relative.clone());
                components.push(target);
                rewritten.insert(graph.path(id), relative);
            }
            Err(e) => missing.push((graph.path(id), file, e.to_string())),
        }
    }

    // ---- Media. A project that references a movie is as dependent on it as
    // one that references a component, and by exactly the same argument: the
    // authoring path is somebody's scratch folder, and the show machine has
    // no such folder.
    let mut media = Vec::new();
    let mut media_names: Vec<String> = Vec::new();
    let mut media_copied: BTreeMap<PathBuf, String> = BTreeMap::new();
    // `(node path, param key) -> relative path`, applied to the file rather
    // than to the graph, for the same reason component references are.
    let mut media_rewritten: BTreeMap<(String, String), String> = BTreeMap::new();

    for id in graph.walk() {
        let node = graph.node(id);
        for (key, param) in &node.params {
            if !param.is_file_ref() {
                continue;
            }
            let value = param.value.as_str();
            if value.trim().is_empty() {
                continue;
            }
            let source = graph.resolve_external(value.trim());
            if let Some(existing) = media_copied.get(&source) {
                media_rewritten.insert((graph.path(id), key.clone()), existing.clone());
                continue;
            }

            let name = Path::new(value.trim())
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "media".into());
            let (stem, extension) = match name.rsplit_once('.') {
                Some((s, e)) => (s.to_string(), format!(".{e}")),
                None => (name.clone(), String::new()),
            };
            let mut unique = format!("{stem}{extension}");
            let mut n = 1;
            while media_names.contains(&unique) {
                unique = format!("{stem}{n}{extension}");
                n += 1;
            }
            media_names.push(unique.clone());

            let relative = format!("{MEDIA_DIR}/{unique}");
            let target = dir.join(&relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            match std::fs::copy(&source, &target) {
                Ok(_) => {
                    media_copied.insert(source, relative.clone());
                    media.push(target);
                    media_rewritten.insert((graph.path(id), key.clone()), relative);
                }
                Err(e) => missing.push((graph.path(id), value, e.to_string())),
            }
        }
    }

    let mut out = Project::from_graph(graph, registry, options.fps);
    out.sample_rate = crate::canonical_project_sample_rate(options.sample_rate);
    out.settings = options.settings.clone();
    out.editor = options.editor.clone();
    for entry in &mut out.nodes {
        if let Some(relative) = rewritten.get(&entry.path) {
            entry.external = Some(relative.clone());
        }
        for (key, param) in entry.params.iter_mut() {
            if let Some(relative) = media_rewritten.get(&(entry.path.clone(), key.clone())) {
                param.value = crate::value::Value::Str(relative.clone());
            }
        }
    }
    let project = dir.join(format!("{name}.otd"));
    out.save(&project)?;

    Ok(Bundle {
        project,
        components,
        media,
        missing,
    })
}

#[cfg(test)]
#[path = "tests/bundle__tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/bundle__media_tests.rs"]
mod media_tests;
