use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) fn copied_face_record_details(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyList>> {
    let details = &result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite
        .copied_face_record_details;
    copied_face_record_details_from(py, details)
}

pub(super) fn copied_face_record_details_from(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibCopiedFaceRecordDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("output_face", detail.output_face)?;
        item.set_item("cut_face", detail.cut_face)?;
        item.set_item("source_face", detail.source_face)?;
        item.set_item("selected_edge_id", detail.selected_edge_id)?;
        item.set_item("selected_source_edge_id", detail.selected_source_edge_id)?;
        item.set_item(
            "selected_source_edge_vertices",
            detail.selected_source_edge_vertices,
        )?;
        item.set_item(
            "selected_by_valid_left_ring",
            detail.selected_by_valid_left_ring,
        )?;
        item.set_item("selected_left_ring_valid", detail.selected_left_ring_valid)?;
        item.set_item("selected_left_ring_error", detail.selected_left_ring_error)?;
        item.set_item(
            "candidates",
            copied_face_record_candidates(py, &detail.candidates)?,
        )?;
        output.append(item)?;
    }
    Ok(output.unbind())
}

fn copied_face_record_candidates(
    py: Python<'_>,
    candidates: &[zennah_geometry_core::MeshlibCopiedFaceRecordCandidateDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for candidate in candidates {
        let item = PyDict::new(py);
        item.set_item("source_edge_id", candidate.source_edge_id)?;
        item.set_item("source_edge_vertices", candidate.source_edge_vertices)?;
        item.set_item("source_edge_left", candidate.source_edge_left)?;
        item.set_item("source_edge_right", candidate.source_edge_right)?;
        item.set_item("source_next_edge_id", candidate.source_next_edge_id)?;
        item.set_item("source_prev_edge_id", candidate.source_prev_edge_id)?;
        item.set_item("mapped_edge_id", candidate.mapped_edge_id)?;
        item.set_item("face_edge_id", candidate.face_edge_id)?;
        item.set_item("face_edge_origin", candidate.face_edge_origin)?;
        item.set_item("face_edge_destination", candidate.face_edge_destination)?;
        item.set_item("face_edge_left", candidate.face_edge_left)?;
        item.set_item("face_edge_right", candidate.face_edge_right)?;
        item.set_item("face_edge_next_edge_id", candidate.face_edge_next_edge_id)?;
        item.set_item("face_edge_prev_edge_id", candidate.face_edge_prev_edge_id)?;
        item.set_item(
            "face_edge_sym_next_edge_id",
            candidate.face_edge_sym_next_edge_id,
        )?;
        item.set_item(
            "face_edge_sym_prev_edge_id",
            candidate.face_edge_sym_prev_edge_id,
        )?;
        item.set_item(
            "face_edge_left_ring_next_edge_id",
            candidate.face_edge_left_ring_next_edge_id,
        )?;
        item.set_item("left_ring_valid", candidate.left_ring_valid)?;
        item.set_item("left_ring_error", candidate.left_ring_error)?;
        output.append(item)?;
    }
    Ok(output.unbind())
}
