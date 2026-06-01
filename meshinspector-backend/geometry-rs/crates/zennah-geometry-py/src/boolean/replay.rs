use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) struct MappedSourceRecordReplaySummary<'a> {
    pub mapped_source_record_replays: usize,
    pub mapped_source_record_replays_on_near_stitch_targets: usize,
    pub mapped_source_record_replay_attempts: usize,
    pub mapped_source_record_replay_attempts_on_near_stitch_targets: usize,
    pub skipped_mapped_source_record_replays: usize,
    pub details: &'a [zennah_geometry_core::MeshlibPreparedSourceRecordReplayDiagnostic],
}

pub(super) fn set_mapped_source_record_replay_diagnostics(
    output: &Bound<'_, PyDict>,
    py: Python<'_>,
    summary: MappedSourceRecordReplaySummary<'_>,
) -> PyResult<()> {
    output.set_item(
        "mapped_source_record_replays",
        summary.mapped_source_record_replays,
    )?;
    output.set_item(
        "mapped_source_record_replays_on_near_stitch_targets",
        summary.mapped_source_record_replays_on_near_stitch_targets,
    )?;
    output.set_item(
        "mapped_source_record_replay_attempts",
        summary.mapped_source_record_replay_attempts,
    )?;
    output.set_item(
        "mapped_source_record_replay_attempts_on_near_stitch_targets",
        summary.mapped_source_record_replay_attempts_on_near_stitch_targets,
    )?;
    output.set_item(
        "skipped_mapped_source_record_replays",
        summary.skipped_mapped_source_record_replays,
    )?;
    output.set_item(
        "mapped_source_record_replay_details",
        mapped_source_record_replay_details(py, summary.details)?,
    )?;
    Ok(())
}

pub(super) fn mapped_source_record_replay_details(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibPreparedSourceRecordReplayDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("target_edge_id", detail.target_edge_id)?;
        item.set_item(
            "target_was_near_stitch_target",
            detail.target_was_near_stitch_target,
        )?;
        item.set_item("target_origin_before", detail.target_origin_before)?;
        item.set_item("target_left_before", detail.target_left_before)?;
        item.set_item("target_right_before", detail.target_right_before)?;
        item.set_item("target_origin_after", detail.target_origin_after)?;
        item.set_item("target_left_after", detail.target_left_after)?;
        item.set_item("target_right_after", detail.target_right_after)?;
        item.set_item("record_next_edge_id", detail.record_next_edge_id)?;
        item.set_item("record_left", detail.record_left)?;
        item.set_item("record_sym_prev_edge_id", detail.record_sym_prev_edge_id)?;
        item.set_item("applied", detail.applied)?;
        item.set_item("skipped_reason", detail.skipped_reason)?;
        output.append(item)?;
    }
    Ok(output.unbind())
}
