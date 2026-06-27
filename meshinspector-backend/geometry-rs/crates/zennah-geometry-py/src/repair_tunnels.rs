use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

fn tunnel_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::TunnelDiagnostics,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("vertex_count", report.vertex_count)?;
    output.set_item("face_count", report.face_count)?;
    output.set_item("edge_count", report.edge_count)?;
    output.set_item(
        "connected_component_count",
        report.connected_component_count,
    )?;
    output.set_item("boundary_edge_count", report.boundary_edge_count)?;
    output.set_item("nonmanifold_edge_count", report.nonmanifold_edge_count)?;
    output.set_item("euler_characteristic", report.euler_characteristic)?;
    output.set_item("genus", report.genus)?;
    output.set_item("tunnel_count", report.tunnel_count)?;
    output.set_item("closed", report.closed)?;
    Ok(output.unbind())
}

fn tunnel_elimination_report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::TunnelEliminationReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item(
        "detected_tunnel_face_count",
        report.detected_tunnel_face_count,
    )?;
    output.set_item("removed_face_count", report.removed_face_count)?;
    output.set_item("filled_holes", report.filled_holes)?;
    output.set_item("added_faces", report.added_faces)?;
    output.set_item("output_face_count", report.output_face_count)?;
    output.set_item(
        "output_boundary_edge_count",
        report.output_boundary_edge_count,
    )?;
    output.set_item("output_tunnel_count", report.output_tunnel_count)?;
    output.set_item(
        "tunnel_face_indices",
        report
            .tunnel_face_indices
            .into_iter()
            .map(|face_index| face_index as i64)
            .collect::<Vec<_>>(),
    )?;
    Ok(output.unbind())
}

fn tunnel_elimination_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::TunnelEliminationResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = tunnel_elimination_report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

#[pyfunction]
fn tunnel_diagnostics(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::tunnel_diagnostics(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    tunnel_report_dict(py, result)
}

#[pyfunction]
fn detect_tunnel_faces(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Vec<i64>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::detect_tunnel_faces(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(result.into_iter().map(|face| face as i64).collect())
}

#[pyfunction]
fn eliminate_tunnels(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| zennah_geometry_core::eliminate_tunnels(&rust_vertices, &rust_faces))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    tunnel_elimination_result_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(tunnel_diagnostics, module)?)?;
    module.add_function(wrap_pyfunction!(detect_tunnel_faces, module)?)?;
    module.add_function(wrap_pyfunction!(eliminate_tunnels, module)?)?;
    Ok(())
}
