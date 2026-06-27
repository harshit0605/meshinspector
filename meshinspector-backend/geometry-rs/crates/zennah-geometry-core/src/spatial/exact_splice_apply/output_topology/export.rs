use super::super::super::exact_boolean::ExactBooleanOperand;
use super::super::super::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use super::OutputFaceTopology;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactMeshlibFaceExportFailureDiagnostic {
    pub face_index: usize,
    pub face_edge_id: usize,
    pub face_operand: Option<ExactBooleanOperand>,
    pub face_cut_face: Option<usize>,
    pub face_source_face: Option<usize>,
    pub error: &'static str,
    pub left_ring_edge_ids: Vec<usize>,
    pub left_ring_record_next_edge_ids: Vec<usize>,
    pub left_ring_record_prev_edge_ids: Vec<usize>,
    pub left_ring_next_edge_ids: Vec<usize>,
    pub left_ring_origins: Vec<Option<usize>>,
    pub left_ring_destinations: Vec<Option<usize>>,
    pub left_ring_left_faces: Vec<Option<usize>>,
    pub left_ring_right_faces: Vec<Option<usize>>,
    pub same_left_face_edge_ids: Vec<usize>,
    pub same_left_face_record_next_edge_ids: Vec<usize>,
    pub same_left_face_record_prev_edge_ids: Vec<usize>,
    pub same_left_face_next_edge_ids: Vec<usize>,
    pub same_left_face_origins: Vec<Option<usize>>,
    pub same_left_face_destinations: Vec<Option<usize>>,
    pub same_left_face_right_faces: Vec<Option<usize>>,
    pub left_ring_repeated_edge_id: Option<usize>,
    pub left_ring_returned_to_start: bool,
}

struct LeftRingTrace {
    edges: Vec<ExactHalfEdgeId>,
    next_edges: Vec<ExactHalfEdgeId>,
    repeated_edge: Option<ExactHalfEdgeId>,
}

impl OutputFaceTopology {
    pub(crate) fn export_face_failure_details(
        &self,
    ) -> Vec<ExactMeshlibFaceExportFailureDiagnostic> {
        self.export_face_results()
            .into_iter()
            .enumerate()
            .filter_map(|(face_index, result)| {
                let error = result.err()?;
                let face_edge = self.face_edges[face_index];
                let left_ring_trace = self.trace_left_ring_edges(face_edge);
                let same_left_face_edges = self
                    .topology
                    .edge_ids()
                    .filter(|edge| self.topology.left(*edge) == Some(face_index))
                    .collect::<Vec<_>>();
                Some(ExactMeshlibFaceExportFailureDiagnostic {
                    face_index,
                    face_edge_id: face_edge.0,
                    face_operand: self.face_operands.get(face_index).copied().flatten(),
                    face_cut_face: self.face_cut_faces.get(face_index).copied().flatten(),
                    face_source_face: self.face_source_faces.get(face_index).copied().flatten(),
                    error,
                    left_ring_edge_ids: left_ring_trace.edges.iter().map(|edge| edge.0).collect(),
                    left_ring_record_next_edge_ids: left_ring_trace
                        .edges
                        .iter()
                        .map(|edge| self.topology.next(*edge).0)
                        .collect(),
                    left_ring_record_prev_edge_ids: left_ring_trace
                        .edges
                        .iter()
                        .map(|edge| self.topology.prev(*edge).0)
                        .collect(),
                    left_ring_next_edge_ids: left_ring_trace
                        .next_edges
                        .iter()
                        .map(|edge| edge.0)
                        .collect(),
                    left_ring_origins: left_ring_trace
                        .edges
                        .iter()
                        .map(|edge| self.topology.origin(*edge))
                        .collect(),
                    left_ring_destinations: left_ring_trace
                        .edges
                        .iter()
                        .map(|edge| self.topology.origin(ExactHalfEdgeTopology::sym(*edge)))
                        .collect(),
                    left_ring_left_faces: left_ring_trace
                        .edges
                        .iter()
                        .map(|edge| self.topology.left(*edge))
                        .collect(),
                    left_ring_right_faces: left_ring_trace
                        .edges
                        .iter()
                        .map(|edge| self.topology.right(*edge))
                        .collect(),
                    same_left_face_edge_ids: same_left_face_edges
                        .iter()
                        .map(|edge| edge.0)
                        .collect(),
                    same_left_face_record_next_edge_ids: same_left_face_edges
                        .iter()
                        .map(|edge| self.topology.next(*edge).0)
                        .collect(),
                    same_left_face_record_prev_edge_ids: same_left_face_edges
                        .iter()
                        .map(|edge| self.topology.prev(*edge).0)
                        .collect(),
                    same_left_face_next_edge_ids: same_left_face_edges
                        .iter()
                        .map(|edge| self.topology.prev(ExactHalfEdgeTopology::sym(*edge)).0)
                        .collect(),
                    same_left_face_origins: same_left_face_edges
                        .iter()
                        .map(|edge| self.topology.origin(*edge))
                        .collect(),
                    same_left_face_destinations: same_left_face_edges
                        .iter()
                        .map(|edge| self.topology.origin(ExactHalfEdgeTopology::sym(*edge)))
                        .collect(),
                    same_left_face_right_faces: same_left_face_edges
                        .iter()
                        .map(|edge| self.topology.right(*edge))
                        .collect(),
                    left_ring_repeated_edge_id: left_ring_trace.repeated_edge.map(|edge| edge.0),
                    left_ring_returned_to_start: left_ring_trace.repeated_edge == Some(face_edge),
                })
            })
            .collect()
    }

    fn trace_left_ring_edges(&self, start: ExactHalfEdgeId) -> LeftRingTrace {
        let limit = self.topology.edge_ids().count().saturating_add(1);
        let mut edges = Vec::new();
        let mut next_edges = Vec::new();
        let mut repeated_edge = None;
        let mut edge = start;
        for _ in 0..limit {
            if edges.contains(&edge) {
                repeated_edge = Some(edge);
                break;
            }
            edges.push(edge);
            let next = self.topology.prev(ExactHalfEdgeTopology::sym(edge));
            next_edges.push(next);
            if next == start {
                repeated_edge = Some(next);
                break;
            }
            edge = next;
        }
        LeftRingTrace {
            edges,
            next_edges,
            repeated_edge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_failure_details_trace_meshlib_left_ring_not_origin_ring() {
        let mut output = OutputFaceTopology::from_faces(&[[0, 1, 2]]).unwrap();
        let face_edge = output.face_edges[0];
        output.topology.set_left_direct(face_edge, None).unwrap();

        let details = output.export_face_failure_details();

        assert_eq!(details.len(), 1);
        assert_eq!(details[0].face_index, 0);
        assert_eq!(
            details[0].error,
            "MeshLib face record edge must have face on left"
        );
        assert_eq!(
            details[0].left_ring_origins,
            vec![Some(0), Some(1), Some(2)]
        );
        assert_eq!(
            details[0].left_ring_destinations,
            vec![Some(1), Some(2), Some(0)]
        );
        assert_eq!(
            details[0].left_ring_left_faces,
            vec![None, Some(0), Some(0)]
        );
        assert_eq!(details[0].left_ring_next_edge_ids.len(), 3);
        assert_eq!(details[0].left_ring_record_next_edge_ids.len(), 3);
        assert_eq!(details[0].left_ring_record_prev_edge_ids.len(), 3);
        assert_eq!(details[0].left_ring_right_faces.len(), 3);
        assert_eq!(details[0].same_left_face_edge_ids, vec![2, 4]);
        assert_eq!(details[0].same_left_face_record_next_edge_ids.len(), 2);
        assert_eq!(details[0].same_left_face_record_prev_edge_ids.len(), 2);
        assert_eq!(details[0].same_left_face_next_edge_ids.len(), 2);
        assert_eq!(details[0].same_left_face_origins, vec![Some(1), Some(2)]);
        assert_eq!(
            details[0].same_left_face_destinations,
            vec![Some(2), Some(0)]
        );
        assert_eq!(details[0].same_left_face_right_faces.len(), 2);
        assert_eq!(
            details[0].left_ring_repeated_edge_id,
            Some(details[0].face_edge_id)
        );
        assert!(details[0].left_ring_returned_to_start);
    }
}
