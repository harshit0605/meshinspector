use numpy::{IntoPyArray, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

#[pyfunction(signature = (first_vertices, first_faces, second_vertices, second_faces, leaf_size = 16, epsilon = 1e-8))]
fn exact_mesh_intersections(
    py: Python<'_>,
    first_vertices: PyReadonlyArray2<'_, f64>,
    first_faces: PyReadonlyArray2<'_, i64>,
    second_vertices: PyReadonlyArray2<'_, f64>,
    second_faces: PyReadonlyArray2<'_, i64>,
    leaf_size: usize,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_first_vertices = read_vertices(first_vertices)?;
    let rust_first_faces = read_faces(first_faces)?;
    let rust_second_vertices = read_vertices(second_vertices)?;
    let rust_second_faces = read_faces(second_faces)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::exact_mesh_intersections(
                &rust_first_vertices,
                &rust_first_faces,
                &rust_second_vertices,
                &rust_second_faces,
                leaf_size,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let first_face_indices: Vec<i64> = result.iter().map(|entry| entry.first_face as i64).collect();
    let second_face_indices: Vec<i64> = result
        .iter()
        .map(|entry| entry.second_face as i64)
        .collect();
    let intersection_counts: Vec<i64> = result
        .iter()
        .map(|entry| entry.intersections.len() as i64)
        .collect();

    let output = PyDict::new(py);
    output.set_item("first_face_indices", first_face_indices.into_pyarray(py))?;
    output.set_item("second_face_indices", second_face_indices.into_pyarray(py))?;
    output.set_item("intersection_counts", intersection_counts.into_pyarray(py))?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(exact_mesh_intersections, module)?)?;
    Ok(())
}
