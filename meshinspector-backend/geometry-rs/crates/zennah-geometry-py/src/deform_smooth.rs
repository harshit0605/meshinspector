use numpy::{IntoPyArray, PyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_faces, read_vertices};

#[pyfunction(signature = (vertices, faces, iterations = 10, lamb = 0.5, nu = -0.53))]
fn taubin_smooth_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    iterations: i64,
    lamb: f64,
    nu: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let smoothed = py
        .detach(|| {
            zennah_geometry_core::taubin_smooth_vertices(
                &rust_vertices,
                &rust_faces,
                iterations,
                lamb,
                nu,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = smoothed.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(taubin_smooth_vertices, module)?)?;
    Ok(())
}
