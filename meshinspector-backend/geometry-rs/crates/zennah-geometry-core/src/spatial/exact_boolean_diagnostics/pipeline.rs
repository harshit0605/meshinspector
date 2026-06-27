use super::super::exact_boolean::{
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation, ExactBooleanOutputMesh,
};
use super::super::exact_boolean_candidate::{
    ExactBooleanPipelineParts, PairedCoplanarCandidateDiagnostics,
};
use super::super::exact_coplanar::coplanar_overlap_contours;
use super::super::exact_cut_pair::ExactCoplanarContourCutTrial;
use super::super::exact_fill_apply::{
    exact_fill_cut_holes_with_replacements, ExactCutHoleFillResult,
};
use super::super::exact_splice::ExactTopologySplicePlan;
use super::super::exact_splice_apply::ExactTopologySpliceApplyPlan;
use super::super::exact_stitch::ExactStitchPlan;
use super::export::{mesh_export_health, mesh_export_stats};
use super::meshlib::{self, meshlib_rewrite_diagnostics, MeshlibRewriteDiagnosticsInput};
use super::output::output_mesh_diagnostics;
use super::prepare::{
    cut_mesh_with_shadow_repair_paths, cut_with_shadow_repair_paths,
    cut_without_trailing_shadow_repair_paths, first_path_source_face_count_deltas,
    lifecycle_slot_export_coverage, lifecycle_slot_export_groups, lifecycle_slot_face_coverage,
    lifecycle_slot_face_groups, meshlib_valid_cut_faces, non_empty_projected_cut_edge_paths,
    paired_replacement_prepare_diagnostics, projected_result_cut_edge_paths_without_shadow_repairs,
    shadow_repair_path_source_faces,
};
use super::result_cut::{cut_path_length_mismatches, meshlib_result_cut_path_summary};
use super::source::{
    cut_path_inventory, cut_source_face_inventory, duplicate_face_counts, face_source_summary,
    raw_face_selection_summary, stitch_result_cut_source_inventory,
};
use super::source_cut2origin::{
    meshlib_cut2origin_owner_remap_diagnostics, meshlib_cut2origin_owner_remapped_cut,
    meshlib_cut2origin_shadow_inventory,
};
use super::topology::{requires_topology_splice, vertices_have_mixed_inside_state};
use super::types::ExactBooleanPipelineDiagnostics;
use super::EXACT_BOOLEAN_SELF_INTERSECTION_FACE_BUDGET;
use crate::GeometryError;

pub(in crate::spatial) struct ExactBooleanPipelineDiagnosticInputs<'a> {
    pub(in crate::spatial) original_first_vertices: &'a [[f64; 3]],
    pub(in crate::spatial) original_first_faces: &'a [[i64; 3]],
    pub(in crate::spatial) original_second_vertices: &'a [[f64; 3]],
    pub(in crate::spatial) original_second_faces: &'a [[i64; 3]],
    pub(in crate::spatial) first_cut: &'a ExactCutHoleFillResult,
    pub(in crate::spatial) second_cut: &'a ExactCutHoleFillResult,
    pub(in crate::spatial) stitch_plan: &'a ExactStitchPlan,
    pub(in crate::spatial) topology_splice_plan: &'a ExactTopologySplicePlan,
    pub(in crate::spatial) topology_splice_apply_plan: &'a ExactTopologySpliceApplyPlan,
    pub(in crate::spatial) assembly: &'a ExactBooleanAssemblyResult,
    pub(in crate::spatial) output: &'a ExactBooleanOutputMesh,
    pub(in crate::spatial) coplanar_cut_trial: Option<&'a ExactCoplanarContourCutTrial>,
    pub(in crate::spatial) paired_coplanar_candidate:
        Option<&'a PairedCoplanarCandidateDiagnostics>,
    pub(in crate::spatial) paired_coplanar_candidate_parts: Option<&'a ExactBooleanPipelineParts>,
    pub(in crate::spatial) active_output_volume: f64,
    pub(in crate::spatial) operation: ExactBooleanOperation,
    pub(in crate::spatial) epsilon: f64,
}

#[rustfmt::skip]
pub(in crate::spatial) fn exact_boolean_pipeline_diagnostics(
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
    let paired_candidate = input.paired_coplanar_candidate.cloned().unwrap_or_default();
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
    let (output_mesh_stats, output_mesh_health) =
        output_mesh_diagnostics(input.output, input.epsilon)?;
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
        first_added_face_ranges: &input.first_cut.added_face_ranges,
        second_added_face_ranges: &input.second_cut.added_face_ranges,
        stitch_plan: Some(input.stitch_plan),
        assembly: input.assembly,
        operation: input.operation,
        epsilon: input.epsilon,
    })?;
    let paired_coplanar_candidate_meshlib = input
        .paired_coplanar_candidate_parts
        .map(|parts| {
            meshlib_rewrite_diagnostics(MeshlibRewriteDiagnosticsInput {
                first_cut: &parts.first_cut.mesh,
                second_cut: &parts.second_cut.mesh,
                first_added_face_ranges: &parts.first_cut.added_face_ranges,
                second_added_face_ranges: &parts.second_cut.added_face_ranges,
                stitch_plan: Some(&parts.stitch_plan),
                assembly: &parts.assembly,
                operation: input.operation,
                epsilon: input.epsilon,
            })
        })
        .transpose()?;
    let paired_coplanar_candidate_prepared_base_record_rewrite = paired_coplanar_candidate_meshlib
        .as_ref()
        .map(|diagnostics| diagnostics.prepared_base_record_rewrite.clone());
    let paired_coplanar_candidate_meshlib_connect = paired_coplanar_candidate_meshlib
        .as_ref()
        .map(|diagnostics| diagnostics.prepared_connect_summary);
    let (paired_first_cut_inventory, paired_second_cut_inventory) = input
        .paired_coplanar_candidate_parts
        .map(|parts| {
            (
                cut_source_face_inventory(&parts.first_cut),
                cut_source_face_inventory(&parts.second_cut),
            )
        })
        .unwrap_or_default();
    let paired_replacement_cuts = input
        .paired_coplanar_candidate_parts
        .map(|parts| -> Result<_, GeometryError> {
            let first_replacement_cut =
                exact_fill_cut_holes_with_replacements(&parts.first_cut.mesh, input.epsilon)?;
            let second_replacement_cut =
                exact_fill_cut_holes_with_replacements(&parts.second_cut.mesh, input.epsilon)?;
            Ok((first_replacement_cut, second_replacement_cut))
        })
        .transpose()?;
    let paired_shadow_repaired_replacement_cuts = input
        .paired_coplanar_candidate_parts
        .map(|parts| -> Result<_, GeometryError> {
            let first_shadow = cut_mesh_with_shadow_repair_paths(
                &parts.first_cut.mesh,
                &parts.first_shadow_repair_paths,
            );
            let second_shadow = cut_mesh_with_shadow_repair_paths(
                &parts.second_cut.mesh,
                &parts.second_shadow_repair_paths,
            );
            let first_replacement_cut =
                exact_fill_cut_holes_with_replacements(&first_shadow, input.epsilon)?;
            let second_replacement_cut =
                exact_fill_cut_holes_with_replacements(&second_shadow, input.epsilon)?;
            Ok((first_replacement_cut, second_replacement_cut))
        })
        .transpose()?;
    let paired_replacement_prepare = paired_replacement_prepare_diagnostics(
        paired_replacement_cuts.as_ref(),
        None,
        None,
        None,
        None,
        None,
        input.operation,
        input.epsilon,
        input.original_first_vertices,
        input.original_first_faces,
        input.original_second_vertices,
        input.original_second_faces,
    )?;
    let (paired_first_replacement_cut_inventory, paired_second_replacement_cut_inventory) =
        paired_replacement_cuts
            .as_ref()
            .map(|(first, second)| {
                (
                    cut_source_face_inventory(first),
                    cut_source_face_inventory(second),
                )
            })
            .unwrap_or_default();
    let (
        paired_first_shadow_repaired_replacement_cut_inventory,
        paired_second_shadow_repaired_replacement_cut_inventory,
    ) = paired_shadow_repaired_replacement_cuts
        .as_ref()
        .map(|(first, second)| {
            (
                cut_source_face_inventory(first),
                cut_source_face_inventory(second),
            )
        })
        .unwrap_or_default();
    let (paired_first_replacement_cut_path_inventory, paired_second_replacement_cut_path_inventory) =
        paired_replacement_cuts
            .as_ref()
            .map(|(first, second)| {
                (
                    cut_path_inventory(&first.mesh),
                    cut_path_inventory(&second.mesh),
                )
            })
            .unwrap_or_default();
    let (paired_first_cut2origin_shadow_inventory, paired_second_cut2origin_shadow_inventory) =
        match (
            input.paired_coplanar_candidate_parts,
            paired_replacement_cuts.as_ref(),
        ) {
            (Some(parts), Some((first, second))) => {
                let first_shadow =
                    cut_with_shadow_repair_paths(first, &parts.first_shadow_repair_paths);
                let second_shadow =
                    cut_with_shadow_repair_paths(second, &parts.second_shadow_repair_paths);
                (
                    meshlib_cut2origin_shadow_inventory(&first_shadow),
                    meshlib_cut2origin_shadow_inventory(&second_shadow),
                )
            }
            _ => Default::default(),
        };
    let paired_first_source_preserving_meshlib_like_cut2origin_source_face_counts = input
        .coplanar_cut_trial
        .map(|trial| {
            trial
                .paired_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts
                .clone()
        })
        .unwrap_or_default();
    let paired_second_source_preserving_meshlib_like_cut2origin_source_face_counts = input
        .coplanar_cut_trial
        .map(|trial| {
            trial
                .paired_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts
                .clone()
        })
        .unwrap_or_default();
    let paired_first_source_preserving_meshlib_like_cut2origin_source_faces = input
        .coplanar_cut_trial
        .map(|trial| {
            trial
                .paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces
                .clone()
        })
        .unwrap_or_default();
    let paired_second_source_preserving_meshlib_like_cut2origin_source_faces = input
        .coplanar_cut_trial
        .map(|trial| {
            trial
                .paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces
                .clone()
        })
        .unwrap_or_default();
    let paired_first_meshlib_valid_cut_faces = input
        .coplanar_cut_trial
        .map(|trial| {
            meshlib_valid_cut_faces(
                input.original_first_faces.len(),
                &trial.paired_combined_first_source_preserving_meshlib_like_cut2origin_source_faces,
                &trial
                    .paired_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates,
            )
        })
        .unwrap_or_default();
    let paired_second_meshlib_valid_cut_faces = input
        .coplanar_cut_trial
        .map(|trial| {
            meshlib_valid_cut_faces(
                input.original_second_faces.len(),
                &trial
                    .paired_combined_second_source_preserving_meshlib_like_cut2origin_source_faces,
                &trial
                    .paired_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates,
            )
        })
        .unwrap_or_default();
    let paired_first_cut2origin_shadow_owner_remap = meshlib_cut2origin_owner_remap_diagnostics(
        &paired_first_cut2origin_shadow_inventory,
        &paired_first_source_preserving_meshlib_like_cut2origin_source_faces,
    );
    let paired_second_cut2origin_shadow_owner_remap = meshlib_cut2origin_owner_remap_diagnostics(
        &paired_second_cut2origin_shadow_inventory,
        &paired_second_source_preserving_meshlib_like_cut2origin_source_faces,
    );
    let paired_owner_remapped_shadow_repaired_replacement_cuts =
        paired_shadow_repaired_replacement_cuts
            .as_ref()
            .and_then(|(first, second)| {
                Some((
                    meshlib_cut2origin_owner_remapped_cut(
                        first,
                        &paired_first_source_preserving_meshlib_like_cut2origin_source_faces,
                    )?,
                    meshlib_cut2origin_owner_remapped_cut(
                        second,
                        &paired_second_source_preserving_meshlib_like_cut2origin_source_faces,
                    )?,
                ))
            });
    let paired_owner_remapped_shadow_repaired_replacement_prepare_cuts =
        paired_owner_remapped_shadow_repaired_replacement_cuts
            .as_ref()
            .zip(input.paired_coplanar_candidate_parts)
            .map(|((first, second), parts)| {
                (
                    cut_without_trailing_shadow_repair_paths(
                        first,
                        &parts.first_shadow_repair_paths,
                    ),
                    cut_without_trailing_shadow_repair_paths(
                        second,
                        &parts.second_shadow_repair_paths,
                    ),
                )
            });
    let paired_first_meshlib_result_cut_edge_paths = input
        .coplanar_cut_trial
        .zip(input.paired_coplanar_candidate_parts)
        .map(|(trial, parts)| {
            projected_result_cut_edge_paths_without_shadow_repairs(
                &trial.paired_combined_first_source_preserving_meshlib_like_cut_edge_paths,
                parts.first_shadow_repair_paths.len(),
            )
        })
        .unwrap_or_default();
    let paired_second_meshlib_result_cut_edge_paths = input
        .coplanar_cut_trial
        .zip(input.paired_coplanar_candidate_parts)
        .map(|(trial, parts)| {
            projected_result_cut_edge_paths_without_shadow_repairs(
                &trial.paired_combined_second_source_preserving_meshlib_like_cut_edge_paths,
                parts.second_shadow_repair_paths.len(),
            )
        })
        .unwrap_or_default();
    let paired_owner_remapped_shadow_repaired_replacement_prepare =
        paired_replacement_prepare_diagnostics(
            paired_owner_remapped_shadow_repaired_replacement_cuts.as_ref(),
            paired_owner_remapped_shadow_repaired_replacement_prepare_cuts.as_ref(),
            Some(&paired_first_meshlib_valid_cut_faces),
            Some(&paired_second_meshlib_valid_cut_faces),
            non_empty_projected_cut_edge_paths(&paired_first_meshlib_result_cut_edge_paths),
            non_empty_projected_cut_edge_paths(&paired_second_meshlib_result_cut_edge_paths),
            input.operation,
            input.epsilon,
            input.original_first_vertices,
            input.original_first_faces,
            input.original_second_vertices,
            input.original_second_faces,
        )?;
    let paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_coverage =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_face_coverage(
                    &trial
                        .paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    &paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_selected_first_face_indices,
                )
            })
            .unwrap_or_default();
    let paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_coverage =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_face_coverage(
                    &trial
                        .paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    &paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_selected_second_face_indices,
                )
            })
            .unwrap_or_default();
    let paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_coverage =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_coverage(
                    &trial
                        .paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::First,
                )
            })
            .unwrap_or_default();
    let paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_coverage =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_coverage(
                    &trial
                        .paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::Second,
                )
            })
            .unwrap_or_default();
    let paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_coverage =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_coverage(
                    &trial
                        .paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_added_fill_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::First,
                )
            })
            .unwrap_or_default();
    let paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_coverage =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_coverage(
                    &trial
                        .paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_added_fill_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::Second,
                )
            })
            .unwrap_or_default();
    let paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_slots =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_face_groups(
                    &trial
                        .paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    &paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_selected_first_face_indices,
                )
            })
            .unwrap_or_default();
    let paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_slots =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_face_groups(
                    &trial
                        .paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    &paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_selected_second_face_indices,
                )
            })
            .unwrap_or_default();
    let paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_slots =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_groups(
                    &trial
                        .paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::First,
                )
            })
            .unwrap_or_default();
    let paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_slots =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_groups(
                    &trial
                        .paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::Second,
                )
            })
            .unwrap_or_default();
    let paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_slots =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_groups(
                    &trial
                        .paired_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_added_fill_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::First,
                )
            })
            .unwrap_or_default();
    let paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_slots =
        input
            .coplanar_cut_trial
            .map(|trial| {
                lifecycle_slot_export_groups(
                    &trial
                        .paired_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs,
                    paired_owner_remapped_shadow_repaired_replacement_prepare
                        .slot_projected_barriered_added_fill_prepared_base_record_rewrite
                        .as_ref(),
                    ExactBooleanOperand::Second,
                )
            })
            .unwrap_or_default();
    let (
        paired_first_owner_remapped_shadow_repaired_replacement_cut_inventory,
        paired_second_owner_remapped_shadow_repaired_replacement_cut_inventory,
    ) = paired_owner_remapped_shadow_repaired_replacement_cuts
        .as_ref()
        .map(|(first, second)| {
            (
                cut_source_face_inventory(first),
                cut_source_face_inventory(second),
            )
        })
        .unwrap_or_default();
    let paired_first_cut2origin_shadow_vs_source_preserving_source_face_count_deltas =
        first_path_source_face_count_deltas(
            &paired_first_cut2origin_shadow_inventory.source_face_counts,
            &paired_first_source_preserving_meshlib_like_cut2origin_source_face_counts,
        );
    let paired_second_cut2origin_shadow_vs_source_preserving_source_face_count_deltas =
        first_path_source_face_count_deltas(
            &paired_second_cut2origin_shadow_inventory.source_face_counts,
            &paired_second_source_preserving_meshlib_like_cut2origin_source_face_counts,
        );
    let (paired_first_cut_path_inventory, paired_second_cut_path_inventory) = input
        .paired_coplanar_candidate_parts
        .map(|parts| {
            (
                cut_path_inventory(&parts.first_cut.mesh),
                cut_path_inventory(&parts.second_cut.mesh),
            )
        })
        .unwrap_or_default();
    let paired_stitch_result_cut_source_inventory = input
        .paired_coplanar_candidate_parts
        .map(|parts| {
            stitch_result_cut_source_inventory(
                &parts.first_cut.mesh,
                &parts.second_cut.mesh,
                &parts.stitch_plan,
            )
        })
        .unwrap_or_default();
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

    build_exact_boolean_pipeline_diagnostics!(ExactBooleanPipelineDiagnostics, input, first_vertices_mixed_against_second, second_vertices_mixed_against_first, first_cut_edges, second_cut_edges, requires_topology_splice, possible_missing_cut_intersections, first_prepare_part_dividable, second_prepare_part_dividable, coplanar_overlap_pairs, coplanar_overlap_region_edges, coplanar_overlap_area, coplanar_overlap_contours, coplanar_overlap_contour_edges, paired_candidate, paired_candidate_active_volume_delta, paired_candidate_preserves_active_volume, stitch_cut_path_length_mismatches, output_mesh_stats, output_mesh_health, duplicate_output_face_groups, duplicate_output_faces, face_source_summary, raw_face_selection_summary, result_cut_summary, topology_splice_exported_mesh_stats, topology_splice_exported_mesh_health, topology_splice_export_changed_faces, meshlib, paired_coplanar_candidate_prepared_base_record_rewrite, paired_coplanar_candidate_meshlib_connect, paired_first_cut_inventory, paired_second_cut_inventory, paired_replacement_prepare, paired_first_replacement_cut_inventory, paired_second_replacement_cut_inventory, paired_first_shadow_repaired_replacement_cut_inventory, paired_second_shadow_repaired_replacement_cut_inventory, paired_first_replacement_cut_path_inventory, paired_second_replacement_cut_path_inventory, paired_first_cut2origin_shadow_inventory, paired_second_cut2origin_shadow_inventory, paired_first_source_preserving_meshlib_like_cut2origin_source_face_counts, paired_second_source_preserving_meshlib_like_cut2origin_source_face_counts, paired_first_meshlib_valid_cut_faces, paired_second_meshlib_valid_cut_faces, paired_first_cut2origin_shadow_owner_remap, paired_second_cut2origin_shadow_owner_remap, paired_owner_remapped_shadow_repaired_replacement_prepare, paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_coverage, paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_coverage, paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_coverage, paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_coverage, paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_coverage, paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_coverage, paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_slots, paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_selected_lifecycle_slots, paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_slots, paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_lifecycle_export_slots, paired_first_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_slots, paired_second_owner_remapped_shadow_repaired_replacement_slot_projected_added_fill_lifecycle_export_slots, paired_first_owner_remapped_shadow_repaired_replacement_cut_inventory, paired_second_owner_remapped_shadow_repaired_replacement_cut_inventory, paired_first_cut2origin_shadow_vs_source_preserving_source_face_count_deltas, paired_second_cut2origin_shadow_vs_source_preserving_source_face_count_deltas, paired_first_cut_path_inventory, paired_second_cut_path_inventory, paired_stitch_result_cut_source_inventory, parity_ready)
}
