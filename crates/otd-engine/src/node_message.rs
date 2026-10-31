use otd_core::{Graph, NodeId};

use crate::Engines;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeMessageLevel {
    Error,
    Warning,
}

pub struct NodeMessage {
    pub level: NodeMessageLevel,
    pub text: String,
}

pub fn is_movie_playback_telemetry(text: &str) -> bool {
    let t = text.trim().to_ascii_lowercase();
    t.starts_with("lag ") && t.contains('·')
}

pub fn classify_node_message(text: &str) -> NodeMessageLevel {
    let t = text.trim().to_ascii_lowercase();
    if t.starts_with("lag ") && t.contains('·') {
        return NodeMessageLevel::Warning;
    }
    if t.contains("pending") && t.contains("decode") {
        return NodeMessageLevel::Warning;
    }
    if t.contains("drops ") && t.starts_with("lag ") {
        return NodeMessageLevel::Warning;
    }
    NodeMessageLevel::Error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movie_lag_telemetry_is_not_a_node_message() {
        assert!(is_movie_playback_telemetry(
            "lag 2 · pending 1 · decode on · drops 0"
        ));
    }

    #[test]
    fn movie_lag_is_a_warning_not_an_error() {
        assert_eq!(
            classify_node_message("lag 2 · pending 1 · decode on · drops 0"),
            NodeMessageLevel::Warning
        );
    }

    #[test]
    fn missing_file_is_an_error() {
        assert_eq!(
            classify_node_message("could not open movie"),
            NodeMessageLevel::Error
        );
    }
}

impl Engines {
    pub fn node_message(&self, graph: &Graph, id: NodeId) -> Option<NodeMessage> {
        if let Some(err) = self.top.shader_error(id) {
            return Some(NodeMessage {
                level: NodeMessageLevel::Error,
                text: err.to_string(),
            });
        }
        let text = self.node_status(graph, id)?;
        if is_movie_playback_telemetry(&text) {
            return None;
        }
        Some(NodeMessage {
            level: classify_node_message(&text),
            text,
        })
    }
}
