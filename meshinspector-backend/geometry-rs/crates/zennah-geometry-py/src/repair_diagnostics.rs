use numpy::PyReadonlyArray2;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::convert::{read_faces, read_vertices};

fn mesh_healer_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::MeshHealerReport,
) -> PyResult<Py<PyDict>> {
    let issues = PyList::empty(py);
    for issue in report.issues {
        let issue_dict = PyDict::new(py);
        issue_dict.set_item("issue_id", issue.issue_id)?;
        issue_dict.set_item("label", issue.label)?;
        issue_dict.set_item("count", issue.count)?;
        issue_dict.set_item("severity", issue.severity)?;
        issue_dict.set_item("rust_repair_available", issue.rust_repair_available)?;
        issue_dict.set_item("repair_command", issue.repair_command)?;
        issues.append(issue_dict)?;
    }

    let output = PyDict::new(py);
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("holes_count", report.holes_count)?;
    output.set_item("boundary_edge_count", report.boundary_edge_count)?;
    output.set_item("nonmanifold_edge_count", report.nonmanifold_edge_count)?;
    output.set_item("self_intersections", report.self_intersections)?;
    output.set_item(
        "self_intersections_available",
        report.self_intersections_available,
    )?;
    output.set_item("total_issue_count", report.total_issue_count)?;
    output.set_item("issue_category_count", report.issue_category_count)?;
    output.set_item("fixable_issue_count", report.fixable_issue_count)?;
    output.set_item("auto_repair_ready", report.auto_repair_ready)?;
    output.set_item("issues", issues)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    vertices,
    faces,
    merge_tolerance = 1e-6,
    area_epsilon = 1e-12,
    detect_self_intersections = true,
    max_self_intersection_faces = 50000,
    epsilon = 1e-8
))]
#[allow(clippy::too_many_arguments)]
fn mesh_healer_diagnostics(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    merge_tolerance: f64,
    area_epsilon: f64,
    detect_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let report = py
        .detach(|| {
            zennah_geometry_core::mesh_healer_diagnostics(
                &rust_vertices,
                &rust_faces,
                merge_tolerance,
                area_epsilon,
                detect_self_intersections,
                max_self_intersection_faces,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_healer_report_dict(py, report)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(mesh_healer_diagnostics, module)?)?;
    Ok(())
}
