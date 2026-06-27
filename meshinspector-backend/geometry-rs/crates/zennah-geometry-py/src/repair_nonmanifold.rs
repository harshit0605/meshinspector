use numpy::IntoPyArray;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

fn report_dict(
    py: Python<'_>,
    report: zennah_geometry_core::DuplicateNonManifoldVerticesReport,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item(
        "input_nonmanifold_vertex_count",
        report.input_nonmanifold_vertex_count,
    )?;
    output.set_item(
        "output_nonmanifold_vertex_count",
        report.output_nonmanifold_vertex_count,
    )?;
    output.set_item("duplicated_vertex_count", report.duplicated_vertex_count)?;
    output.set_item("input_vertex_count", report.input_vertex_count)?;
    output.set_item("output_vertex_count", report.output_vertex_count)?;
    output.set_item("input_face_count", report.input_face_count)?;
    output.set_item("output_face_count", report.output_face_count)?;
    Ok(output.unbind())
}

fn result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::DuplicateNonManifoldVerticesResult,
) -> PyResult<Py<PyDict>> {
    let vertices: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let faces: Vec<i64> = result.faces.into_iter().flatten().collect();
    let report = report_dict(py, result.report)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vertices.into_pyarray(py))?;
    output.set_item("faces", faces.into_pyarray(py))?;
    output.set_item("report", report)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, faces, region_face_indices=None))]
fn duplicate_nonmanifold_vertices(
    py: Python<'_>,
    vertices: numpy::PyReadonlyArray2<'_, f64>,
    faces: numpy::PyReadonlyArray2<'_, i64>,
    region_face_indices: Option<Vec<usize>>,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let result = py
        .detach(|| {
            if let Some(region) = region_face_indices.as_deref() {
                zennah_geometry_core::duplicate_nonmanifold_vertices_in_region(
                    &rust_vertices,
                    &rust_faces,
                    region,
                )
            } else {
                zennah_geometry_core::duplicate_nonmanifold_vertices(&rust_vertices, &rust_faces)
            }
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    result_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(duplicate_nonmanifold_vertices, module)?)?;
    Ok(())
}
