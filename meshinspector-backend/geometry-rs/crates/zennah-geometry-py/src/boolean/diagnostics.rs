use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::near_stitch::near_stitch_failure_details;
use super::paired_coplanar::set_paired_coplanar_candidate_diagnostics;
use super::prepared_base::prepared_base_record_rewrite_dict;

macro_rules! set_diag {
    ($output:expr, $diagnostics:expr, $field:ident) => {
        $output.set_item(stringify!($field), $diagnostics.$field.clone())?
    };
}

fn output_mesh_source_label(
    source: zennah_geometry_core::ExactBooleanOutputMeshSource,
) -> &'static str {
    match source {
        zennah_geometry_core::ExactBooleanOutputMeshSource::Assembly => "assembly",
        zennah_geometry_core::ExactBooleanOutputMeshSource::MeshlibPreparedBaseExport => {
            "meshlib_prepared_base_export"
        }
    }
}

#[rustfmt::skip]
pub(super) fn exact_boolean_diagnostics_dict(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyDict>> {
    let diagnostics = &result.diagnostics;
    let assembly = &result.assembly;
    let output = PyDict::new(py);

    output.set_item("parity_ready", diagnostics.parity_ready)?;
    output.set_item(
        "output_mesh_source",
        output_mesh_source_label(diagnostics.output_mesh_source),
    )?;
    set_diag!(output, diagnostics, stitch_compatible);
    set_diag!(output, diagnostics, stitch_unmatched_first_edges);
    set_diag!(output, diagnostics, stitch_unmatched_second_edges);
    set_diag!(output, diagnostics, stitch_cut_path_length_mismatches);
    set_diag!(output, diagnostics, coplanar_cut_trial_contours);
    set_diag!(output, diagnostics, coplanar_cut_trial_contour_edges);
    set_diag!(output, diagnostics, coplanar_cut_trial_first_cut_edges);
    set_diag!(output, diagnostics, coplanar_cut_trial_second_cut_edges);
    set_diag!(output, diagnostics, paired_coplanar_cut_trial_contours);
    set_diag!(output, diagnostics, paired_coplanar_cut_trial_contour_edges);
    set_diag!(output, diagnostics, paired_coplanar_cut_trial_first_cut_edges);
    set_diag!(output, diagnostics, paired_coplanar_cut_trial_second_cut_edges);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_cut_path_lengths);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_cut_path_lengths);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_cut_path_source_faces);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_cut_path_source_faces);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_cut_path_source_face_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_cut_path_source_face_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_collapsed_cut_path_lengths);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_collapsed_cut_path_lengths);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_collapsed_cut_path_source_faces);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_collapsed_cut_path_source_faces);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_collapsed_cut_path_source_face_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_collapsed_cut_path_source_face_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_cut_path_lengths);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_cut_path_lengths);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_cut_path_source_faces
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_cut_path_source_faces
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_cut_path_source_face_runs
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_cut_path_source_face_runs
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_cut_path_collapsed);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_cut_path_collapsed);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_cut_path_start_primitive_kinds
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_cut_path_start_primitive_kinds
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_cut_path_start_primitive_faces
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_cut_path_start_primitive_faces
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_order_rotations
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_order_rotations
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_cut_path_start_primitive_faces
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_cut_path_start_primitive_faces
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_cut_path_collapsed
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_cut_path_collapsed
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_like_cut_edge_paths);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_like_cut_edge_paths);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_removed_face_owner_candidates
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_removed_face_owner_candidates
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidates
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_collapsed_removed_face_owner_candidate_runs
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_removed_face_owner_candidate_runs
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_removed_face_owner_candidate_runs
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_source_faces);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_source_faces);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_source_face_counts
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_source_face_counts
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_source_face_runs
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_source_face_runs
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_lifecycle_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_lifecycle_runs);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_replacement_lifecycle_slot_runs
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_replacement_lifecycle_slot_runs
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_faces);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_faces);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_face_counts
    );
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_face_counts
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_like_cut2origin_source_face_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_like_cut2origin_source_face_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_removed_face_owner_candidates);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_removed_face_owner_candidates);
    set_diag!(output, diagnostics, paired_coplanar_combined_first_source_preserving_meshlib_removed_face_owner_candidate_runs);
    set_diag!(output, diagnostics, paired_coplanar_combined_second_source_preserving_meshlib_removed_face_owner_candidate_runs);
    set_diag!(
        output,
        diagnostics,
        paired_coplanar_combined_source_preserving_meshlib_removed_face_owner_missing_records
    );
    set_diag!(output, diagnostics, paired_coplanar_combined_duplicate_first_path_edge_occurrences);
    set_diag!(output, diagnostics, paired_coplanar_combined_duplicate_second_path_edge_occurrences);
    set_diag!(output, diagnostics, paired_coplanar_combined_duplicate_first_path_edge_path_indices);
    set_diag!(output, diagnostics, paired_coplanar_combined_duplicate_second_path_edge_path_indices);
    set_diag!(output, diagnostics, paired_coplanar_stitch_cut_path_length_mismatches);
    set_diag!(output, diagnostics, paired_coplanar_stitch_unmatched_first_edges);
    set_diag!(output, diagnostics, paired_coplanar_stitch_unmatched_second_edges);
    set_diag!(output, diagnostics, paired_coplanar_duplicate_first_path_edges);
    set_diag!(output, diagnostics, paired_coplanar_duplicate_second_path_edges);
    set_diag!(output, diagnostics, paired_coplanar_duplicate_first_path_edge_occurrences);
    set_diag!(output, diagnostics, paired_coplanar_duplicate_second_path_edge_occurrences);
    set_diag!(output, diagnostics, paired_coplanar_duplicate_first_path_edge_path_indices);
    set_diag!(output, diagnostics, paired_coplanar_duplicate_second_path_edge_path_indices);
    set_diag!(output, diagnostics, meshlib_topology_rewrite_ready);
    set_diag!(output, diagnostics, meshlib_topology_open_stitch_paths);
    set_diag!(output, diagnostics, meshlib_topology_copied_edge_prepared_faces);
    set_diag!(output, diagnostics, meshlib_topology_copied_edge_prepared_vertices);
    set_diag!(output, diagnostics, meshlib_topology_virtual_copied_vertices);
    set_diag!(output, diagnostics, meshlib_topology_copied_edge_prepared_edges);
    set_diag!(output, diagnostics, meshlib_topology_copied_edge_mapped_edges);
    set_diag!(output, diagnostics, meshlib_topology_copied_edges);
    set_diag!(output, diagnostics, meshlib_topology_copied_edges_mapped_to_existing_output);
    set_diag!(output, diagnostics, meshlib_topology_copied_edges_mapped_to_output);
    set_diag!(output, diagnostics, meshlib_topology_copied_edges_missing_output_vertices);
    set_diag!(output, diagnostics, meshlib_topology_copied_edge_translation_ready);
    set_diag!(output, diagnostics, meshlib_topology_open_stitch_near_edge_updates);
    set_diag!(output, diagnostics, meshlib_topology_open_stitch_near_edge_blocked_updates);
    set_diag!(output, diagnostics, meshlib_topology_open_stitch_near_edge_ready);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_update_commands);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_applied);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_failed);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_failed_start);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_failed_end);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_missing_previous_edges);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_missing_next_edges);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_origin_mismatches);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_previous_left_faces);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_next_right_faces);
    set_diag!(output, diagnostics, meshlib_topology_near_stitch_updates_failed_other);
    output.set_item(
        "meshlib_topology_near_stitch_failed_details",
        near_stitch_failure_details(py, &diagnostics.meshlib_topology_near_stitch_failed_details)?,
    )?;
    output.set_item(
        "meshlib_topology_prepared_base_record_rewrite",
        prepared_base_record_rewrite_dict(py, result)?,
    )?;
    set_diag!(output, diagnostics, output_faces);
    set_diag!(output, diagnostics, result_cut_paths);
    set_diag!(output, diagnostics, result_cut_path_edges);
    set_diag!(output, diagnostics, result_cut_paths_complete);
    set_diag!(output, diagnostics, meshlib_topology_base_faces);
    set_diag!(output, diagnostics, meshlib_topology_incoming_faces);
    output.set_item("prepare_first_faces", assembly.prepare_first_faces.len())?;
    output.set_item("prepare_second_faces", assembly.prepare_second_faces.len())?;
    output.set_item("selected_first_faces", assembly.selected_first_faces.len())?;
    output.set_item("selected_second_faces", assembly.selected_second_faces.len())?;
    output.set_item("prepare_first_face_indices", assembly.prepare_first_faces.clone())?;
    output.set_item("prepare_second_face_indices", assembly.prepare_second_faces.clone())?;
    output.set_item("selected_first_face_indices", assembly.selected_first_faces.clone())?;
    output.set_item("selected_second_face_indices", assembly.selected_second_faces.clone())?;
    set_diag!(output, diagnostics, first_prepare_part_dividable);
    set_diag!(output, diagnostics, second_prepare_part_dividable);
    set_diag!(output, diagnostics, first_cut_path_side_components);
    set_diag!(output, diagnostics, second_cut_path_side_components);
    set_diag!(output, diagnostics, first_cut_path_overlap_components);
    set_diag!(output, diagnostics, second_cut_path_overlap_components);
    set_paired_coplanar_candidate_diagnostics(py, &output, diagnostics)?;
    output.set_item(
        "boundary_edge_count",
        diagnostics.output_mesh_health.boundary_edge_count,
    )?;
    output.set_item(
        "nonmanifold_edge_count",
        diagnostics.output_mesh_health.nonmanifold_edge_count,
    )?;
    output.set_item("is_closed", diagnostics.output_mesh_health.is_closed)?;
    Ok(output.unbind())
}
