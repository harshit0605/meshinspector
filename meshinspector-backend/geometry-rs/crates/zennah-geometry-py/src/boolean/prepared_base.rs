use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use super::near_stitch::near_stitch_failure_details;
use super::replay::{set_mapped_source_record_replay_diagnostics, MappedSourceRecordReplaySummary};
use super::rewrite::record_rewrite_target_details;

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
    output.set_item("ready_for_export", prepared_base.ready_for_export)?;
    Ok(output.unbind())
}

fn export_failed_face_details(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyList>> {
    let details = &result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite
        .export_failed_face_details;
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("face_index", detail.face_index)?;
        item.set_item("face_edge_id", detail.face_edge_id)?;
        item.set_item("face_operand", detail.face_operand)?;
        item.set_item("error", detail.error)?;
        item.set_item("left_ring_edge_ids", detail.left_ring_edge_ids.clone())?;
        item.set_item(
            "left_ring_record_next_edge_ids",
            detail.left_ring_record_next_edge_ids.clone(),
        )?;
        item.set_item(
            "left_ring_record_prev_edge_ids",
            detail.left_ring_record_prev_edge_ids.clone(),
        )?;
        item.set_item(
            "left_ring_next_edge_ids",
            detail.left_ring_next_edge_ids.clone(),
        )?;
        item.set_item("left_ring_origins", detail.left_ring_origins.clone())?;
        item.set_item("left_ring_left_faces", detail.left_ring_left_faces.clone())?;
        item.set_item(
            "left_ring_right_faces",
            detail.left_ring_right_faces.clone(),
        )?;
        item.set_item(
            "left_ring_repeated_edge_id",
            detail.left_ring_repeated_edge_id,
        )?;
        item.set_item(
            "left_ring_returned_to_start",
            detail.left_ring_returned_to_start,
        )?;
        output.append(item)?;
    }
    Ok(output.unbind())
}
