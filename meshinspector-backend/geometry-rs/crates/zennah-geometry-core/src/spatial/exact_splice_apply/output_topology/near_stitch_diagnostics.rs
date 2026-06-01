use super::super::super::exact_halfedge::ExactHalfEdgeId;
use super::OutputFaceTopology;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactMeshlibNearStitchRingDiagnostic {
    pub edge_ids: Vec<usize>,
    pub origins: Vec<Option<usize>>,
    pub left_faces: Vec<Option<usize>>,
    pub error: Option<&'static str>,
}

impl OutputFaceTopology {
    pub(super) fn meshlib_near_stitch_left_ring_diagnostic(
        &self,
        start: ExactHalfEdgeId,
    ) -> ExactMeshlibNearStitchRingDiagnostic {
        match self.topology.left_ring_edges(start) {
            Ok(edges) => ExactMeshlibNearStitchRingDiagnostic {
                edge_ids: edges.iter().map(|edge| edge.0).collect(),
                origins: edges
                    .iter()
                    .map(|edge| self.topology.origin(*edge))
                    .collect(),
                left_faces: edges.iter().map(|edge| self.topology.left(*edge)).collect(),
                error: None,
            },
            Err(error) => ExactMeshlibNearStitchRingDiagnostic {
                edge_ids: Vec::new(),
                origins: Vec::new(),
                left_faces: Vec::new(),
                error: Some(error),
            },
        }
    }
}
