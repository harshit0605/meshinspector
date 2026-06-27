use super::copied_records::{
    MeshlibCopiedFaceRecordCandidateDiagnostic, MeshlibCopiedFaceRecordDiagnostic,
    MeshlibCopiedPrevNextEdgeUpdateDiagnostic,
};
use super::{near_stitch_failure_details, MeshlibNearStitchFailureDiagnostic};
use crate::spatial::exact_boolean::ExactBooleanOperand;
use crate::spatial::exact_meshlib_near_stitch::ExactMeshlibNearStitchPlan;
use crate::spatial::exact_meshlib_rewrite_apply::{
    ExactMeshlibPreparedBaseRecordRewriteApplyPlan, ExactMeshlibRecordRewriteApplyPlan,
};
use crate::spatial::exact_splice_apply::{
    ExactMeshlibCopiedFaceRecordCandidateDiagnostic as ExactCopiedFaceCandidateDiagnostic,
    ExactMeshlibCopiedFaceRecordDiagnostic as ExactCopiedFaceDiagnostic,
    ExactMeshlibCopiedPrevNextEdgeUpdateDiagnostic as ExactCopiedPrevNextDiagnostic,
    ExactMeshlibPreparedSourceRecordReplayDiagnostic as ExactReplayDiagnostic,
    ExactMeshlibRecordRewriteTargetDiagnostic as ExactRecordRewriteDiagnostic,
};
use crate::{MeshHealth, MeshStats};

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibPreparedBaseRecordRewriteDiagnostics {
    pub prepared_faces: usize,
    pub prepared_vertices: usize,
    pub virtual_vertices: usize,
    pub prepared_face_sources: usize,
    pub applied_commands: usize,
    pub failed_commands: usize,
    pub record_failed_missing_targets: usize,
    pub record_failed_closed_targets: usize,
    pub record_failed_missing_sources: usize,
    pub record_failed_other_commands: usize,
    pub record_rewrite_target_details: Vec<MeshlibRecordRewriteTargetDiagnostic>,
    pub record_rewrite_failed_command_details: Vec<MeshlibRecordRewriteFailedCommandDiagnostic>,
    pub record_rewrite_near_stitch_target_left_closures: usize,
    pub record_rewrite_near_stitch_target_right_closures: usize,
    pub translated_copied_edge_records: usize,
    pub translated_copied_face_records: usize,
    pub mapped_source_record_replays: usize,
    pub mapped_source_record_replays_on_near_stitch_targets: usize,
    pub mapped_source_record_replay_attempts: usize,
    pub mapped_source_record_replay_attempts_on_near_stitch_targets: usize,
    pub skipped_mapped_source_record_replays: usize,
    pub mapped_source_record_replay_details: Vec<MeshlibPreparedSourceRecordReplayDiagnostic>,
    pub copied_prev_next_edge_update_attempts: usize,
    pub copied_prev_next_edge_updates_applied: usize,
    pub copied_prev_next_edge_updates_skipped: usize,
    pub copied_prev_next_edge_update_details: Vec<MeshlibCopiedPrevNextEdgeUpdateDiagnostic>,
    pub copied_face_record_details: Vec<MeshlibCopiedFaceRecordDiagnostic>,
    pub failed_copied_edge_records: usize,
    pub refreshed_face_records: usize,
    pub near_stitch_updates_applied: usize,
    pub near_stitch_updates_failed: usize,
    pub near_stitch_failed_start: usize,
    pub near_stitch_failed_end: usize,
    pub near_stitch_skipped_previous_left_source_edges: usize,
    pub near_stitch_skipped_next_right_source_edges: usize,
    pub near_stitch_missing_previous_edges: usize,
    pub near_stitch_missing_next_edges: usize,
    pub near_stitch_origin_mismatches: usize,
    pub near_stitch_previous_left_faces: usize,
    pub near_stitch_previous_left_copied_source_edges: usize,
    pub near_stitch_next_right_faces: usize,
    pub near_stitch_next_right_copied_source_edges: usize,
    pub near_stitch_failed_other: usize,
    pub near_stitch_failed_details: Vec<MeshlibNearStitchFailureDiagnostic>,
    pub exported_faces: usize,
    pub export_failed_faces: usize,
    pub export_failed_face_indices: Vec<usize>,
    pub export_failed_face_details: Vec<MeshlibFaceExportFailureDiagnostic>,
    pub export_non_triangular_faces: usize,
    pub export_left_ring_not_closed_faces: usize,
    pub export_missing_origin_faces: usize,
    pub export_face_record_left_mismatch_faces: usize,
    pub export_face_left_ring_mismatch_faces: usize,
    pub export_other_failed_faces: usize,
    pub exported_face_operands: Vec<Option<ExactBooleanOperand>>,
    pub exported_face_cut_faces: Vec<Option<usize>>,
    pub exported_face_source_faces: Vec<Option<usize>>,
    pub exported_mesh_stats: Option<MeshStats>,
    pub exported_mesh_health: Option<MeshHealth>,
    pub packed_mesh_stats: Option<MeshStats>,
    pub packed_mesh_health: Option<MeshHealth>,
    pub ready_for_export: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibFaceExportFailureDiagnostic {
    pub face_index: usize,
    pub face_edge_id: usize,
    pub face_operand: Option<&'static str>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibRecordRewriteTargetDiagnostic {
    pub stitch_pair_index: usize,
    pub target_edge_id: usize,
    pub target_was_near_stitch_target: bool,
    pub target_origin_before: Option<usize>,
    pub target_left_before: Option<usize>,
    pub target_right_before: Option<usize>,
    pub target_next_edge_id_before: usize,
    pub target_prev_edge_id_before: usize,
    pub target_origin_after: Option<usize>,
    pub target_left_after: Option<usize>,
    pub target_right_after: Option<usize>,
    pub target_next_edge_id_after: usize,
    pub target_prev_edge_id_after: usize,
    pub record_next_edge_id: usize,
    pub record_left: Option<usize>,
    pub record_sym_prev_edge_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibRecordRewriteFailedCommandDiagnostic {
    pub stitch_pair_index: usize,
    pub error: &'static str,
    pub this_operand: &'static str,
    pub from_operand: &'static str,
    pub output_edge: [usize; 2],
    pub this_contour_edge: [usize; 2],
    pub from_contour_edge: [usize; 2],
    pub this_source_edge_index: usize,
    pub from_source_edge_index: usize,
    pub this_source_edge: [usize; 2],
    pub from_source_edge: [usize; 2],
    pub this_side_synthetic: bool,
    pub from_side_synthetic: bool,
    pub synthetic_sides: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshlibPreparedSourceRecordReplayDiagnostic {
    pub target_edge_id: usize,
    pub target_was_near_stitch_target: bool,
    pub target_origin_before: Option<usize>,
    pub target_left_before: Option<usize>,
    pub target_right_before: Option<usize>,
    pub target_origin_after: Option<usize>,
    pub target_left_after: Option<usize>,
    pub target_right_after: Option<usize>,
    pub record_next_edge_id: usize,
    pub record_left: Option<usize>,
    pub record_sym_prev_edge_id: usize,
    pub applied: bool,
    pub skipped_reason: Option<&'static str>,
}

impl MeshlibPreparedBaseRecordRewriteDiagnostics {
    pub(super) fn from_apply_plan(
        plan: &ExactMeshlibPreparedBaseRecordRewriteApplyPlan,
        near_stitch: &ExactMeshlibNearStitchPlan,
        exported_mesh_stats: Option<MeshStats>,
        exported_mesh_health: Option<MeshHealth>,
        packed_mesh_stats: Option<MeshStats>,
        packed_mesh_health: Option<MeshHealth>,
    ) -> Self {
        let record_rewrite_target_details = plan
            .apply
            .entries
            .iter()
            .filter_map(|entry| entry.target_diagnostic.as_ref())
            .map(record_rewrite_target_detail)
            .collect::<Vec<_>>();
        let record_rewrite_failed_command_details = plan
            .apply
            .entries
            .iter()
            .filter(|entry| entry.error.is_some())
            .map(record_rewrite_failed_command_detail)
            .collect::<Vec<_>>();
        let record_rewrite_near_stitch_target_left_closures = record_rewrite_target_details
            .iter()
            .filter(|detail| {
                detail.target_was_near_stitch_target
                    && detail.target_left_before.is_none()
                    && detail.target_left_after.is_some()
            })
            .count();
        let record_rewrite_near_stitch_target_right_closures = record_rewrite_target_details
            .iter()
            .filter(|detail| {
                detail.target_was_near_stitch_target
                    && detail.target_right_before.is_none()
                    && detail.target_right_after.is_some()
            })
            .count();
        let near_stitch_failed_details = near_stitch_failure_details(&plan.apply);
        let near_stitch_previous_left_copied_source_edges =
            near_stitch_previous_left_copied_source_edges(&near_stitch_failed_details);
        let near_stitch_next_right_copied_source_edges =
            near_stitch_next_right_copied_source_edges(&near_stitch_failed_details);
        Self {
            prepared_faces: plan.prepared_faces,
            prepared_vertices: plan.prepared_vertices,
            virtual_vertices: plan.virtual_vertices,
            prepared_face_sources: plan.prepared_face_sources,
            applied_commands: plan.apply.applied_commands,
            failed_commands: plan.apply.failed_commands,
            record_failed_missing_targets: plan.apply.failed_missing_target_edges,
            record_failed_closed_targets: plan.apply.failed_closed_target_edges,
            record_failed_missing_sources: plan.apply.failed_missing_source_edges,
            record_failed_other_commands: plan.apply.failed_other_commands,
            record_rewrite_target_details,
            record_rewrite_failed_command_details,
            record_rewrite_near_stitch_target_left_closures,
            record_rewrite_near_stitch_target_right_closures,
            translated_copied_edge_records: plan.apply.translated_copied_edge_records,
            translated_copied_face_records: plan.apply.translated_copied_face_records,
            mapped_source_record_replays: plan.apply.mapped_source_record_replays,
            mapped_source_record_replays_on_near_stitch_targets: plan
                .apply
                .mapped_source_record_replays_on_near_stitch_targets,
            mapped_source_record_replay_attempts: plan.apply.mapped_source_record_replay_attempts,
            mapped_source_record_replay_attempts_on_near_stitch_targets: plan
                .apply
                .mapped_source_record_replay_attempts_on_near_stitch_targets,
            skipped_mapped_source_record_replays: plan.apply.skipped_mapped_source_record_replays,
            mapped_source_record_replay_details: plan
                .apply
                .mapped_source_record_replay_details
                .iter()
                .map(mapped_source_record_replay_detail)
                .collect(),
            copied_prev_next_edge_update_attempts: plan.apply.copied_prev_next_edge_update_attempts,
            copied_prev_next_edge_updates_applied: plan.apply.copied_prev_next_edge_updates_applied,
            copied_prev_next_edge_updates_skipped: plan.apply.copied_prev_next_edge_updates_skipped,
            copied_prev_next_edge_update_details: plan
                .apply
                .copied_prev_next_edge_update_details
                .iter()
                .map(copied_prev_next_edge_update_detail)
                .collect(),
            copied_face_record_details: plan
                .apply
                .copied_face_record_details
                .iter()
                .map(copied_face_record_detail)
                .collect(),
            failed_copied_edge_records: plan.apply.failed_copied_edge_records,
            refreshed_face_records: plan.apply.refreshed_face_records,
            near_stitch_updates_applied: plan.apply.applied_near_stitch_updates,
            near_stitch_updates_failed: plan.apply.failed_near_stitch_updates,
            near_stitch_failed_start: plan.apply.failed_near_stitch_start_updates,
            near_stitch_failed_end: plan.apply.failed_near_stitch_end_updates,
            near_stitch_skipped_previous_left_source_edges: near_stitch
                .skipped_previous_left_source_edges,
            near_stitch_skipped_next_right_source_edges: near_stitch
                .skipped_next_right_source_edges,
            near_stitch_missing_previous_edges: plan
                .apply
                .failed_missing_near_stitch_previous_edges,
            near_stitch_missing_next_edges: plan.apply.failed_missing_near_stitch_next_edges,
            near_stitch_origin_mismatches: plan.apply.failed_near_stitch_origin_mismatches,
            near_stitch_previous_left_faces: plan.apply.failed_near_stitch_previous_left_faces,
            near_stitch_previous_left_copied_source_edges,
            near_stitch_next_right_faces: plan.apply.failed_near_stitch_next_right_faces,
            near_stitch_next_right_copied_source_edges,
            near_stitch_failed_other: plan.apply.failed_other_near_stitch_updates,
            near_stitch_failed_details,
            exported_faces: plan.apply.exported_faces,
            export_failed_faces: plan.apply.export_failed_faces,
            export_failed_face_indices: plan.apply.export_failed_face_indices.clone(),
            export_failed_face_details: face_export_failure_details(&plan.apply),
            export_non_triangular_faces: plan.apply.export_non_triangular_faces,
            export_left_ring_not_closed_faces: plan.apply.export_left_ring_not_closed_faces,
            export_missing_origin_faces: plan.apply.export_missing_origin_faces,
            export_face_record_left_mismatch_faces: plan
                .apply
                .export_face_record_left_mismatch_faces,
            export_face_left_ring_mismatch_faces: plan.apply.export_face_left_ring_mismatch_faces,
            export_other_failed_faces: plan.apply.export_other_failed_faces,
            exported_face_operands: plan.apply.exported_face_operands.clone(),
            exported_face_cut_faces: plan.apply.exported_face_cut_faces.clone(),
            exported_face_source_faces: plan.apply.exported_face_source_faces.clone(),
            exported_mesh_stats,
            exported_mesh_health,
            packed_mesh_stats,
            packed_mesh_health,
            ready_for_export: plan.apply.ready_for_export,
        }
    }
}

fn near_stitch_previous_left_copied_source_edges(
    details: &[MeshlibNearStitchFailureDiagnostic],
) -> usize {
    details
        .iter()
        .filter(|detail| detail.error == "previous near stitch edge must not have a left face")
        .filter(|detail| {
            detail
                .candidate_diagnostics
                .as_ref()
                .and_then(|diagnostics| diagnostics.previous_source_lookup.as_ref())
                .and_then(|lookup| lookup.copied_source_edge.as_ref())
                .is_some_and(|copied| copied.output_left.is_some())
        })
        .count()
}

fn near_stitch_next_right_copied_source_edges(
    details: &[MeshlibNearStitchFailureDiagnostic],
) -> usize {
    details
        .iter()
        .filter(|detail| detail.error == "next near stitch edge must not have a right face")
        .filter(|detail| {
            detail
                .candidate_diagnostics
                .as_ref()
                .and_then(|diagnostics| diagnostics.next_source_lookup.as_ref())
                .and_then(|lookup| lookup.copied_source_edge.as_ref())
                .is_some_and(|copied| copied.output_right.is_some())
        })
        .count()
}

fn record_rewrite_target_detail(
    detail: &ExactRecordRewriteDiagnostic,
) -> MeshlibRecordRewriteTargetDiagnostic {
    MeshlibRecordRewriteTargetDiagnostic {
        stitch_pair_index: detail.stitch_pair_index,
        target_edge_id: detail.target_edge_id,
        target_was_near_stitch_target: detail.target_was_near_stitch_target,
        target_origin_before: detail.target_origin_before,
        target_left_before: detail.target_left_before,
        target_right_before: detail.target_right_before,
        target_next_edge_id_before: detail.target_next_edge_id_before,
        target_prev_edge_id_before: detail.target_prev_edge_id_before,
        target_origin_after: detail.target_origin_after,
        target_left_after: detail.target_left_after,
        target_right_after: detail.target_right_after,
        target_next_edge_id_after: detail.target_next_edge_id_after,
        target_prev_edge_id_after: detail.target_prev_edge_id_after,
        record_next_edge_id: detail.record_next_edge_id,
        record_left: detail.record_left,
        record_sym_prev_edge_id: detail.record_sym_prev_edge_id,
    }
}

fn record_rewrite_failed_command_detail(
    entry: &crate::spatial::exact_meshlib_rewrite_apply::ExactMeshlibRecordRewriteApplyEntry,
) -> MeshlibRecordRewriteFailedCommandDiagnostic {
    MeshlibRecordRewriteFailedCommandDiagnostic {
        stitch_pair_index: entry.command.stitch_pair_index,
        error: entry.error.unwrap_or("unknown MeshLib rewrite failure"),
        this_operand: operand_label(entry.command.this_operand),
        from_operand: operand_label(entry.command.from_operand),
        output_edge: entry.command.output_edge,
        this_contour_edge: entry.command.this_contour_edge,
        from_contour_edge: entry.command.from_contour_edge,
        this_source_edge_index: entry.command.this_source_edge_index,
        from_source_edge_index: entry.command.from_source_edge_index,
        this_source_edge: entry.command.this_source_edge,
        from_source_edge: entry.command.from_source_edge,
        this_side_synthetic: entry.command.this_side_synthetic,
        from_side_synthetic: entry.command.from_side_synthetic,
        synthetic_sides: entry.command.synthetic_sides,
    }
}

fn mapped_source_record_replay_detail(
    detail: &ExactReplayDiagnostic,
) -> MeshlibPreparedSourceRecordReplayDiagnostic {
    MeshlibPreparedSourceRecordReplayDiagnostic {
        target_edge_id: detail.target_edge_id,
        target_was_near_stitch_target: detail.target_was_near_stitch_target,
        target_origin_before: detail.target_origin_before,
        target_left_before: detail.target_left_before,
        target_right_before: detail.target_right_before,
        target_origin_after: detail.target_origin_after,
        target_left_after: detail.target_left_after,
        target_right_after: detail.target_right_after,
        record_next_edge_id: detail.record_next_edge_id,
        record_left: detail.record_left,
        record_sym_prev_edge_id: detail.record_sym_prev_edge_id,
        applied: detail.applied,
        skipped_reason: detail.skipped_reason,
    }
}

fn copied_prev_next_edge_update_detail(
    detail: &ExactCopiedPrevNextDiagnostic,
) -> MeshlibCopiedPrevNextEdgeUpdateDiagnostic {
    MeshlibCopiedPrevNextEdgeUpdateDiagnostic {
        source_contour_edge_id: detail.source_contour_edge_id,
        target_contour_edge_id: detail.target_contour_edge_id,
        walked_source_edge_id: detail.walked_source_edge_id,
        update_kind: detail.update_kind,
        previous_edge_id: detail.previous_edge_id,
        next_edge_id: detail.next_edge_id,
        previous_origin: detail.previous_origin,
        next_origin: detail.next_origin,
        previous_left: detail.previous_left,
        next_right: detail.next_right,
        applied: detail.applied,
        skipped_reason: detail.skipped_reason,
    }
}

fn copied_face_record_detail(
    detail: &ExactCopiedFaceDiagnostic,
) -> MeshlibCopiedFaceRecordDiagnostic {
    MeshlibCopiedFaceRecordDiagnostic {
        output_face: detail.output_face,
        cut_face: detail.cut_face,
        source_face: detail.source_face,
        selected_edge_id: detail.selected_edge_id,
        selected_source_edge_id: detail.selected_source_edge_id,
        selected_source_edge_vertices: detail.selected_source_edge_vertices,
        selected_by_valid_left_ring: detail.selected_by_valid_left_ring,
        selected_left_ring_valid: detail.selected_left_ring_valid,
        selected_left_ring_error: detail.selected_left_ring_error,
        candidates: detail
            .candidates
            .iter()
            .map(copied_face_record_candidate_detail)
            .collect(),
    }
}

fn copied_face_record_candidate_detail(
    detail: &ExactCopiedFaceCandidateDiagnostic,
) -> MeshlibCopiedFaceRecordCandidateDiagnostic {
    MeshlibCopiedFaceRecordCandidateDiagnostic {
        source_edge_id: detail.source_edge_id,
        source_edge_vertices: detail.source_edge_vertices,
        source_edge_left: detail.source_edge_left,
        source_edge_right: detail.source_edge_right,
        source_next_edge_id: detail.source_next_edge_id,
        source_prev_edge_id: detail.source_prev_edge_id,
        mapped_edge_id: detail.mapped_edge_id,
        face_edge_id: detail.face_edge_id,
        face_edge_origin: detail.face_edge_origin,
        face_edge_destination: detail.face_edge_destination,
        face_edge_left: detail.face_edge_left,
        face_edge_right: detail.face_edge_right,
        face_edge_next_edge_id: detail.face_edge_next_edge_id,
        face_edge_prev_edge_id: detail.face_edge_prev_edge_id,
        face_edge_sym_next_edge_id: detail.face_edge_sym_next_edge_id,
        face_edge_sym_prev_edge_id: detail.face_edge_sym_prev_edge_id,
        face_edge_left_ring_next_edge_id: detail.face_edge_left_ring_next_edge_id,
        left_ring_valid: detail.left_ring_valid,
        left_ring_error: detail.left_ring_error,
    }
}

fn face_export_failure_details(
    plan: &ExactMeshlibRecordRewriteApplyPlan,
) -> Vec<MeshlibFaceExportFailureDiagnostic> {
    plan.export_failed_face_details
        .iter()
        .map(|detail| MeshlibFaceExportFailureDiagnostic {
            face_index: detail.face_index,
            face_edge_id: detail.face_edge_id,
            face_operand: detail.face_operand.map(operand_label),
            face_cut_face: detail.face_cut_face,
            face_source_face: detail.face_source_face,
            error: detail.error,
            left_ring_edge_ids: detail.left_ring_edge_ids.clone(),
            left_ring_record_next_edge_ids: detail.left_ring_record_next_edge_ids.clone(),
            left_ring_record_prev_edge_ids: detail.left_ring_record_prev_edge_ids.clone(),
            left_ring_next_edge_ids: detail.left_ring_next_edge_ids.clone(),
            left_ring_origins: detail.left_ring_origins.clone(),
            left_ring_destinations: detail.left_ring_destinations.clone(),
            left_ring_left_faces: detail.left_ring_left_faces.clone(),
            left_ring_right_faces: detail.left_ring_right_faces.clone(),
            same_left_face_edge_ids: detail.same_left_face_edge_ids.clone(),
            same_left_face_record_next_edge_ids: detail.same_left_face_record_next_edge_ids.clone(),
            same_left_face_record_prev_edge_ids: detail.same_left_face_record_prev_edge_ids.clone(),
            same_left_face_next_edge_ids: detail.same_left_face_next_edge_ids.clone(),
            same_left_face_origins: detail.same_left_face_origins.clone(),
            same_left_face_destinations: detail.same_left_face_destinations.clone(),
            same_left_face_right_faces: detail.same_left_face_right_faces.clone(),
            left_ring_repeated_edge_id: detail.left_ring_repeated_edge_id,
            left_ring_returned_to_start: detail.left_ring_returned_to_start,
        })
        .collect()
}

fn operand_label(operand: ExactBooleanOperand) -> &'static str {
    match operand {
        ExactBooleanOperand::First => "first",
        ExactBooleanOperand::Second => "second",
    }
}
