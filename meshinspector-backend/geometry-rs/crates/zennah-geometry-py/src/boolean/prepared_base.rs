use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::copied_face::copied_face_record_details;
use super::near_stitch::near_stitch_failure_details;
use super::prepared_base_details::{
    copied_prev_next_edge_update_details, export_failed_face_details, set_optional_mesh_health,
    set_optional_mesh_stats,
};
use super::replay::{set_mapped_source_record_replay_diagnostics, MappedSourceRecordReplaySummary};
use super::rewrite::{record_rewrite_failed_command_details_from, record_rewrite_target_details};

pub(super) fn prepared_base_record_rewrite_dict(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyDict>> {
    let prepared_base = &result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    let output = PyDict::new(py);
    output.set_item("prepared_faces", prepared_base.prepared_faces)?;
    output.set_item("prepared_vertices", prepared_base.prepared_vertices)?;
    output.set_item("virtual_vertices", prepared_base.virtual_vertices)?;
    output.set_item("prepared_face_sources", prepared_base.prepared_face_sources)?;
    output.set_item("applied_commands", prepared_base.applied_commands)?;
    output.set_item("failed_commands", prepared_base.failed_commands)?;
    output.set_item(
        "record_failed_missing_targets",
        prepared_base.record_failed_missing_targets,
    )?;
    output.set_item(
        "record_failed_closed_targets",
        prepared_base.record_failed_closed_targets,
    )?;
    output.set_item(
        "record_failed_missing_sources",
        prepared_base.record_failed_missing_sources,
    )?;
    output.set_item(
        "record_failed_other_commands",
        prepared_base.record_failed_other_commands,
    )?;
    output.set_item(
        "record_rewrite_target_details",
        record_rewrite_target_details(py, result)?,
    )?;
    output.set_item(
        "record_rewrite_failed_command_details",
        record_rewrite_failed_command_details_from(
            py,
            &prepared_base.record_rewrite_failed_command_details,
        )?,
    )?;
    output.set_item(
        "record_rewrite_near_stitch_target_left_closures",
        prepared_base.record_rewrite_near_stitch_target_left_closures,
    )?;
    output.set_item(
        "record_rewrite_near_stitch_target_right_closures",
        prepared_base.record_rewrite_near_stitch_target_right_closures,
    )?;
    output.set_item(
        "translated_copied_edge_records",
        prepared_base.translated_copied_edge_records,
    )?;
    output.set_item(
        "translated_copied_face_records",
        prepared_base.translated_copied_face_records,
    )?;
    set_mapped_source_record_replay_diagnostics(
        &output,
        py,
        MappedSourceRecordReplaySummary {
            mapped_source_record_replays: prepared_base.mapped_source_record_replays,
            mapped_source_record_replays_on_near_stitch_targets: prepared_base
                .mapped_source_record_replays_on_near_stitch_targets,
            mapped_source_record_replay_attempts: prepared_base
                .mapped_source_record_replay_attempts,
            mapped_source_record_replay_attempts_on_near_stitch_targets: prepared_base
                .mapped_source_record_replay_attempts_on_near_stitch_targets,
            skipped_mapped_source_record_replays: prepared_base
                .skipped_mapped_source_record_replays,
            details: &prepared_base.mapped_source_record_replay_details,
        },
    )?;
    output.set_item(
        "copied_prev_next_edge_update_attempts",
        prepared_base.copied_prev_next_edge_update_attempts,
    )?;
    output.set_item(
        "copied_prev_next_edge_updates_applied",
        prepared_base.copied_prev_next_edge_updates_applied,
    )?;
    output.set_item(
        "copied_prev_next_edge_updates_skipped",
        prepared_base.copied_prev_next_edge_updates_skipped,
    )?;
    output.set_item(
        "copied_prev_next_edge_update_details",
        copied_prev_next_edge_update_details(py, result)?,
    )?;
    output.set_item(
        "copied_face_record_details",
        copied_face_record_details(py, result)?,
    )?;
    output.set_item(
        "failed_copied_edge_records",
        prepared_base.failed_copied_edge_records,
    )?;
    output.set_item(
        "refreshed_face_records",
        prepared_base.refreshed_face_records,
    )?;
    output.set_item(
        "near_stitch_updates_applied",
        prepared_base.near_stitch_updates_applied,
    )?;
    output.set_item(
        "near_stitch_updates_failed",
        prepared_base.near_stitch_updates_failed,
    )?;
    output.set_item(
        "near_stitch_failed_start",
        prepared_base.near_stitch_failed_start,
    )?;
    output.set_item(
        "near_stitch_failed_end",
        prepared_base.near_stitch_failed_end,
    )?;
    output.set_item(
        "near_stitch_skipped_previous_left_source_edges",
        prepared_base.near_stitch_skipped_previous_left_source_edges,
    )?;
    output.set_item(
        "near_stitch_skipped_next_right_source_edges",
        prepared_base.near_stitch_skipped_next_right_source_edges,
    )?;
    output.set_item(
        "near_stitch_missing_previous_edges",
        prepared_base.near_stitch_missing_previous_edges,
    )?;
    output.set_item(
        "near_stitch_missing_next_edges",
        prepared_base.near_stitch_missing_next_edges,
    )?;
    output.set_item(
        "near_stitch_origin_mismatches",
        prepared_base.near_stitch_origin_mismatches,
    )?;
    output.set_item(
        "near_stitch_previous_left_faces",
        prepared_base.near_stitch_previous_left_faces,
    )?;
    output.set_item(
        "near_stitch_previous_left_copied_source_edges",
        prepared_base.near_stitch_previous_left_copied_source_edges,
    )?;
    output.set_item(
        "near_stitch_next_right_faces",
        prepared_base.near_stitch_next_right_faces,
    )?;
    output.set_item(
        "near_stitch_next_right_copied_source_edges",
        prepared_base.near_stitch_next_right_copied_source_edges,
    )?;
    output.set_item(
        "near_stitch_failed_other",
        prepared_base.near_stitch_failed_other,
    )?;
    output.set_item(
        "near_stitch_failed_details",
        near_stitch_failure_details(py, &prepared_base.near_stitch_failed_details)?,
    )?;
    output.set_item("exported_faces", prepared_base.exported_faces)?;
    output.set_item("export_failed_faces", prepared_base.export_failed_faces)?;
    output.set_item(
        "export_failed_face_indices",
        prepared_base.export_failed_face_indices.clone(),
    )?;
    output.set_item(
        "export_failed_face_details",
        export_failed_face_details(py, result)?,
    )?;
    output.set_item(
        "export_non_triangular_faces",
        prepared_base.export_non_triangular_faces,
    )?;
    output.set_item(
        "export_left_ring_not_closed_faces",
        prepared_base.export_left_ring_not_closed_faces,
    )?;
    output.set_item(
        "export_missing_origin_faces",
        prepared_base.export_missing_origin_faces,
    )?;
    output.set_item(
        "export_face_record_left_mismatch_faces",
        prepared_base.export_face_record_left_mismatch_faces,
    )?;
    output.set_item(
        "export_face_left_ring_mismatch_faces",
        prepared_base.export_face_left_ring_mismatch_faces,
    )?;
    output.set_item(
        "export_other_failed_faces",
        prepared_base.export_other_failed_faces,
    )?;
    output.set_item(
        "exported_face_operands",
        prepared_base_exported_face_operand_labels(prepared_base),
    )?;
    output.set_item(
        "exported_face_cut_faces",
        prepared_base.exported_face_cut_faces.clone(),
    )?;
    output.set_item(
        "exported_face_source_faces",
        prepared_base.exported_face_source_faces.clone(),
    )?;
    set_optional_mesh_stats(
        &output,
        py,
        "exported_mesh_stats",
        prepared_base.exported_mesh_stats.as_ref(),
    )?;
    set_optional_mesh_health(
        &output,
        py,
        "exported_mesh_health",
        prepared_base.exported_mesh_health.as_ref(),
    )?;
    set_optional_mesh_stats(
        &output,
        py,
        "packed_mesh_stats",
        prepared_base.packed_mesh_stats.as_ref(),
    )?;
    set_optional_mesh_health(
        &output,
        py,
        "packed_mesh_health",
        prepared_base.packed_mesh_health.as_ref(),
    )?;
    output.set_item("ready_for_export", prepared_base.ready_for_export)?;
    Ok(output.unbind())
}

fn prepared_base_exported_face_operand_labels(
    prepared_base: &zennah_geometry_core::MeshlibPreparedBaseRecordRewriteDiagnostics,
) -> Vec<Option<&'static str>> {
    prepared_base
        .exported_face_operands
        .iter()
        .map(|operand| operand.map(prepared_base_operand_label))
        .collect()
}

fn prepared_base_operand_label(operand: zennah_geometry_core::ExactBooleanOperand) -> &'static str {
    match operand {
        zennah_geometry_core::ExactBooleanOperand::First => "first",
        zennah_geometry_core::ExactBooleanOperand::Second => "second",
    }
}
