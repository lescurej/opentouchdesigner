use otd_core::{Graph, NodeId};

use crate::Engines;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeMessageLevel {
    Error,
    Warning,
}

#[derive(Clone)]
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

impl Engines {
    pub fn node_message(&self, graph: &Graph, id: NodeId) -> Option<NodeMessage> {
        if let Some(text) = otd_core::missing_node_message(graph, id) {
            return Some(NodeMessage {
                level: NodeMessageLevel::Error,
                text,
            });
        }
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

#[cfg(test)]
#[path = "tests/node_message__tests.rs"]
mod tests;
