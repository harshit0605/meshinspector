use super::super::super::output_topology::{
    ExactMeshlibCopiedFaceRecordCandidateDiagnostic, ExactMeshlibCopiedFaceRecordDiagnostic,
    OutputFaceTopology,
};
use super::{ExactHalfEdgeId, ExactHalfEdgeTopology, SourcePreparedTopology};
use std::collections::BTreeMap;

struct CopiedFaceRecordDiagnosticInput {
    output_face: usize,
    cut_face: usize,
    source_face: Option<usize>,
    selected_source_edge: ExactHalfEdgeId,
    selected_edge: ExactHalfEdgeId,
    selected_by_valid_left_ring: bool,
    selected_left_ring_error: Option<&'static str>,
    candidates: Vec<ExactMeshlibCopiedFaceRecordCandidateDiagnostic>,
    selected_source_edge_vertices: Option<[usize; 2]>,
}

impl SourcePreparedTopology {
    #[cfg(test)]
    pub(in crate::spatial::exact_splice_apply::copied_edges) fn mapped_face_edge(
        &self,
        _output: &OutputFaceTopology,
        face: usize,
        _output_face: usize,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        flip_orientation: bool,
    ) -> Option<ExactHalfEdgeId> {
        let stored_edges = self.face_edges.get(&face)?;
        let ring_edges = stored_edges
            .first()
            .and_then(|edge| self.topology.left_ring_edges(*edge).ok());
        for edge in ring_edges.as_deref().unwrap_or(stored_edges) {
            let Some(mapped_edge) = self.map_edge_like_meshlib(*edge, edge_map) else {
                continue;
            };
            return Some(if flip_orientation {
                ExactHalfEdgeTopology::sym(mapped_edge)
            } else {
                mapped_edge
            });
        }
        None
    }

    pub(in crate::spatial::exact_splice_apply::copied_edges) fn mapped_face_edge_with_diagnostic(
        &self,
        output: &OutputFaceTopology,
        face: usize,
        output_face: usize,
        edge_map: &BTreeMap<ExactHalfEdgeId, ExactHalfEdgeId>,
        flip_orientation: bool,
    ) -> Option<(ExactHalfEdgeId, ExactMeshlibCopiedFaceRecordDiagnostic)> {
        let stored_edges = self.face_edges.get(&face)?;
        let ring_edges = stored_edges
            .first()
            .and_then(|edge| self.topology.left_ring_edges(*edge).ok());
        let mut candidates = Vec::new();
        let mut first_mapped = None;
        let mut first_valid = None;
        for source_edge in ring_edges.as_deref().unwrap_or(stored_edges) {
            let mapped_edge = self.map_edge_like_meshlib(*source_edge, edge_map);
            let face_edge = mapped_edge.map(|edge| {
                if flip_orientation {
                    ExactHalfEdgeTopology::sym(edge)
                } else {
                    edge
                }
            });
            let validation = face_edge.map(|edge| {
                output
                    .topology
                    .validate_meshlib_face_left_ring(edge, output_face)
            });
            let left_ring_error = validation.and_then(Result::err);
            let left_ring_valid = validation.is_some_and(|result| result.is_ok());
            candidates.push(ExactMeshlibCopiedFaceRecordCandidateDiagnostic {
                source_edge_id: source_edge.0,
                source_edge_vertices: self.source_vertices_for_edge(*source_edge),
                source_edge_left: self.topology.left(*source_edge),
                source_edge_right: self.topology.right(*source_edge),
                source_next_edge_id: self.topology.next(*source_edge).0,
                source_prev_edge_id: self.topology.prev(*source_edge).0,
                mapped_edge_id: mapped_edge.map(|edge| edge.0),
                face_edge_id: face_edge.map(|edge| edge.0),
                face_edge_origin: face_edge.and_then(|edge| output.topology.origin(edge)),
                face_edge_destination: face_edge
                    .and_then(|edge| output.topology.origin(ExactHalfEdgeTopology::sym(edge))),
                face_edge_left: face_edge.and_then(|edge| output.topology.left(edge)),
                face_edge_right: face_edge.and_then(|edge| output.topology.right(edge)),
                face_edge_next_edge_id: face_edge.map(|edge| output.topology.next(edge).0),
                face_edge_prev_edge_id: face_edge.map(|edge| output.topology.prev(edge).0),
                face_edge_sym_next_edge_id: face_edge
                    .map(|edge| output.topology.next(ExactHalfEdgeTopology::sym(edge)).0),
                face_edge_sym_prev_edge_id: face_edge
                    .map(|edge| output.topology.prev(ExactHalfEdgeTopology::sym(edge)).0),
                face_edge_left_ring_next_edge_id: face_edge
                    .map(|edge| output.topology.prev(ExactHalfEdgeTopology::sym(edge)).0),
                left_ring_valid,
                left_ring_error,
            });
            let Some(face_edge) = face_edge else {
                continue;
            };
            if left_ring_valid && first_valid.is_none() {
                first_valid = Some((*source_edge, face_edge, left_ring_error));
            }
            if first_mapped.is_none() {
                first_mapped = Some((*source_edge, face_edge, left_ring_error));
            }
        }
        let (source_edge, face_edge, selected_error, selected_by_valid_left_ring) =
            if let Some((source_edge, face_edge, selected_error)) = first_valid {
                (source_edge, face_edge, selected_error, true)
            } else {
                let (source_edge, face_edge, selected_error) = first_mapped?;
                (source_edge, face_edge, selected_error, false)
            };
        Some((
            face_edge,
            copied_face_record_diagnostic(CopiedFaceRecordDiagnosticInput {
                output_face,
                cut_face: face,
                source_face: self.source_face_for_face(face),
                selected_source_edge: source_edge,
                selected_edge: face_edge,
                selected_by_valid_left_ring,
                selected_left_ring_error: selected_error,
                candidates,
                selected_source_edge_vertices: self.source_vertices_for_edge(source_edge),
            }),
        ))
    }
}

fn copied_face_record_diagnostic(
    input: CopiedFaceRecordDiagnosticInput,
) -> ExactMeshlibCopiedFaceRecordDiagnostic {
    ExactMeshlibCopiedFaceRecordDiagnostic {
        output_face: input.output_face,
        cut_face: input.cut_face,
        source_face: input.source_face,
        selected_edge_id: input.selected_edge.0,
        selected_source_edge_id: input.selected_source_edge.0,
        selected_source_edge_vertices: input.selected_source_edge_vertices,
        selected_by_valid_left_ring: input.selected_by_valid_left_ring,
        selected_left_ring_valid: input.selected_left_ring_error.is_none(),
        selected_left_ring_error: input.selected_left_ring_error,
        candidates: input.candidates,
    }
}
