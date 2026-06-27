use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::super::copied_face::copied_face_record_details_from;
use super::super::near_stitch::near_stitch_failure_details;
use super::super::prepared_base_details::{
    copied_prev_next_edge_update_details_from, export_failed_face_details_from,
    set_optional_mesh_health, set_optional_mesh_stats,
};
use super::super::replay::{
    set_mapped_source_record_replay_diagnostics, MappedSourceRecordReplaySummary,
};
use super::super::rewrite::{
    record_rewrite_failed_command_details_from, record_rewrite_target_details_from,
};

pub(super) fn set_paired_prepared_base_record_rewrite(
    py: Python<'_>,
    output: &Bound<'_, PyDict>,
    diagnostics: &zennah_geometry_core::ExactBooleanPipelineDiagnostics,
) -> PyResult<()> {
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_prepared_base_record_rewrite
            .as_ref(),
    )?;
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_replacement_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_replacement_prepared_base_record_rewrite
            .as_ref(),
    )?;
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_replacement_barriered_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_replacement_barriered_prepared_base_record_rewrite
            .as_ref(),
    )?;
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_prepared_base_record_rewrite
            .as_ref(),
    )?;
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_barriered_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_barriered_prepared_base_record_rewrite
            .as_ref(),
    )?;
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_prepared_base_record_rewrite
            .as_ref(),
    )?;
    set_prepared_base_record_rewrite_value(
        py,
        output,
        "paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_prepared_base_record_rewrite",
        diagnostics
            .paired_coplanar_candidate_owner_remapped_shadow_repaired_replacement_slot_projected_barriered_added_fill_prepared_base_record_rewrite
            .as_ref(),
    )
}

fn set_prepared_base_record_rewrite_value(
    py: Python<'_>,
    output: &Bound<'_, PyDict>,
    key: &str,
    prepared_base: Option<&zennah_geometry_core::MeshlibPreparedBaseRecordRewriteDiagnostics>,
) -> PyResult<()> {
    let Some(prepared_base) = prepared_base else {
        output.set_item(key, py.None())?;
        return Ok(());
    };
    let value = PyDict::new(py);
    value.set_item("prepared_faces", prepared_base.prepared_faces)?;
    value.set_item("prepared_vertices", prepared_base.prepared_vertices)?;
    value.set_item("virtual_vertices", prepared_base.virtual_vertices)?;
    value.set_item("prepared_face_sources", prepared_base.prepared_face_sources)?;
    value.set_item("applied_commands", prepared_base.applied_commands)?;
    value.set_item("failed_commands", prepared_base.failed_commands)?;
    value.set_item(
        "record_failed_missing_targets",
        prepared_base.record_failed_missing_targets,
    )?;
    value.set_item(
        "record_failed_closed_targets",
        prepared_base.record_failed_closed_targets,
    )?;
    value.set_item(
        "record_failed_missing_sources",
        prepared_base.record_failed_missing_sources,
    )?;
    value.set_item(
        "record_failed_other_commands",
        prepared_base.record_failed_other_commands,
    )?;
    value.set_item(
        "record_rewrite_target_details",
        record_rewrite_target_details_from(py, &prepared_base.record_rewrite_target_details)?,
    )?;
    value.set_item(
        "record_rewrite_failed_command_details",
        record_rewrite_failed_command_details_from(
            py,
            &prepared_base.record_rewrite_failed_command_details,
        )?,
    )?;
    value.set_item(
        "record_rewrite_near_stitch_target_left_closures",
        prepared_base.record_rewrite_near_stitch_target_left_closures,
    )?;
    value.set_item(
        "record_rewrite_near_stitch_target_right_closures",
        prepared_base.record_rewrite_near_stitch_target_right_closures,
    )?;
    value.set_item(
        "translated_copied_edge_records",
        prepared_base.translated_copied_edge_records,
    )?;
    value.set_item(
        "translated_copied_face_records",
        prepared_base.translated_copied_face_records,
    )?;
    set_mapped_source_record_replay_diagnostics(
        &value,
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
    value.set_item(
        "copied_prev_next_edge_update_attempts",
        prepared_base.copied_prev_next_edge_update_attempts,
    )?;
    value.set_item(
        "copied_prev_next_edge_updates_applied",
        prepared_base.copied_prev_next_edge_updates_applied,
    )?;
    value.set_item(
        "copied_prev_next_edge_updates_skipped",
        prepared_base.copied_prev_next_edge_updates_skipped,
    )?;
    value.set_item(
        "copied_prev_next_edge_update_details",
        copied_prev_next_edge_update_details_from(
            py,
            &prepared_base.copied_prev_next_edge_update_details,
        )?,
    )?;
    value.set_item(
        "copied_face_record_details",
        copied_face_record_details_from(py, &prepared_base.copied_face_record_details)?,
    )?;
    value.set_item(
        "failed_copied_edge_records",
        prepared_base.failed_copied_edge_records,
    )?;
    value.set_item(
        "refreshed_face_records",
        prepared_base.refreshed_face_records,
    )?;
    value.set_item(
        "near_stitch_updates_applied",
        prepared_base.near_stitch_updates_applied,
    )?;
    value.set_item(
        "near_stitch_updates_failed",
        prepared_base.near_stitch_updates_failed,
    )?;
    value.set_item(
        "near_stitch_failed_start",
        prepared_base.near_stitch_failed_start,
    )?;
    value.set_item(
        "near_stitch_failed_end",
        prepared_base.near_stitch_failed_end,
    )?;
    value.set_item(
        "near_stitch_skipped_previous_left_source_edges",
        prepared_base.near_stitch_skipped_previous_left_source_edges,
    )?;
    value.set_item(
        "near_stitch_skipped_next_right_source_edges",
        prepared_base.near_stitch_skipped_next_right_source_edges,
    )?;
    value.set_item(
        "near_stitch_missing_previous_edges",
        prepared_base.near_stitch_missing_previous_edges,
    )?;
    value.set_item(
        "near_stitch_missing_next_edges",
        prepared_base.near_stitch_missing_next_edges,
    )?;
    value.set_item(
        "near_stitch_origin_mismatches",
        prepared_base.near_stitch_origin_mismatches,
    )?;
    value.set_item(
        "near_stitch_previous_left_faces",
        prepared_base.near_stitch_previous_left_faces,
    )?;
    value.set_item(
        "near_stitch_previous_left_copied_source_edges",
        prepared_base.near_stitch_previous_left_copied_source_edges,
    )?;
    value.set_item(
        "near_stitch_next_right_faces",
        prepared_base.near_stitch_next_right_faces,
    )?;
    value.set_item(
        "near_stitch_next_right_copied_source_edges",
        prepared_base.near_stitch_next_right_copied_source_edges,
    )?;
    value.set_item(
        "near_stitch_failed_other",
        prepared_base.near_stitch_failed_other,
    )?;
    value.set_item(
        "near_stitch_failed_details",
        near_stitch_failure_details(py, &prepared_base.near_stitch_failed_details)?,
    )?;
    value.set_item("exported_faces", prepared_base.exported_faces)?;
    value.set_item("export_failed_faces", prepared_base.export_failed_faces)?;
    value.set_item(
        "export_failed_face_indices",
        prepared_base.export_failed_face_indices.clone(),
    )?;
    value.set_item(
        "export_failed_face_details",
        export_failed_face_details_from(py, &prepared_base.export_failed_face_details)?,
    )?;
    value.set_item(
        "export_non_triangular_faces",
        prepared_base.export_non_triangular_faces,
    )?;
    value.set_item(
        "export_left_ring_not_closed_faces",
        prepared_base.export_left_ring_not_closed_faces,
    )?;
    value.set_item(
        "export_missing_origin_faces",
        prepared_base.export_missing_origin_faces,
    )?;
    value.set_item(
        "export_face_record_left_mismatch_faces",
        prepared_base.export_face_record_left_mismatch_faces,
    )?;
    value.set_item(
        "export_face_left_ring_mismatch_faces",
        prepared_base.export_face_left_ring_mismatch_faces,
    )?;
    value.set_item(
        "export_other_failed_faces",
        prepared_base.export_other_failed_faces,
    )?;
    value.set_item(
        "exported_face_operands",
        prepared_base_exported_face_operand_labels(prepared_base),
    )?;
    value.set_item(
        "exported_face_cut_faces",
        prepared_base.exported_face_cut_faces.clone(),
    )?;
    value.set_item(
        "exported_face_source_faces",
        prepared_base.exported_face_source_faces.clone(),
    )?;
    value.set_item("ready_for_export", prepared_base.ready_for_export)?;
    set_optional_mesh_stats(
        &value,
        py,
        "exported_mesh_stats",
        prepared_base.exported_mesh_stats.as_ref(),
    )?;
    set_optional_mesh_health(
        &value,
        py,
        "exported_mesh_health",
        prepared_base.exported_mesh_health.as_ref(),
    )?;
    set_optional_mesh_stats(
        &value,
        py,
        "packed_mesh_stats",
        prepared_base.packed_mesh_stats.as_ref(),
    )?;
    set_optional_mesh_health(
        &value,
        py,
        "packed_mesh_health",
        prepared_base.packed_mesh_health.as_ref(),
    )?;
    output.set_item(key, value)?;
    Ok(())
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
