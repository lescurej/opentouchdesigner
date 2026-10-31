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
mod tests {
    use super::*;
    use crate::Graph;

    #[test]
    fn default_movie_file_resolves_from_shipped_media_dir() {
        let graph = Graph::new();
        let path = resolve_media_path(&graph, "");
        assert!(
            path.is_file(),
            "expected {} on disk, got {}",
            MOVIE_IN_DEFAULT_FILE,
            path.display()
        );
        assert_eq!(path.file_name().and_then(|n| n.to_str()), Some(MOVIE_IN_DEFAULT_FILE));
    }

    #[test]
    fn project_relative_path_wins_over_bundled_default() {
        let dir = std::env::temp_dir().join(format!("otd-media-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("mine.png");
        std::fs::write(&clip, b"fake").unwrap();
        let mut graph = Graph::new();
        graph.set_base_dir(Some(dir.clone()));
        let path = resolve_media_path(&graph, "mine.png");
        assert_eq!(path, clip);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
