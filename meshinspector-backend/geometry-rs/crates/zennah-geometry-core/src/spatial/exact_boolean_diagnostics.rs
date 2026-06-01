use super::exact_boolean::{ExactBooleanAssemblyResult, ExactBooleanOperation};
use super::exact_boolean_candidate::PairedCoplanarCandidateDiagnostics;
use super::exact_coplanar::coplanar_overlap_contours;
use super::exact_cut_pair::ExactCoplanarContourCutTrial;
use super::exact_fill_apply::ExactCutHoleFillResult;
use super::exact_splice::ExactTopologySplicePlan;
use super::exact_splice_apply::ExactTopologySpliceApplyPlan;
use super::exact_stitch::ExactStitchPlan;
use crate::mesh::{mesh_health, mesh_stats};
use crate::{GeometryError, MeshHealth, MeshStats};
mod copied_edges;
mod export;
use export::{mesh_export_health, mesh_export_stats};
mod meshlib;
use meshlib::{meshlib_rewrite_diagnostics, MeshlibRewriteDiagnosticsInput};
pub use meshlib::{
    MeshlibFaceExportFailureDiagnostic, MeshlibNearStitchFailureDiagnostic,
    MeshlibNearStitchLinkedEdgeDiagnostic, MeshlibNearStitchRingDiagnostic,
    MeshlibNearStitchSourceLookupDiagnostic, MeshlibNearStitchTargetSnapshotDiagnostic,
    MeshlibPreparedBaseRecordRewriteDiagnostics, MeshlibPreparedSourceRecordReplayDiagnostic,
};
mod result_cut;
use result_cut::cut_path_length_mismatches;
pub(super) use result_cut::meshlib_result_cut_path_summary;
#[cfg(test)]
pub(super) use result_cut::MeshlibResultCutPathSummary;
mod source;
use source::{duplicate_face_counts, face_source_summary, raw_face_selection_summary};
mod topology;
use topology::{requires_topology_splice, trial_usize, vertices_have_mixed_inside_state};
const BOOLEAN_DIAGNOSTIC_RAY_DIRECTION: [f64; 3] = [1.0, 0.371, 0.219];
const EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET: usize = 20_000;

#[derive(Debug, Clone, PartialEq)]
pub struct ExactBooleanPipelineDiagnostics {
    pub first_cut_edges: usize,
    pub second_cut_edges: usize,
    pub first_skipped_source_faces: Vec<usize>,
    pub second_skipped_source_faces: Vec<usize>,
    pub first_fill_plans: usize,
    pub second_fill_plans: usize,
    pub stitch_compatible: bool,
    pub stitch_unmatched_first_edges: usize,
    pub stitch_unmatched_second_edges: usize,
    pub stitch_cut_path_length_mismatches: usize,
    pub requires_topology_splice: bool,
    pub first_vertices_mixed_against_second: bool,
    pub second_vertices_mixed_against_first: bool,
    pub possible_missing_cut_intersections: bool,
    pub first_prepare_part_dividable: bool,
    pub second_prepare_part_dividable: bool,
    pub first_cut_path_side_components: [usize; 2],
    pub second_cut_path_side_components: [usize; 2],
    pub first_cut_path_overlap_components: usize,
    pub second_cut_path_overlap_components: usize,
    pub coplanar_overlap_pairs: usize,
    pub coplanar_overlap_region_edges: usize,
    pub coplanar_overlap_area: f64,
    pub coplanar_overlap_contours: usize,
    pub coplanar_overlap_contour_edges: usize,
    pub coplanar_cut_trial_contours: usize,
    pub coplanar_cut_trial_contour_edges: usize,
    pub coplanar_cut_trial_first_cut_edges: usize,
    pub coplanar_cut_trial_second_cut_edges: usize,
    pub paired_coplanar_cut_trial_contours: usize,
    pub paired_coplanar_cut_trial_contour_edges: usize,
    pub paired_coplanar_cut_trial_first_cut_edges: usize,
    pub paired_coplanar_cut_trial_second_cut_edges: usize,
    pub paired_coplanar_stitch_cut_path_length_mismatches: usize,
    pub paired_coplanar_stitch_unmatched_first_edges: usize,
    pub paired_coplanar_stitch_unmatched_second_edges: usize,
    pub paired_coplanar_duplicate_first_path_edges: usize,
    pub paired_coplanar_duplicate_second_path_edges: usize,
    pub paired_coplanar_candidate_stitch_compatible: bool,
    pub paired_coplanar_candidate_first_prepare_part_dividable: bool,
    pub paired_coplanar_candidate_second_prepare_part_dividable: bool,
    pub paired_coplanar_candidate_first_cut_path_side_components: [usize; 2],
    pub paired_coplanar_candidate_second_cut_path_side_components: [usize; 2],
    pub paired_coplanar_candidate_first_cut_path_overlap_components: usize,
    pub paired_coplanar_candidate_second_cut_path_overlap_components: usize,
    pub paired_coplanar_candidate_result_cut_paths_complete: bool,
    pub paired_coplanar_candidate_output_faces: usize,
    pub paired_coplanar_candidate_output_area: f64,
    pub paired_coplanar_candidate_output_volume: f64,
    pub paired_coplanar_candidate_self_intersections: Option<usize>,
    pub paired_coplanar_candidate_self_intersections_available: bool,
    pub paired_coplanar_candidate_active_volume_delta: f64,
    pub paired_coplanar_candidate_preserves_active_volume: bool,
    pub paired_coplanar_candidate_boundary_edges: usize,
    pub paired_coplanar_candidate_nonmanifold_edges: usize,
    pub paired_coplanar_candidate_duplicate_output_faces: usize,
    pub coplanar_cut_trial_first_skipped_faces: Vec<usize>,
    pub coplanar_cut_trial_second_skipped_faces: Vec<usize>,
    pub coplanar_cut_trial_accepted: bool,
    pub stitched_output_edges: usize,
    pub stitched_output_edges_with_two_faces: usize,
    pub stitched_output_edges_needing_splice: usize,
    pub topology_splice_ready: bool,
    pub topology_splice_missing_edges: usize,
    pub topology_splice_non_manifold_edges: usize,
    pub topology_splice_apply_ready: bool,
    pub topology_splice_verified_boundary_edges: usize,
    pub topology_splice_blocked_edges: usize,
    pub topology_splice_failed_edges: usize,
    pub topology_splice_synthetic_side_edges: usize,
    pub topology_splice_materialized_boundary_edges: usize,
    pub topology_splice_materialization_failed_edges: usize,
    pub topology_splice_exported_faces: usize,
    pub topology_splice_export_failed_faces: usize,
    pub topology_splice_edges_before_materialization: usize,
    pub topology_splice_edges_after_materialization: usize,
    pub topology_splice_deleted_synthetic_edges: usize,
    pub topology_splice_export_changed_faces: bool,
    pub topology_splice_duplicated_output_edges: usize,
    pub topology_splice_duplicate_output_face_groups: usize,
    pub topology_splice_duplicate_output_faces: usize,
    pub meshlib_topology_base_faces: usize,
    pub meshlib_topology_incoming_faces: usize,
    pub meshlib_topology_selected_first_faces: usize,
    pub meshlib_topology_selected_second_faces: usize,
    pub meshlib_topology_raw_selected_faces: [usize; 2],
    pub meshlib_topology_same_oriented_overlap_faces: [usize; 2],
    pub meshlib_topology_boundary_misses: [[usize; 2]; 2],
    pub meshlib_topology_coplanar_selection_delta_faces: [i64; 2],
    pub meshlib_topology_first_source_face_groups: usize,
    pub meshlib_topology_second_source_face_groups: usize,
    pub meshlib_topology_duplicate_first_source_faces: usize,
    pub meshlib_topology_duplicate_second_source_faces: usize,
    pub meshlib_topology_mapped_contour_edges: usize,
    pub meshlib_topology_missing_base_edges: usize,
    pub meshlib_topology_missing_incoming_edges: usize,
    pub meshlib_topology_direction_mismatches: usize,
    pub meshlib_topology_mapped_stitch_contour_edges: usize,
    pub meshlib_topology_missing_stitch_contour_edges: usize,
    pub meshlib_topology_synthetic_stitch_contour_edges: usize,
    pub meshlib_topology_stitch_direction_mismatches: usize,
    pub meshlib_topology_stitch_metadata_ready: bool,
    pub meshlib_topology_materialized_stitch_contour_edges: usize,
    pub meshlib_topology_unmaterialized_stitch_contour_edges: usize,
    pub meshlib_topology_materialized_synthetic_stitch_sides: usize,
    pub meshlib_topology_stitch_materialization_direction_mismatches: usize,
    pub meshlib_topology_stitch_materialization_ready: bool,
    pub meshlib_topology_record_rewrite_commands: usize,
    pub meshlib_topology_record_rewrite_blocked_edges: usize,
    pub meshlib_topology_record_rewrite_synthetic_sides: usize,
    pub meshlib_topology_record_rewrite_direction_mismatches: usize,
    pub meshlib_topology_record_rewrite_ready: bool,
    pub meshlib_topology_copied_edge_prepared_faces: usize,
    pub meshlib_topology_copied_edge_prepared_vertices: usize,
    pub meshlib_topology_virtual_copied_vertices: usize,
    pub meshlib_topology_copied_edge_prepared_edges: usize,
    pub meshlib_topology_copied_edge_mapped_edges: usize,
    pub meshlib_topology_copied_edges: usize,
    pub meshlib_topology_copied_edges_mapped_to_existing_output: usize,
    pub meshlib_topology_copied_edges_mapped_to_output: usize,
    pub meshlib_topology_copied_edges_missing_output_vertices: usize,
    pub meshlib_topology_copied_edge_translation_ready: bool,
    pub meshlib_topology_open_stitch_paths: usize,
    pub meshlib_topology_open_stitch_near_edge_updates: usize,
    pub meshlib_topology_open_stitch_near_edge_blocked_updates: usize,
    pub meshlib_topology_open_stitch_near_edge_ready: bool,
    pub meshlib_topology_near_stitch_update_commands: usize,
    pub meshlib_topology_near_stitch_updates_applied: usize,
    pub meshlib_topology_near_stitch_updates_failed: usize,
    pub meshlib_topology_near_stitch_updates_failed_start: usize,
    pub meshlib_topology_near_stitch_updates_failed_end: usize,
    pub meshlib_topology_near_stitch_updates_missing_previous_edges: usize,
    pub meshlib_topology_near_stitch_updates_missing_next_edges: usize,
    pub meshlib_topology_near_stitch_updates_origin_mismatches: usize,
    pub meshlib_topology_near_stitch_updates_previous_left_faces: usize,
    pub meshlib_topology_near_stitch_updates_next_right_faces: usize,
    pub meshlib_topology_near_stitch_updates_failed_other: usize,
    pub meshlib_topology_near_stitch_failed_details: Vec<MeshlibNearStitchFailureDiagnostic>,
    pub meshlib_topology_record_rewrite_applied_commands: usize,
    pub meshlib_topology_record_rewrite_failed_commands: usize,
    pub meshlib_topology_record_rewrite_failed_missing_targets: usize,
    pub meshlib_topology_record_rewrite_failed_closed_targets: usize,
    pub meshlib_topology_record_rewrite_failed_missing_sources: usize,
    pub meshlib_topology_record_rewrite_failed_other_commands: usize,
    pub meshlib_topology_record_rewrite_prepared_synthetic_targets: usize,
    pub meshlib_topology_record_rewrite_translated_face_records: usize,
    pub meshlib_topology_record_rewrite_apply_synthetic_sides: usize,
    pub meshlib_topology_record_rewrite_exported_faces: usize,
    pub meshlib_topology_record_rewrite_export_failed_faces: usize,
    pub meshlib_topology_record_rewrite_export_non_triangular_faces: usize,
    pub meshlib_topology_record_rewrite_export_left_ring_not_closed_faces: usize,
    pub meshlib_topology_record_rewrite_export_missing_origin_faces: usize,
    pub meshlib_topology_record_rewrite_export_other_failed_faces: usize,
    pub meshlib_topology_record_rewrite_export_changed_faces: bool,
    pub meshlib_topology_record_rewrite_apply_ready: bool,
    pub meshlib_topology_record_rewrite_exported_mesh_stats: Option<MeshStats>,
    pub meshlib_topology_record_rewrite_exported_mesh_health: Option<MeshHealth>,
    pub meshlib_topology_record_rewrite_packed_mesh_stats: Option<MeshStats>,
    pub meshlib_topology_record_rewrite_packed_mesh_health: Option<MeshHealth>,
    pub meshlib_topology_prepared_base_record_rewrite: MeshlibPreparedBaseRecordRewriteDiagnostics,
    pub meshlib_topology_rewrite_ready: bool,
    pub output_mesh_stats: MeshStats,
    pub output_mesh_health: MeshHealth,
    pub topology_splice_exported_mesh_stats: Option<MeshStats>,
    pub topology_splice_exported_mesh_health: Option<MeshHealth>,
    pub output_faces: usize,
    pub result_cut_paths: usize,
    pub result_cut_path_edges: usize,
    pub result_cut_closed_paths: usize,
    pub result_cut_mapped_paths: usize,
    pub result_cut_mapped_path_edges: usize,
    pub result_cut_mapped_closed_paths: usize,
    pub result_cut_paths_complete: bool,
    pub parity_ready: bool,
}

pub(super) struct ExactBooleanPipelineDiagnosticInputs<'a> {
    pub(super) first_cut: &'a ExactCutHoleFillResult,
    pub(super) second_cut: &'a ExactCutHoleFillResult,
    pub(super) stitch_plan: &'a ExactStitchPlan,
    pub(super) topology_splice_plan: &'a ExactTopologySplicePlan,
    pub(super) topology_splice_apply_plan: &'a ExactTopologySpliceApplyPlan,
    pub(super) assembly: &'a ExactBooleanAssemblyResult,
    pub(super) coplanar_cut_trial: Option<&'a ExactCoplanarContourCutTrial>,
    pub(super) paired_coplanar_candidate: Option<&'a PairedCoplanarCandidateDiagnostics>,
    pub(super) active_output_volume: f64,
    pub(super) operation: ExactBooleanOperation,
    pub(super) epsilon: f64,
}

pub(super) fn exact_boolean_pipeline_diagnostics(
    input: ExactBooleanPipelineDiagnosticInputs<'_>,
) -> Result<ExactBooleanPipelineDiagnostics, GeometryError> {
    let first_vertices_mixed_against_second = vertices_have_mixed_inside_state(
        &input.first_cut.mesh.vertices,
        &input.second_cut.mesh.vertices,
        &input.second_cut.mesh.faces,
        input.operation,
        input.epsilon,
    )?;
    let second_vertices_mixed_against_first = vertices_have_mixed_inside_state(
        &input.second_cut.mesh.vertices,
        &input.first_cut.mesh.vertices,
        &input.first_cut.mesh.faces,
        input.operation,
        input.epsilon,
    )?;
    let first_cut_edges = input.first_cut.mesh.cut_edges.len();
    let second_cut_edges = input.second_cut.mesh.cut_edges.len();
    let requires_topology_splice =
        requires_topology_splice(input.operation) && (first_cut_edges > 0 || second_cut_edges > 0);
    let possible_missing_cut_intersections = first_cut_edges == 0
        && second_cut_edges == 0
        && (first_vertices_mixed_against_second || second_vertices_mixed_against_first);
    let first_prepare_part_dividable = input.assembly.first_cut_paths_consistent;
    let second_prepare_part_dividable = input.assembly.second_cut_paths_consistent;
    let coplanar_contours = coplanar_overlap_contours(
        &input.first_cut.mesh.vertices,
        &input.first_cut.mesh.faces,
        &input.second_cut.mesh.vertices,
        &input.second_cut.mesh.faces,
        input.epsilon,
    )?;
    let coplanar_overlap_pairs = coplanar_contours.overlaps.len();
    let coplanar_overlap_region_edges = coplanar_contours
        .overlaps
        .iter()
        .map(|overlap| overlap.polygon.len())
        .sum();
    let coplanar_overlap_area = coplanar_contours
        .overlaps
        .iter()
        .map(|overlap| overlap.area)
        .sum();
    let coplanar_overlap_contours = coplanar_contours.contours.first.len();
    let coplanar_overlap_contour_edges = coplanar_contours
        .contours
        .first
        .iter()
        .map(|contour| contour.intersections.len())
        .sum();
    let paired_candidate = input.paired_coplanar_candidate.copied().unwrap_or_default();
    let paired_candidate_active_volume_delta =
        paired_candidate.output_volume - input.active_output_volume;
    let paired_candidate_preserves_active_volume =
        input.paired_coplanar_candidate.is_some_and(|candidate| {
            candidate.preserves_reference_volume(input.active_output_volume, input.epsilon)
        });
    let stitch_cut_path_length_mismatches = cut_path_length_mismatches(
        &input.first_cut.mesh.cut_edge_paths,
        &input.second_cut.mesh.cut_edge_paths,
    );
    let output_mesh_stats = mesh_stats(&input.assembly.vertices, &input.assembly.faces)?;
    let output_mesh_health = mesh_health(
        &input.assembly.vertices,
        &input.assembly.faces,
        true,
        Some(EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET),
        input.epsilon,
    )?;
    let (duplicate_output_face_groups, duplicate_output_faces) =
        duplicate_face_counts(&input.assembly.faces);
    let face_source_summary = face_source_summary(input.assembly);
    let raw_face_selection_summary = raw_face_selection_summary(&input, &face_source_summary)?;
    let result_cut_summary =
        meshlib_result_cut_path_summary(input.operation, input.first_cut, input.second_cut);
    let topology_splice_exported_mesh_stats = mesh_export_stats(
        &input.assembly.vertices,
        &input.topology_splice_apply_plan.exported_face_indices,
        input.topology_splice_apply_plan.export_failed_faces,
    )?;
    let topology_splice_exported_mesh_health = mesh_export_health(
        &input.assembly.vertices,
        &input.topology_splice_apply_plan.exported_face_indices,
        input.topology_splice_apply_plan.export_failed_faces,
        input.epsilon,
        EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET,
    )?;
    let topology_splice_export_changed_faces = input.topology_splice_apply_plan.export_failed_faces
        == 0
        && input.topology_splice_apply_plan.exported_face_indices != input.assembly.faces;
    let meshlib = meshlib_rewrite_diagnostics(MeshlibRewriteDiagnosticsInput {
        first_cut: &input.first_cut.mesh,
        second_cut: &input.second_cut.mesh,
        assembly: input.assembly,
        operation: input.operation,
        epsilon: input.epsilon,
    })?;
    let topology_splice_parity_ready = !requires_topology_splice
        || (input.topology_splice_apply_plan.ready_for_mutation
            && output_mesh_health.is_closed
            && output_mesh_health.boundary_edge_count == 0
            && output_mesh_health.nonmanifold_edge_count == 0
            && input.topology_splice_apply_plan.export_failed_faces == 0
            && input.topology_splice_apply_plan.exported_boundary_edges == 0
            && input.topology_splice_apply_plan.exported_non_manifold_edges == 0
            && input
                .topology_splice_apply_plan
                .duplicated_output_topology_edges
                == 0
            && duplicate_output_faces == 0);
    let parity_ready = input.first_cut.mesh.skipped_source_faces.is_empty()
        && input.second_cut.mesh.skipped_source_faces.is_empty()
        && input.stitch_plan.compatible
        && first_prepare_part_dividable
        && second_prepare_part_dividable
        && input.assembly.result_cut_paths_complete
        && topology_splice_parity_ready
        && !possible_missing_cut_intersections;

    Ok(ExactBooleanPipelineDiagnostics {
        first_cut_edges,
        second_cut_edges,
        first_skipped_source_faces: input.first_cut.mesh.skipped_source_faces.clone(),
        second_skipped_source_faces: input.second_cut.mesh.skipped_source_faces.clone(),
        first_fill_plans: input.first_cut.fill_plans.len(),
        second_fill_plans: input.second_cut.fill_plans.len(),
        stitch_compatible: input.stitch_plan.compatible,
        stitch_unmatched_first_edges: input.stitch_plan.unmatched_first_edges.len(),
        stitch_unmatched_second_edges: input.stitch_plan.unmatched_second_edges.len(),
        stitch_cut_path_length_mismatches,
        requires_topology_splice,
        first_vertices_mixed_against_second,
        second_vertices_mixed_against_first,
        possible_missing_cut_intersections,
        first_prepare_part_dividable,
        second_prepare_part_dividable,
        first_cut_path_side_components: input.assembly.first_cut_path_side_components,
        second_cut_path_side_components: input.assembly.second_cut_path_side_components,
        first_cut_path_overlap_components: input.assembly.first_cut_path_overlap_components,
        second_cut_path_overlap_components: input.assembly.second_cut_path_overlap_components,
        coplanar_overlap_pairs,
        coplanar_overlap_region_edges,
        coplanar_overlap_area,
        coplanar_overlap_contours,
        coplanar_overlap_contour_edges,
        coplanar_cut_trial_contours: trial_usize(input.coplanar_cut_trial, |trial| trial.contours),
        coplanar_cut_trial_contour_edges: trial_usize(input.coplanar_cut_trial, |trial| {
            trial.contour_edges
        }),
        coplanar_cut_trial_first_cut_edges: trial_usize(input.coplanar_cut_trial, |trial| {
            trial.first_cut_edges
        }),
        coplanar_cut_trial_second_cut_edges: trial_usize(input.coplanar_cut_trial, |trial| {
            trial.second_cut_edges
        }),
        paired_coplanar_cut_trial_contours: trial_usize(input.coplanar_cut_trial, |trial| {
            trial.paired_contours
        }),
        paired_coplanar_cut_trial_contour_edges: trial_usize(input.coplanar_cut_trial, |trial| {
            trial.paired_contour_edges
        }),
        paired_coplanar_cut_trial_first_cut_edges: trial_usize(input.coplanar_cut_trial, |trial| {
            trial.paired_first_cut_edges
        }),
        paired_coplanar_cut_trial_second_cut_edges: trial_usize(
            input.coplanar_cut_trial,
            |trial| trial.paired_second_cut_edges,
        ),
        paired_coplanar_stitch_cut_path_length_mismatches: trial_usize(
            input.coplanar_cut_trial,
            |trial| trial.paired_stitch_cut_path_length_mismatches,
        ),
        paired_coplanar_stitch_unmatched_first_edges: trial_usize(
            input.coplanar_cut_trial,
            |trial| trial.paired_stitch_unmatched_first_edges,
        ),
        paired_coplanar_stitch_unmatched_second_edges: trial_usize(
            input.coplanar_cut_trial,
            |trial| trial.paired_stitch_unmatched_second_edges,
        ),
        paired_coplanar_duplicate_first_path_edges: trial_usize(
            input.coplanar_cut_trial,
            |trial| trial.paired_duplicate_first_path_edges,
        ),
        paired_coplanar_duplicate_second_path_edges: trial_usize(
            input.coplanar_cut_trial,
            |trial| trial.paired_duplicate_second_path_edges,
        ),
        paired_coplanar_candidate_stitch_compatible: paired_candidate.stitch_compatible,
        paired_coplanar_candidate_first_prepare_part_dividable: paired_candidate
            .first_prepare_part_dividable,
        paired_coplanar_candidate_second_prepare_part_dividable: paired_candidate
            .second_prepare_part_dividable,
        paired_coplanar_candidate_first_cut_path_side_components: paired_candidate
            .first_cut_path_side_components,
        paired_coplanar_candidate_second_cut_path_side_components: paired_candidate
            .second_cut_path_side_components,
        paired_coplanar_candidate_first_cut_path_overlap_components: paired_candidate
            .first_cut_path_overlap_components,
        paired_coplanar_candidate_second_cut_path_overlap_components: paired_candidate
            .second_cut_path_overlap_components,
        paired_coplanar_candidate_result_cut_paths_complete: paired_candidate
            .result_cut_paths_complete,
        paired_coplanar_candidate_output_faces: paired_candidate.output_faces,
        paired_coplanar_candidate_output_area: paired_candidate.output_area,
        paired_coplanar_candidate_output_volume: paired_candidate.output_volume,
        paired_coplanar_candidate_self_intersections: paired_candidate.output_self_intersections,
        paired_coplanar_candidate_self_intersections_available: paired_candidate
            .output_self_intersections_available,
        paired_coplanar_candidate_active_volume_delta: paired_candidate_active_volume_delta,
        paired_coplanar_candidate_preserves_active_volume: paired_candidate_preserves_active_volume,
        paired_coplanar_candidate_boundary_edges: paired_candidate.boundary_edges,
        paired_coplanar_candidate_nonmanifold_edges: paired_candidate.nonmanifold_edges,
        paired_coplanar_candidate_duplicate_output_faces: paired_candidate.duplicate_output_faces,
        coplanar_cut_trial_first_skipped_faces: input
            .coplanar_cut_trial
            .map(|trial| trial.first_skipped_source_faces.clone())
            .unwrap_or_default(),
        coplanar_cut_trial_second_skipped_faces: input
            .coplanar_cut_trial
            .map(|trial| trial.second_skipped_source_faces.clone())
            .unwrap_or_default(),
        coplanar_cut_trial_accepted: input
            .coplanar_cut_trial
            .map(|trial| trial.accepted)
            .unwrap_or_default(),
        stitched_output_edges: input.topology_splice_plan.entries.len(),
        stitched_output_edges_with_two_faces: input.topology_splice_plan.manifold_edges,
        stitched_output_edges_needing_splice: input.topology_splice_plan.boundary_edges,
        topology_splice_ready: input.topology_splice_plan.ready_for_splice,
        topology_splice_missing_edges: input.topology_splice_plan.missing_edges,
        topology_splice_non_manifold_edges: input.topology_splice_plan.non_manifold_edges,
        topology_splice_apply_ready: input.topology_splice_apply_plan.ready_for_mutation,
        topology_splice_verified_boundary_edges: input
            .topology_splice_apply_plan
            .verified_boundary_edges,
        topology_splice_blocked_edges: input.topology_splice_apply_plan.blocked_edges,
        topology_splice_failed_edges: input.topology_splice_apply_plan.failed_edges,
        topology_splice_synthetic_side_edges: input.topology_splice_apply_plan.synthetic_side_edges,
        topology_splice_materialized_boundary_edges: input
            .topology_splice_apply_plan
            .materialized_boundary_edges,
        topology_splice_materialization_failed_edges: input
            .topology_splice_apply_plan
            .materialization_failed_edges,
        topology_splice_exported_faces: input.topology_splice_apply_plan.exported_faces,
        topology_splice_export_failed_faces: input.topology_splice_apply_plan.export_failed_faces,
        topology_splice_edges_before_materialization: input
            .topology_splice_apply_plan
            .topology_edges_before_materialization,
        topology_splice_edges_after_materialization: input
            .topology_splice_apply_plan
            .topology_edges_after_materialization,
        topology_splice_deleted_synthetic_edges: input
            .topology_splice_apply_plan
            .deleted_synthetic_stitch_edges,
        topology_splice_export_changed_faces,
        topology_splice_duplicated_output_edges: input
            .topology_splice_apply_plan
            .duplicated_output_topology_edges,
        topology_splice_duplicate_output_face_groups: duplicate_output_face_groups,
        topology_splice_duplicate_output_faces: duplicate_output_faces,
        meshlib_topology_base_faces: meshlib.topology_rewrite.base_faces,
        meshlib_topology_incoming_faces: meshlib.topology_rewrite.incoming_faces,
        meshlib_topology_selected_first_faces: face_source_summary.selected_first_faces,
        meshlib_topology_selected_second_faces: face_source_summary.selected_second_faces,
        meshlib_topology_raw_selected_faces: raw_face_selection_summary.raw_selected_faces,
        meshlib_topology_same_oriented_overlap_faces: raw_face_selection_summary.overlap_faces,
        meshlib_topology_boundary_misses: raw_face_selection_summary.boundary_misses,
        meshlib_topology_coplanar_selection_delta_faces: raw_face_selection_summary
            .selection_delta_faces,
        meshlib_topology_first_source_face_groups: face_source_summary.first_source_face_groups,
        meshlib_topology_second_source_face_groups: face_source_summary.second_source_face_groups,
        meshlib_topology_duplicate_first_source_faces: face_source_summary
            .duplicate_first_source_faces,
        meshlib_topology_duplicate_second_source_faces: face_source_summary
            .duplicate_second_source_faces,
        meshlib_topology_mapped_contour_edges: meshlib.topology_rewrite.mapped_contour_edges,
        meshlib_topology_missing_base_edges: meshlib.topology_rewrite.missing_base_contour_edges,
        meshlib_topology_missing_incoming_edges: meshlib
            .topology_rewrite
            .missing_incoming_contour_edges,
        meshlib_topology_direction_mismatches: meshlib
            .topology_rewrite
            .contour_direction_mismatches,
        meshlib_topology_mapped_stitch_contour_edges: meshlib
            .topology_rewrite
            .mapped_stitch_contour_edges,
        meshlib_topology_missing_stitch_contour_edges: meshlib
            .topology_rewrite
            .missing_stitch_contour_edges,
        meshlib_topology_synthetic_stitch_contour_edges: meshlib
            .topology_rewrite
            .synthetic_stitch_contour_edges,
        meshlib_topology_stitch_direction_mismatches: meshlib
            .topology_rewrite
            .stitch_direction_mismatches,
        meshlib_topology_stitch_metadata_ready: input.stitch_plan.compatible
            && input.assembly.result_cut_paths_complete
            && meshlib.topology_rewrite.stitch_metadata_ready,
        meshlib_topology_materialized_stitch_contour_edges: meshlib
            .topology_rewrite
            .materialized_stitch_contour_edges,
        meshlib_topology_unmaterialized_stitch_contour_edges: meshlib
            .topology_rewrite
            .unmaterialized_stitch_contour_edges,
        meshlib_topology_materialized_synthetic_stitch_sides: meshlib
            .topology_rewrite
            .materialized_synthetic_stitch_sides,
        meshlib_topology_stitch_materialization_direction_mismatches: meshlib
            .topology_rewrite
            .stitch_materialization_direction_mismatches,
        meshlib_topology_stitch_materialization_ready: input.stitch_plan.compatible
            && input.assembly.result_cut_paths_complete
            && meshlib.topology_rewrite.stitch_materialization_ready,
        meshlib_topology_record_rewrite_commands: meshlib.topology_rewrite.record_rewrite_commands,
        meshlib_topology_record_rewrite_blocked_edges: meshlib
            .topology_rewrite
            .record_rewrite_blocked_edges,
        meshlib_topology_record_rewrite_synthetic_sides: meshlib
            .topology_rewrite
            .record_rewrite_synthetic_sides,
        meshlib_topology_record_rewrite_direction_mismatches: meshlib
            .topology_rewrite
            .record_rewrite_direction_mismatches,
        meshlib_topology_record_rewrite_ready: input.stitch_plan.compatible
            && input.assembly.result_cut_paths_complete
            && meshlib.topology_rewrite.record_rewrite_ready,
        meshlib_topology_copied_edge_prepared_faces: meshlib.copied_edges.prepared_faces,
        meshlib_topology_copied_edge_prepared_vertices: meshlib.copied_edges.prepared_vertices,
        meshlib_topology_virtual_copied_vertices: meshlib.copied_edges.virtual_copied_vertices,
        meshlib_topology_copied_edge_prepared_edges: meshlib.copied_edges.prepared_edges,
        meshlib_topology_copied_edge_mapped_edges: meshlib.copied_edges.mapped_edges,
        meshlib_topology_copied_edges: meshlib.copied_edges.copied_edges,
        meshlib_topology_copied_edges_mapped_to_existing_output: meshlib
            .copied_edges
            .copied_edges_mapped_to_existing_output,
        meshlib_topology_copied_edges_mapped_to_output: meshlib
            .copied_edges
            .copied_edges_mapped_to_output,
        meshlib_topology_copied_edges_missing_output_vertices: meshlib
            .copied_edges
            .copied_edges_missing_output_vertices,
        meshlib_topology_copied_edge_translation_ready: meshlib
            .copied_edges
            .ready_for_record_translation(),
        meshlib_topology_open_stitch_paths: meshlib.topology_rewrite.open_stitch_paths,
        meshlib_topology_open_stitch_near_edge_updates: meshlib
            .topology_rewrite
            .open_stitch_near_edge_updates,
        meshlib_topology_open_stitch_near_edge_blocked_updates: meshlib
            .topology_rewrite
            .open_stitch_near_edge_blocked_updates,
        meshlib_topology_open_stitch_near_edge_ready: input.stitch_plan.compatible
            && input.assembly.result_cut_paths_complete
            && meshlib.topology_rewrite.open_stitch_near_edge_ready,
        meshlib_topology_near_stitch_update_commands: meshlib
            .record_rewrite_apply
            .near_stitch_update_commands,
        meshlib_topology_near_stitch_updates_applied: meshlib
            .record_rewrite_apply
            .applied_near_stitch_updates,
        meshlib_topology_near_stitch_updates_failed: meshlib
            .record_rewrite_apply
            .failed_near_stitch_updates,
        meshlib_topology_near_stitch_updates_failed_start: meshlib
            .record_rewrite_apply
            .failed_near_stitch_start_updates,
        meshlib_topology_near_stitch_updates_failed_end: meshlib
            .record_rewrite_apply
            .failed_near_stitch_end_updates,
        meshlib_topology_near_stitch_updates_missing_previous_edges: meshlib
            .record_rewrite_apply
            .failed_missing_near_stitch_previous_edges,
        meshlib_topology_near_stitch_updates_missing_next_edges: meshlib
            .record_rewrite_apply
            .failed_missing_near_stitch_next_edges,
        meshlib_topology_near_stitch_updates_origin_mismatches: meshlib
            .record_rewrite_apply
            .failed_near_stitch_origin_mismatches,
        meshlib_topology_near_stitch_updates_previous_left_faces: meshlib
            .record_rewrite_apply
            .failed_near_stitch_previous_left_faces,
        meshlib_topology_near_stitch_updates_next_right_faces: meshlib
            .record_rewrite_apply
            .failed_near_stitch_next_right_faces,
        meshlib_topology_near_stitch_updates_failed_other: meshlib
            .record_rewrite_apply
            .failed_other_near_stitch_updates,
        meshlib_topology_near_stitch_failed_details: meshlib::near_stitch_failure_details(
            &meshlib.record_rewrite_apply,
        ),
        meshlib_topology_record_rewrite_applied_commands: meshlib
            .record_rewrite_apply
            .applied_commands,
        meshlib_topology_record_rewrite_failed_commands: meshlib
            .record_rewrite_apply
            .failed_commands,
        meshlib_topology_record_rewrite_failed_missing_targets: meshlib
            .record_rewrite_apply
            .failed_missing_target_edges,
        meshlib_topology_record_rewrite_failed_closed_targets: meshlib
            .record_rewrite_apply
            .failed_closed_target_edges,
        meshlib_topology_record_rewrite_failed_missing_sources: meshlib
            .record_rewrite_apply
            .failed_missing_source_edges,
        meshlib_topology_record_rewrite_failed_other_commands: meshlib
            .record_rewrite_apply
            .failed_other_commands,
        meshlib_topology_record_rewrite_prepared_synthetic_targets: meshlib
            .record_rewrite_apply
            .prepared_synthetic_target_edges,
        meshlib_topology_record_rewrite_translated_face_records: meshlib
            .record_rewrite_apply
            .translated_face_records,
        meshlib_topology_record_rewrite_apply_synthetic_sides: meshlib
            .record_rewrite_apply
            .synthetic_side_edges,
        meshlib_topology_record_rewrite_exported_faces: meshlib.record_rewrite_apply.exported_faces,
        meshlib_topology_record_rewrite_export_failed_faces: meshlib
            .record_rewrite_apply
            .export_failed_faces,
        meshlib_topology_record_rewrite_export_non_triangular_faces: meshlib
            .record_rewrite_apply
            .export_non_triangular_faces,
        meshlib_topology_record_rewrite_export_left_ring_not_closed_faces: meshlib
            .record_rewrite_apply
            .export_left_ring_not_closed_faces,
        meshlib_topology_record_rewrite_export_missing_origin_faces: meshlib
            .record_rewrite_apply
            .export_missing_origin_faces,
        meshlib_topology_record_rewrite_export_other_failed_faces: meshlib
            .record_rewrite_apply
            .export_other_failed_faces,
        meshlib_topology_record_rewrite_export_changed_faces: meshlib
            .record_rewrite_apply
            .export_changed_faces,
        meshlib_topology_record_rewrite_apply_ready: input.stitch_plan.compatible
            && input.assembly.result_cut_paths_complete
            && meshlib.topology_rewrite.record_rewrite_ready
            && meshlib.near_stitch.blocked_updates == 0
            && meshlib.record_rewrite_apply.ready_for_export,
        meshlib_topology_record_rewrite_exported_mesh_stats: meshlib
            .record_rewrite_exported_mesh_stats,
        meshlib_topology_record_rewrite_exported_mesh_health: meshlib
            .record_rewrite_exported_mesh_health,
        meshlib_topology_record_rewrite_packed_mesh_stats: meshlib.record_rewrite_packed_mesh_stats,
        meshlib_topology_record_rewrite_packed_mesh_health: meshlib
            .record_rewrite_packed_mesh_health,
        meshlib_topology_prepared_base_record_rewrite: meshlib.prepared_base_record_rewrite,
        meshlib_topology_rewrite_ready: input.stitch_plan.compatible
            && input.assembly.result_cut_paths_complete
            && meshlib.topology_rewrite.ready_for_rewrite,
        output_mesh_stats,
        output_mesh_health,
        topology_splice_exported_mesh_stats,
        topology_splice_exported_mesh_health,
        output_faces: input.assembly.faces.len(),
        result_cut_paths: result_cut_summary.paths,
        result_cut_path_edges: result_cut_summary.path_edges,
        result_cut_closed_paths: result_cut_summary.closed_paths,
        result_cut_mapped_paths: input.assembly.result_cut_paths.len(),
        result_cut_mapped_path_edges: input.assembly.result_cut_paths.iter().map(Vec::len).sum(),
        result_cut_mapped_closed_paths: input
            .assembly
            .result_cut_path_closed
            .iter()
            .filter(|&&is_closed| is_closed)
            .count(),
        result_cut_paths_complete: input.assembly.result_cut_paths_complete,
        parity_ready,
    })
}
