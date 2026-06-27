use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

pub(super) fn set_optional_mesh_stats(
    output: &Bound<'_, PyDict>,
    py: Python<'_>,
    key: &str,
    stats: Option<&zennah_geometry_core::MeshStats>,
) -> PyResult<()> {
    let Some(stats) = stats else {
        output.set_item(key, py.None())?;
        return Ok(());
    };
    let value = PyDict::new(py);
    value.set_item("bbox_min", stats.bbox_min.to_vec())?;
    value.set_item("bbox_max", stats.bbox_max.to_vec())?;
    value.set_item("bbox_size", stats.bbox_size.to_vec())?;
    value.set_item("surface_area_mm2", stats.surface_area_mm2)?;
    value.set_item("volume_mm3", stats.volume_mm3)?;
    value.set_item("vertex_count", stats.vertex_count)?;
    value.set_item("face_count", stats.face_count)?;
    value.set_item("connected_components", stats.connected_components)?;
    value.set_item("boundary_edge_count", stats.boundary_edge_count)?;
    output.set_item(key, value)?;
    Ok(())
}

pub(super) fn set_optional_mesh_health(
    output: &Bound<'_, PyDict>,
    py: Python<'_>,
    key: &str,
    health: Option<&zennah_geometry_core::MeshHealth>,
) -> PyResult<()> {
    let Some(health) = health else {
        output.set_item(key, py.None())?;
        return Ok(());
    };
    let value = PyDict::new(py);
    value.set_item("is_closed", health.is_closed)?;
    value.set_item("holes_count", health.holes_count)?;
    value.set_item("boundary_edge_count", health.boundary_edge_count)?;
    value.set_item("nonmanifold_edge_count", health.nonmanifold_edge_count)?;
    value.set_item("self_intersections", health.self_intersections)?;
    value.set_item(
        "self_intersections_available",
        health.self_intersections_available,
    )?;
    output.set_item(key, value)?;
    Ok(())
}

pub(super) fn copied_prev_next_edge_update_details(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyList>> {
    let details = &result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite
        .copied_prev_next_edge_update_details;
    copied_prev_next_edge_update_details_from(py, details)
}

pub(super) fn copied_prev_next_edge_update_details_from(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibCopiedPrevNextEdgeUpdateDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("source_contour_edge_id", detail.source_contour_edge_id)?;
        item.set_item("target_contour_edge_id", detail.target_contour_edge_id)?;
        item.set_item("walked_source_edge_id", detail.walked_source_edge_id)?;
        item.set_item("update_kind", detail.update_kind)?;
        item.set_item("previous_edge_id", detail.previous_edge_id)?;
        item.set_item("next_edge_id", detail.next_edge_id)?;
        item.set_item("previous_origin", detail.previous_origin)?;
        item.set_item("next_origin", detail.next_origin)?;
        item.set_item("previous_left", detail.previous_left)?;
        item.set_item("next_right", detail.next_right)?;
        item.set_item("applied", detail.applied)?;
        item.set_item("skipped_reason", detail.skipped_reason)?;
        output.append(item)?;
    }
    Ok(output.unbind())
}

pub(super) fn export_failed_face_details(
    py: Python<'_>,
    result: &zennah_geometry_core::ExactBooleanPipelineResult,
) -> PyResult<Py<PyList>> {
    let details = &result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite
        .export_failed_face_details;
    export_failed_face_details_from(py, details)
}

pub(super) fn export_failed_face_details_from(
    py: Python<'_>,
    details: &[zennah_geometry_core::MeshlibFaceExportFailureDiagnostic],
) -> PyResult<Py<PyList>> {
    let output = PyList::empty(py);
    for detail in details {
        let item = PyDict::new(py);
        item.set_item("face_index", detail.face_index)?;
        item.set_item("face_edge_id", detail.face_edge_id)?;
        item.set_item("face_operand", detail.face_operand)?;
        item.set_item("face_cut_face", detail.face_cut_face)?;
        item.set_item("face_source_face", detail.face_source_face)?;
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
        item.set_item(
            "left_ring_destinations",
            detail.left_ring_destinations.clone(),
        )?;
        item.set_item("left_ring_left_faces", detail.left_ring_left_faces.clone())?;
        item.set_item(
            "left_ring_right_faces",
            detail.left_ring_right_faces.clone(),
        )?;
        item.set_item(
            "same_left_face_edge_ids",
            detail.same_left_face_edge_ids.clone(),
        )?;
        item.set_item(
            "same_left_face_record_next_edge_ids",
            detail.same_left_face_record_next_edge_ids.clone(),
        )?;
        item.set_item(
            "same_left_face_record_prev_edge_ids",
            detail.same_left_face_record_prev_edge_ids.clone(),
        )?;
        item.set_item(
            "same_left_face_next_edge_ids",
            detail.same_left_face_next_edge_ids.clone(),
        )?;
        item.set_item(
            "same_left_face_origins",
            detail.same_left_face_origins.clone(),
        )?;
        item.set_item(
            "same_left_face_destinations",
            detail.same_left_face_destinations.clone(),
        )?;
        item.set_item(
            "same_left_face_right_faces",
            detail.same_left_face_right_faces.clone(),
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
