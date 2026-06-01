use numpy::PyReadonlyArray2;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_faces, read_vertices};

#[pyfunction(signature = (vertices, faces, max_listed_faces = 100, epsilon = 1e-8))]
fn service_mesh_health(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_listed_faces: usize,
    epsilon: f64,
) -> PyResult<Py<PyDict>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let health = py
        .detach(|| {
            zennah_geometry_core::service_mesh_health(
                &rust_vertices,
                &rust_faces,
                max_listed_faces,
                epsilon,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let output = PyDict::new(py);
    output.set_item("is_closed", health.is_closed)?;
    output.set_item("self_intersections", health.self_intersections)?;
    output.set_item("self_intersection_faces", health.self_intersection_faces)?;
    output.set_item("holes_count", health.holes_count)?;
    output.set_item("degenerate_faces", health.degenerate_faces)?;
    output.set_item("health_score", health.health_score)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(service_mesh_health, module)?)?;
    Ok(())
}
