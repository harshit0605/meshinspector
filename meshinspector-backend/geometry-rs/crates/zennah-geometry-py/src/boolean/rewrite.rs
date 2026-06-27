use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) fn record_rewrite_target_details(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyList>> {
    let details = &result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite
        .record_rewrite_target_details;
    record_rewrite_target_details_from(py, details)
}

pub(super) fn record_rewrite_target_details_from(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibRecordRewriteTargetDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("stitch_pair_index", detail.stitch_pair_index)?;
        item.set_item("target_edge_id", detail.target_edge_id)?;
        item.set_item(
            "target_was_near_stitch_target",
            detail.target_was_near_stitch_target,
        )?;
        item.set_item("target_origin_before", detail.target_origin_before)?;
        item.set_item("target_left_before", detail.target_left_before)?;
        item.set_item("target_right_before", detail.target_right_before)?;
        item.set_item(
            "target_next_edge_id_before",
            detail.target_next_edge_id_before,
        )?;
        item.set_item(
            "target_prev_edge_id_before",
            detail.target_prev_edge_id_before,
        )?;
        item.set_item("target_origin_after", detail.target_origin_after)?;
        item.set_item("target_left_after", detail.target_left_after)?;
        item.set_item("target_right_after", detail.target_right_after)?;
        item.set_item(
            "target_next_edge_id_after",
            detail.target_next_edge_id_after,
        )?;
        item.set_item(
            "target_prev_edge_id_after",
            detail.target_prev_edge_id_after,
        )?;
        item.set_item("record_next_edge_id", detail.record_next_edge_id)?;
        item.set_item("record_left", detail.record_left)?;
        item.set_item("record_sym_prev_edge_id", detail.record_sym_prev_edge_id)?;
        output.append(item)?;
    }
    Ok(output.unbind())
}

pub(super) fn record_rewrite_failed_command_details_from(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibRecordRewriteFailedCommandDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("stitch_pair_index", detail.stitch_pair_index)?;
        item.set_item("error", detail.error)?;
        item.set_item("this_operand", detail.this_operand)?;
        item.set_item("from_operand", detail.from_operand)?;
        item.set_item("output_edge", detail.output_edge)?;
        item.set_item("this_contour_edge", detail.this_contour_edge)?;
        item.set_item("from_contour_edge", detail.from_contour_edge)?;
        item.set_item("this_source_edge_index", detail.this_source_edge_index)?;
        item.set_item("from_source_edge_index", detail.from_source_edge_index)?;
        item.set_item("this_source_edge", detail.this_source_edge)?;
        item.set_item("from_source_edge", detail.from_source_edge)?;
        item.set_item("this_side_synthetic", detail.this_side_synthetic)?;
        item.set_item("from_side_synthetic", detail.from_side_synthetic)?;
        item.set_item("synthetic_sides", detail.synthetic_sides)?;
        output.append(item)?;
    }
    Ok(output.unbind())
}
