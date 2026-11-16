use otd_core::{Graph, NodeId};

/// The wires that keep a chain joined when a run of nodes is taken out of it.
///
/// Without this, "make it simpler" deletes three operators out of the middle
/// of a chain and leaves the survivors with an empty input 0 — a black
/// texture, and a patch that looks like the delete broke it.
///
/// Read-only, and worked out for the whole `doomed` set at once, because the
/// answer depends on the graph *before* any of them are removed. Returns
/// `(source, consumer, slot)` triples to run once they are gone.
pub fn splices_for(graph: &Graph, doomed: &[NodeId]) -> Vec<(NodeId, NodeId, usize)> {
    let mut out = Vec::new();
    for id in doomed {
        // Walk back past anything that is also going, so the two surviving
        // ends meet rather than one of them joining a corpse.
        let mut source = graph.node(*id).inputs.first().copied().flatten();
        while let Some(src) = source {
            if !doomed.contains(&src) {
                break;
            }
            source = graph.node(src).inputs.first().copied().flatten();
        }
        let Some(src) = source else { continue };
        for consumer in graph.consumers(*id) {
            if doomed.contains(&consumer) {
                continue;
            }
            for (slot, wired) in graph.node(consumer).inputs.iter().enumerate() {
                if *wired == Some(*id) {
                    out.push((src, consumer, slot));
                }
            }
        }
    }
    out
}
