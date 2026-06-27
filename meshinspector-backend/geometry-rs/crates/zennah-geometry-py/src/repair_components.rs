use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

fn component_prune_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::ComponentPruneReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_component_count", report.input_component_count)?;
    output.set_item("output_component_count", report.output_component_count)?;
    output.set_item("removed_component_count", report.removed_component_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    output.set_item("removed_face_count", report.removed_face_count)?;
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("removed_vertex_count", report.removed_vertex_count)?;
    output.set_item("retained_face_count", report.retained_face_count)?;
    output.set_item("min_area_mm2", report.min_area_mm2)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, min_area_mm2 = 0.0))]
fn prune_small_components(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    min_area_mm2: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::prune_small_components(&rust_vertices, &rust_faces, min_area_mm2)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = component_prune_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, eps_mm = 1e-5))]
fn weld_coincident_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    eps_mm: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::weld_coincident_vertices(&rust_vertices, &rust_faces, eps_mm)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = PyDict::new(py);
    report.set_item("input_vertex_count", result.report.input_vertex_count)?;
    report.set_item("output_vertex_count", result.report.output_vertex_count)?;
    report.set_item("merged_vertex_count", result.report.merged_vertex_count)?;
    report.set_item("input_face_count", result.report.input_face_count)?;
    report.set_item("output_face_count", result.report.output_face_count)?;
    report.set_item("removed_face_count", result.report.removed_face_count)?;
    report.set_item("eps_mm", result.report.eps_mm)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report.unbind())?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(prune_small_components, module)?)?;
    module.add_function(wrap_pyfunction!(weld_coincident_vertices, module)?)?;
    Ok(())
}
