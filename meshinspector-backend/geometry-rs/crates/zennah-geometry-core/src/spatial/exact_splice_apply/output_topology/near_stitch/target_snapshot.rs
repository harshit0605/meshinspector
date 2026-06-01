use super::super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::super::super::super::exact_meshlib_near_stitch::ExactMeshlibNearStitchEndpoint;
use super::super::OutputFaceTopology;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchTargetSnapshot {
    pub edge_id: usize,
    pub origin: Option<usize>,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub next_edge_id: usize,
    pub prev_edge_id: usize,
}

impl ExactMeshlibNearStitchTargetSnapshot {
    fn capture(topology: &ExactHalfEdgeTopology, edge: ExactHalfEdgeId) -> Self {
        Self {
            edge_id: edge.0,
            origin: topology.origin(edge),
            left: topology.left(edge),
            right: topology.right(edge),
            next_edge_id: topology.next(edge).0,
            prev_edge_id: topology.prev(edge).0,
        }
    }
}

impl OutputFaceTopology {
    pub(super) fn capture_meshlib_near_stitch_target_snapshot(
        &mut self,
        stitch_pair_index: usize,
        endpoint: ExactMeshlibNearStitchEndpoint,
        edge: ExactHalfEdgeId,
    ) {
        self.meshlib_near_stitch_target_snapshots
            .entry((stitch_pair_index, endpoint, edge))
            .or_insert_with(|| ExactMeshlibNearStitchTargetSnapshot::capture(&self.topology, edge));
    }

    pub(super) fn meshlib_near_stitch_target_snapshot(
        &self,
        stitch_pair_index: usize,
        endpoint: ExactMeshlibNearStitchEndpoint,
        edge: ExactHalfEdgeId,
    ) -> Option<ExactMeshlibNearStitchTargetSnapshot> {
        self.meshlib_near_stitch_target_snapshots
            .get(&(stitch_pair_index, endpoint, edge))
            .copied()
    }
}
