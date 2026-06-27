use super::{
    reverse_edge, ExactMeshlibCopiedPrevNextEdgeUpdate,
    ExactMeshlibCopiedPrevNextEdgeUpdateDiagnostic, OutputFaceTopology,
};
use crate::spatial::exact_boolean::ExactBooleanOperand;
use crate::spatial::exact_halfedge::{ExactHalfEdgeId, ExactHalfEdgeTopology};
use crate::spatial::exact_splice_apply::copied_edges::{
    ExactMeshlibCopiedSourceEdgeDiagnostic, ExactMeshlibCopiedSourceEdgeLookupDiagnostic,
    ExactMeshlibCopiedSourceEdgeStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactMeshlibCopiedFaceRecordCandidateDiagnostic {
    pub source_edge_id: usize,
    pub source_edge_vertices: Option<[usize; 2]>,
    pub source_edge_left: Option<usize>,
    pub source_edge_right: Option<usize>,
    pub source_next_edge_id: usize,
    pub source_prev_edge_id: usize,
    pub mapped_edge_id: Option<usize>,
    pub face_edge_id: Option<usize>,
    pub face_edge_origin: Option<usize>,
    pub face_edge_destination: Option<usize>,
    pub face_edge_left: Option<usize>,
    pub face_edge_right: Option<usize>,
    pub face_edge_next_edge_id: Option<usize>,
    pub face_edge_prev_edge_id: Option<usize>,
    pub face_edge_sym_next_edge_id: Option<usize>,
    pub face_edge_sym_prev_edge_id: Option<usize>,
    pub face_edge_left_ring_next_edge_id: Option<usize>,
    pub left_ring_valid: bool,
    pub left_ring_error: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactMeshlibCopiedFaceRecordDiagnostic {
    pub output_face: usize,
    pub cut_face: usize,
    pub source_face: Option<usize>,
    pub selected_edge_id: usize,
    pub selected_source_edge_id: usize,
    pub selected_source_edge_vertices: Option<[usize; 2]>,
    pub selected_by_valid_left_ring: bool,
    pub selected_left_ring_valid: bool,
    pub selected_left_ring_error: Option<&'static str>,
    pub candidates: Vec<ExactMeshlibCopiedFaceRecordCandidateDiagnostic>,
}

impl OutputFaceTopology {
    pub(in crate::spatial::exact_splice_apply) fn apply_meshlib_copied_prev_next_edges(
        &mut self,
        updates: Vec<ExactMeshlibCopiedPrevNextEdgeUpdate>,
    ) -> usize {
        self.meshlib_copied_prev_next_edge_update_attempts += updates.len();
        let mut applied = 0;
        for update in updates {
            let previous = update.previous;
            let next = update.next;
            let skipped_reason = self
                .topology
                .validate_meshlib_near_stitch_edge_update(previous, next)
                .err();
            if let Some(skipped_reason) = skipped_reason {
                self.meshlib_copied_prev_next_edge_updates_skipped += 1;
                self.push_meshlib_copied_prev_next_edge_update_detail(
                    update,
                    false,
                    Some(skipped_reason),
                );
                continue;
            }
            if self
                .topology
                .apply_meshlib_near_stitch_edge_update(previous, next)
                .is_ok()
            {
                applied += 1;
                self.push_meshlib_copied_prev_next_edge_update_detail(update, true, None);
            } else {
                self.meshlib_copied_prev_next_edge_updates_skipped += 1;
                self.push_meshlib_copied_prev_next_edge_update_detail(
                    update,
                    false,
                    Some("failed copied prev-next edge update"),
                );
            }
        }
        self.meshlib_copied_prev_next_edge_updates_applied += applied;
        applied
    }

    fn push_meshlib_copied_prev_next_edge_update_detail(
        &mut self,
        update: ExactMeshlibCopiedPrevNextEdgeUpdate,
        applied: bool,
        skipped_reason: Option<&'static str>,
    ) {
        let previous = update.previous;
        let next = update.next;
        self.meshlib_copied_prev_next_edge_update_details.push(
            ExactMeshlibCopiedPrevNextEdgeUpdateDiagnostic {
                source_contour_edge_id: Some(update.source_contour_edge.0),
                target_contour_edge_id: Some(update.target_contour_edge.0),
                walked_source_edge_id: Some(update.walked_source_edge.0),
                update_kind: Some(update.update_kind),
                previous_edge_id: previous.0,
                next_edge_id: next.0,
                previous_origin: self.topology.origin(previous),
                next_origin: self.topology.origin(next),
                previous_left: self.topology.left(previous),
                next_right: self.topology.right(next),
                applied,
                skipped_reason,
            },
        );
    }

    pub(in crate::spatial::exact_splice_apply) fn register_meshlib_copied_edge(
        &mut self,
        operand: ExactBooleanOperand,
        source_edge: [usize; 2],
        edge: [usize; 2],
        edge_id: ExactHalfEdgeId,
    ) {
        self.meshlib_copied_directed_edges
            .entry(edge)
            .or_default()
            .push(edge_id);
        self.meshlib_copied_directed_edges
            .entry(reverse_edge(edge))
            .or_default()
            .push(ExactHalfEdgeTopology::sym(edge_id));
        self.register_meshlib_source_edge(operand, source_edge, edge_id);
    }

    pub(in crate::spatial::exact_splice_apply) fn record_meshlib_copied_source_edge_status(
        &mut self,
        operand: ExactBooleanOperand,
        source_edge: [usize; 2],
        diagnostic: ExactMeshlibCopiedSourceEdgeDiagnostic,
        reverse_diagnostic: ExactMeshlibCopiedSourceEdgeDiagnostic,
    ) {
        self.push_meshlib_copied_source_edge_status(operand, source_edge, diagnostic);
        self.push_meshlib_copied_source_edge_status(
            operand,
            reverse_edge(source_edge),
            reverse_diagnostic,
        );
    }

    fn push_meshlib_copied_source_edge_status(
        &mut self,
        operand: ExactBooleanOperand,
        source_edge: [usize; 2],
        diagnostic: ExactMeshlibCopiedSourceEdgeDiagnostic,
    ) {
        let diagnostics = self
            .meshlib_copied_source_edge_statuses
            .entry((operand, source_edge))
            .or_default();
        if !diagnostics.contains(&diagnostic) {
            diagnostics.push(diagnostic);
        }
    }

    pub(in crate::spatial::exact_splice_apply) fn meshlib_copied_source_edge_lookup(
        &self,
        operand: Option<ExactBooleanOperand>,
        source_edge: Option<[usize; 2]>,
    ) -> Option<ExactMeshlibCopiedSourceEdgeLookupDiagnostic> {
        let operand = operand?;
        let source_edge = source_edge?;
        let Some(statuses) = self
            .meshlib_copied_source_edge_statuses
            .get(&(operand, source_edge))
        else {
            return Some(ExactMeshlibCopiedSourceEdgeLookupDiagnostic {
                status: ExactMeshlibCopiedSourceEdgeStatus::NotPreparedSourceEdge,
                matched_source_edge: None,
                source_halfedge: None,
                source_origin: None,
                source_left: None,
                source_right: None,
                source_left_mapped_face: None,
                source_right_mapped_face: None,
                source_next_halfedge: None,
                source_prev_halfedge: None,
                output_edge_id: None,
                output_origin: None,
                output_left: None,
                output_right: None,
                output_next_edge_id: None,
                output_prev_edge_id: None,
                matching_statuses: 0,
            });
        };
        let selected = statuses
            .iter()
            .find(|diagnostic| diagnostic.status == ExactMeshlibCopiedSourceEdgeStatus::Copied)
            .or_else(|| statuses.first())?;
        let output_edge = selected.output_edge_id.map(ExactHalfEdgeId);
        Some(ExactMeshlibCopiedSourceEdgeLookupDiagnostic {
            status: selected.status,
            matched_source_edge: Some(source_edge),
            source_halfedge: selected.source_halfedge,
            source_origin: selected.source_origin,
            source_left: selected.source_left,
            source_right: selected.source_right,
            source_left_mapped_face: selected.source_left_mapped_face,
            source_right_mapped_face: selected.source_right_mapped_face,
            source_next_halfedge: selected.source_next_halfedge,
            source_prev_halfedge: selected.source_prev_halfedge,
            output_edge_id: selected.output_edge_id,
            output_origin: output_edge.and_then(|edge| self.topology.origin(edge)),
            output_left: output_edge.and_then(|edge| self.topology.left(edge)),
            output_right: output_edge.and_then(|edge| self.topology.right(edge)),
            output_next_edge_id: output_edge.map(|edge| self.topology.next(edge).0),
            output_prev_edge_id: output_edge.map(|edge| self.topology.prev(edge).0),
            matching_statuses: statuses.len(),
        })
    }

    pub(in crate::spatial::exact_splice_apply) fn record_meshlib_copied_face_record_detail(
        &mut self,
        detail: ExactMeshlibCopiedFaceRecordDiagnostic,
    ) {
        self.meshlib_copied_face_record_details.push(detail);
    }
}
