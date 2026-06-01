use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::near_stitch::near_stitch_failure_details;
use super::paired_coplanar::set_paired_coplanar_candidate_diagnostics;
use super::prepared_base::prepared_base_record_rewrite_dict;

pub(super) fn exact_boolean_diagnostics_dict(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyDict>> {
    let diagnostics = &result.diagnostics;
    let assembly = &result.assembly;
    let output = PyDict::new(py);
    output.set_item("parity_ready", diagnostics.parity_ready)?;
    output.set_item("stitch_compatible", diagnostics.stitch_compatible)?;
    output.set_item(
        "stitch_unmatched_first_edges",
        diagnostics.stitch_unmatched_first_edges,
    )?;
    output.set_item(
        "stitch_unmatched_second_edges",
        diagnostics.stitch_unmatched_second_edges,
    )?;
    output.set_item(
        "stitch_cut_path_length_mismatches",
        diagnostics.stitch_cut_path_length_mismatches,
    )?;
    output.set_item(
        "meshlib_topology_rewrite_ready",
        diagnostics.meshlib_topology_rewrite_ready,
    )?;
    output.set_item(
        "meshlib_topology_open_stitch_paths",
        diagnostics.meshlib_topology_open_stitch_paths,
    )?;
    output.set_item(
        "meshlib_topology_copied_edge_prepared_faces",
        diagnostics.meshlib_topology_copied_edge_prepared_faces,
    )?;
    output.set_item(
        "meshlib_topology_copied_edge_prepared_vertices",
        diagnostics.meshlib_topology_copied_edge_prepared_vertices,
    )?;
    output.set_item(
        "meshlib_topology_virtual_copied_vertices",
        diagnostics.meshlib_topology_virtual_copied_vertices,
    )?;
    output.set_item(
        "meshlib_topology_copied_edge_prepared_edges",
        diagnostics.meshlib_topology_copied_edge_prepared_edges,
    )?;
    output.set_item(
        "meshlib_topology_copied_edge_mapped_edges",
        diagnostics.meshlib_topology_copied_edge_mapped_edges,
    )?;
    output.set_item(
        "meshlib_topology_copied_edges",
        diagnostics.meshlib_topology_copied_edges,
    )?;
    output.set_item(
        "meshlib_topology_copied_edges_mapped_to_existing_output",
        diagnostics.meshlib_topology_copied_edges_mapped_to_existing_output,
    )?;
    output.set_item(
        "meshlib_topology_copied_edges_mapped_to_output",
        diagnostics.meshlib_topology_copied_edges_mapped_to_output,
    )?;
    output.set_item(
        "meshlib_topology_copied_edges_missing_output_vertices",
        diagnostics.meshlib_topology_copied_edges_missing_output_vertices,
    )?;
    output.set_item(
        "meshlib_topology_copied_edge_translation_ready",
        diagnostics.meshlib_topology_copied_edge_translation_ready,
    )?;
    output.set_item(
        "meshlib_topology_open_stitch_near_edge_updates",
        diagnostics.meshlib_topology_open_stitch_near_edge_updates,
    )?;
    output.set_item(
        "meshlib_topology_open_stitch_near_edge_blocked_updates",
        diagnostics.meshlib_topology_open_stitch_near_edge_blocked_updates,
    )?;
    output.set_item(
        "meshlib_topology_open_stitch_near_edge_ready",
        diagnostics.meshlib_topology_open_stitch_near_edge_ready,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_update_commands",
        diagnostics.meshlib_topology_near_stitch_update_commands,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_applied",
        diagnostics.meshlib_topology_near_stitch_updates_applied,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_failed",
        diagnostics.meshlib_topology_near_stitch_updates_failed,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_failed_start",
        diagnostics.meshlib_topology_near_stitch_updates_failed_start,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_failed_end",
        diagnostics.meshlib_topology_near_stitch_updates_failed_end,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_missing_previous_edges",
        diagnostics.meshlib_topology_near_stitch_updates_missing_previous_edges,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_missing_next_edges",
        diagnostics.meshlib_topology_near_stitch_updates_missing_next_edges,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_origin_mismatches",
        diagnostics.meshlib_topology_near_stitch_updates_origin_mismatches,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_previous_left_faces",
        diagnostics.meshlib_topology_near_stitch_updates_previous_left_faces,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_next_right_faces",
        diagnostics.meshlib_topology_near_stitch_updates_next_right_faces,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_updates_failed_other",
        diagnostics.meshlib_topology_near_stitch_updates_failed_other,
    )?;
    output.set_item(
        "meshlib_topology_near_stitch_failed_details",
        near_stitch_failure_details(py, &diagnostics.meshlib_topology_near_stitch_failed_details)?,
    )?;
    output.set_item(
        "meshlib_topology_prepared_base_record_rewrite",
        prepared_base_record_rewrite_dict(py, result)?,
    )?;
    output.set_item("output_faces", diagnostics.output_faces)?;
    output.set_item("result_cut_paths", diagnostics.result_cut_paths)?;
    output.set_item("result_cut_path_edges", diagnostics.result_cut_path_edges)?;
    output.set_item(
        "result_cut_paths_complete",
        diagnostics.result_cut_paths_complete,
    )?;
    output.set_item(
        "meshlib_topology_base_faces",
        diagnostics.meshlib_topology_base_faces,
    )?;
    output.set_item(
        "meshlib_topology_incoming_faces",
        diagnostics.meshlib_topology_incoming_faces,
    )?;
    output.set_item("prepare_first_faces", assembly.prepare_first_faces.len())?;
    output.set_item("prepare_second_faces", assembly.prepare_second_faces.len())?;
    output.set_item("selected_first_faces", assembly.selected_first_faces.len())?;
    output.set_item(
        "selected_second_faces",
        assembly.selected_second_faces.len(),
    )?;
    output.set_item(
        "first_prepare_part_dividable",
        diagnostics.first_prepare_part_dividable,
    )?;
    output.set_item(
        "second_prepare_part_dividable",
        diagnostics.second_prepare_part_dividable,
    )?;
    output.set_item(
        "first_cut_path_side_components",
        diagnostics.first_cut_path_side_components,
    )?;
    output.set_item(
        "second_cut_path_side_components",
        diagnostics.second_cut_path_side_components,
    )?;
    output.set_item(
        "first_cut_path_overlap_components",
        diagnostics.first_cut_path_overlap_components,
    )?;
    output.set_item(
        "second_cut_path_overlap_components",
        diagnostics.second_cut_path_overlap_components,
    )?;
    set_paired_coplanar_candidate_diagnostics(&output, diagnostics)?;
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
