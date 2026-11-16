use std::path::PathBuf;

use crate::graph::Graph;
use crate::tools::bundled_tool_directories;

pub const MOVIE_IN_DEFAULT_FILE: &str = "apple.png";

pub fn resolve_media_path(graph: &Graph, file: &str) -> PathBuf {
    let trimmed = file.trim();
    let name = if trimmed.is_empty() {
        MOVIE_IN_DEFAULT_FILE
    } else {
        trimmed
    };
    let via_project = graph.resolve_external(name);
    if via_project.is_file() {
        return via_project;
    }
    if trimmed.is_empty() || name == MOVIE_IN_DEFAULT_FILE {
        for dir in media_search_directories() {
            for candidate in [
                dir.join(MOVIE_IN_DEFAULT_FILE),
                dir.join("media").join(MOVIE_IN_DEFAULT_FILE),
            ] {
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    via_project
}

fn media_search_directories() -> Vec<PathBuf> {
    let mut out = bundled_tool_directories();
    if let Some(dev) = option_env!("OTD_MEDIA_DIR") {
        out.push(PathBuf::from(dev));
    }
    out
}

#[cfg(test)]
#[path = "tests/media__tests.rs"]
mod tests;
