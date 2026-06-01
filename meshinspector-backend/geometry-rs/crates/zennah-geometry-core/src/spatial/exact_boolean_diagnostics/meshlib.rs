use super::super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation,
    ExactBooleanOutputFaceSource,
};
use super::super::exact_boolean_topology::{
    meshlib_topology_rewrite_plan, ExactMeshlibTopologyRewritePlan,
};
use super::super::exact_cut_apply::ExactCutMeshResult;
use super::super::exact_meshlib_near_stitch::{
    exact_meshlib_near_stitch_plan_with_prepared_parts, exact_meshlib_near_stitch_plan_with_source,
    ExactMeshlibNearStitchPlan,
};
use super::super::exact_meshlib_rewrite_apply::{
    exact_meshlib_prepared_base_record_rewrite_apply_plan,
    exact_meshlib_record_rewrite_apply_plan_with_copied_edges,
    ExactMeshlibPreparedBaseRecordRewriteApplyPlan, ExactMeshlibRecordRewriteApplyPlan,
};
use super::super::exact_splice_apply::{
    output_topology_from_prepared_base, ExactMeshlibPreparedBaseTopologyInput,
    ExactMeshlibPreparedSourceRecordReplayDiagnostic as ExactReplayDiagnostic,
    ExactMeshlibRecordRewriteTargetDiagnostic as ExactRecordRewriteDiagnostic,
};
use super::copied_edges::{
    exact_meshlib_copied_edge_plan, exact_meshlib_copied_edge_translation_input,
    exact_meshlib_near_stitch_source_input, ExactMeshlibCopiedEdgePlan,
};
use super::export::{mesh_export_health, mesh_export_stats, packed_mesh_export};
use crate::{GeometryError, MeshHealth, MeshStats};
mod near_stitch;
pub(super) use near_stitch::near_stitch_failure_details;
pub use near_stitch::{
    MeshlibNearStitchFailureDiagnostic, MeshlibNearStitchLinkedEdgeDiagnostic,
    MeshlibNearStitchRingDiagnostic, MeshlibNearStitchSourceLookupDiagnostic,
    MeshlibNearStitchTargetSnapshotDiagnostic,
};

pub(super) struct MeshlibRewriteDiagnosticsInput<'a> {
    pub first_cut: &'a ExactCutMeshResult,
    pub second_cut: &'a ExactCutMeshResult,
    pub assembly: &'a ExactBooleanAssemblyResult,
    pub operation: ExactBooleanOperation,
    pub epsilon: f64,
}

pub(super) struct MeshlibRewriteDiagnostics {
    pub topology_rewrite: ExactMeshlibTopologyRewritePlan,
    pub copied_edges: ExactMeshlibCopiedEdgePlan,
    pub near_stitch: ExactMeshlibNearStitchPlan,
    pub record_rewrite_apply: ExactMeshlibRecordRewriteApplyPlan,
    pub prepared_base_record_rewrite: MeshlibPreparedBaseRecordRewriteDiagnostics,
    pub record_rewrite_exported_mesh_stats: Option<MeshStats>,
    pub record_rewrite_exported_mesh_health: Option<MeshHealth>,
    pub record_rewrite_packed_mesh_stats: Option<MeshStats>,
    pub record_rewrite_packed_mesh_health: Option<MeshHealth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub ready_for_export: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibFaceExportFailureDiagnostic {
    pub face_index: usize,
    pub face_edge_id: usize,
    pub face_operand: Option<&'static str>,
    pub error: &'static str,
    pub left_ring_edge_ids: Vec<usize>,
    pub left_ring_record_next_edge_ids: Vec<usize>,
    pub left_ring_record_prev_edge_ids: Vec<usize>,
    pub left_ring_next_edge_ids: Vec<usize>,
    pub left_ring_origins: Vec<Option<usize>>,
    pub left_ring_left_faces: Vec<Option<usize>>,
    pub left_ring_right_faces: Vec<Option<usize>>,
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
    fn from_apply_plan(
        plan: &ExactMeshlibPreparedBaseRecordRewriteApplyPlan,
        near_stitch: &ExactMeshlibNearStitchPlan,
    ) -> Self {
        let record_rewrite_target_details = plan
            .apply
            .entries
            .iter()
            .filter_map(|entry| entry.target_diagnostic.as_ref())
            .map(record_rewrite_target_detail)
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

fn face_export_failure_details(
    plan: &ExactMeshlibRecordRewriteApplyPlan,
) -> Vec<MeshlibFaceExportFailureDiagnostic> {
    plan.export_failed_face_details
        .iter()
        .map(|detail| MeshlibFaceExportFailureDiagnostic {
            face_index: detail.face_index,
            face_edge_id: detail.face_edge_id,
            face_operand: detail.face_operand.map(operand_label),
            error: detail.error,
            left_ring_edge_ids: detail.left_ring_edge_ids.clone(),
            left_ring_record_next_edge_ids: detail.left_ring_record_next_edge_ids.clone(),
            left_ring_record_prev_edge_ids: detail.left_ring_record_prev_edge_ids.clone(),
            left_ring_next_edge_ids: detail.left_ring_next_edge_ids.clone(),
            left_ring_origins: detail.left_ring_origins.clone(),
            left_ring_left_faces: detail.left_ring_left_faces.clone(),
            left_ring_right_faces: detail.left_ring_right_faces.clone(),
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

pub(super) fn meshlib_rewrite_diagnostics(
    input: MeshlibRewriteDiagnosticsInput<'_>,
) -> Result<MeshlibRewriteDiagnostics, GeometryError> {
    let topology_rewrite = meshlib_topology_rewrite_plan(input.assembly, input.operation);
    let copied_edges = exact_meshlib_copied_edge_plan(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    let near_stitch = exact_meshlib_near_stitch_plan_with_source(
        input.assembly,
        topology_rewrite.base_operand,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
        exact_meshlib_near_stitch_source_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.incoming_operand,
            &topology_rewrite.record_rewrite_command_edges,
        ),
    );
    let record_rewrite_apply = exact_meshlib_record_rewrite_apply_plan_with_copied_edges(
        &input.assembly.faces,
        &input.assembly.face_sources,
        &topology_rewrite.record_rewrite_command_edges,
        &near_stitch.commands,
        exact_meshlib_copied_edge_translation_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.incoming_operand,
            &topology_rewrite.record_rewrite_command_edges,
        ),
    );
    let empty_face_sources: &[ExactBooleanOutputFaceSource] = &[];
    let prepared_base_vertex_count = prepared_base_vertex_count(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.base_operand,
    );
    let mut prepared_base_copied_edges = exact_meshlib_copied_edge_translation_input(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    prepared_base_copied_edges.face_sources = empty_face_sources;
    prepared_base_copied_edges.append_prepared_faces = true;
    prepared_base_copied_edges.first_virtual_vertex = prepared_base_vertex_count;
    let mut prepared_base_incoming_source = exact_meshlib_near_stitch_source_input(
        input.first_cut,
        input.second_cut,
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
    );
    prepared_base_incoming_source.first_virtual_vertex = prepared_base_vertex_count;
    let prepared_base_near_stitch = exact_meshlib_near_stitch_plan_with_prepared_parts(
        input.assembly,
        topology_rewrite.incoming_operand,
        &topology_rewrite.record_rewrite_command_edges,
        exact_meshlib_near_stitch_source_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.base_operand,
            &topology_rewrite.record_rewrite_command_edges,
        ),
        prepared_base_incoming_source,
    );
    let prepared_base_record_rewrite_apply = exact_meshlib_prepared_base_record_rewrite_apply_plan(
        prepared_base_topology_input(
            input.first_cut,
            input.second_cut,
            input.assembly,
            topology_rewrite.base_operand,
        ),
        &topology_rewrite.record_rewrite_command_edges,
        &prepared_base_near_stitch.commands,
        Some(prepared_base_copied_edges),
    );
    let prepared_base_record_rewrite = MeshlibPreparedBaseRecordRewriteDiagnostics::from_apply_plan(
        &prepared_base_record_rewrite_apply,
        &prepared_base_near_stitch,
    );
    let record_rewrite_exported_mesh_stats = mesh_export_stats(
        &input.assembly.vertices,
        &record_rewrite_apply.exported_face_indices,
        record_rewrite_apply.export_failed_faces,
    )?;
    let record_rewrite_exported_mesh_health = mesh_export_health(
        &input.assembly.vertices,
        &record_rewrite_apply.exported_face_indices,
        record_rewrite_apply.export_failed_faces,
        input.epsilon,
        super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET,
    )?;
    let packed_export = packed_mesh_export(
        &input.assembly.vertices,
        &record_rewrite_apply.exported_face_indices,
        record_rewrite_apply.export_failed_faces,
    )?;
    let record_rewrite_packed_mesh_stats = if let Some(export) = &packed_export {
        crate::mesh::mesh_stats(&export.vertices, &export.faces).map(Some)?
    } else {
        None
    };
    let record_rewrite_packed_mesh_health = if let Some(export) = &packed_export {
        crate::mesh::mesh_health(
            &export.vertices,
            &export.faces,
            true,
            Some(super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET),
            input.epsilon,
        )
        .map(Some)?
    } else {
        None
    };

    Ok(MeshlibRewriteDiagnostics {
        topology_rewrite,
        copied_edges,
        near_stitch,
        record_rewrite_apply,
        prepared_base_record_rewrite,
        record_rewrite_exported_mesh_stats,
        record_rewrite_exported_mesh_health,
        record_rewrite_packed_mesh_stats,
        record_rewrite_packed_mesh_health,
    })
}

fn prepared_base_vertex_count(
    first_cut: &ExactCutMeshResult,
    second_cut: &ExactCutMeshResult,
    assembly: &ExactBooleanAssemblyResult,
    base_operand: ExactBooleanOperand,
) -> usize {
    output_topology_from_prepared_base(prepared_base_topology_input(
        first_cut,
        second_cut,
        assembly,
        base_operand,
    ))
    .map(|prepared| prepared.vertices.len())
    .unwrap_or(assembly.vertices.len())
}

fn prepared_base_topology_input<'a>(
    first_cut: &'a ExactCutMeshResult,
    second_cut: &'a ExactCutMeshResult,
    assembly: &'a ExactBooleanAssemblyResult,
    base_operand: ExactBooleanOperand,
) -> ExactMeshlibPreparedBaseTopologyInput<'a> {
    let (cut_mesh, prepared_faces, vertex_map, flip_orientation) = match base_operand {
        ExactBooleanOperand::First => (
            first_cut,
            assembly.prepare_first_faces.as_slice(),
            assembly.first_output_vertex_for_cut_vertex.as_slice(),
            assembly.flipped_first,
        ),
        ExactBooleanOperand::Second => (
            second_cut,
            assembly.prepare_second_faces.as_slice(),
            assembly.second_output_vertex_for_cut_vertex.as_slice(),
            assembly.flipped_second,
        ),
    };
    ExactMeshlibPreparedBaseTopologyInput {
        cut_mesh,
        prepared_faces,
        vertex_map,
        output_vertices: &assembly.vertices,
        operand: base_operand,
        first_virtual_vertex: assembly.vertices.len(),
        flip_orientation,
    }
}
