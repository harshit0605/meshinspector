use numpy::{IntoPyArray, PyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_faces, read_vertices};

fn thickness_options(
    max_radius: f64,
    max_iters: usize,
    min_shrinkage: f64,
    min_angle_cos: f64,
    epsilon: f64,
) -> zennah_geometry_core::InSphereThicknessOptions {
    zennah_geometry_core::InSphereThicknessOptions {
        max_radius,
        max_iters,
        min_shrinkage,
        min_angle_cos,
        epsilon,
    }
}

#[pyfunction(signature = (
    vertices,
    faces,
    max_radius = 1.0,
    max_iters = 16,
    min_shrinkage = 0.99999,
    min_angle_cos = -1.0,
    epsilon = 1e-5
))]
#[allow(clippy::too_many_arguments)]
fn insphere_thickness_at_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_radius: f64,
    max_iters: usize,
    min_shrinkage: f64,
    min_angle_cos: f64,
    epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let values = py
        .detach(|| {
            zennah_geometry_core::insphere_thickness_at_vertices(
                &rust_vertices,
                &rust_faces,
                thickness_options(max_radius, max_iters, min_shrinkage, min_angle_cos, epsilon),
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(values.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    vertices,
    faces,
    max_radius = 1.0,
    max_iters = 16,
    min_shrinkage = 0.99999,
    min_angle_cos = -1.0,
    epsilon = 1e-5
))]
#[allow(clippy::too_many_arguments)]
fn service_thickness_at_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    max_radius: f64,
    max_iters: usize,
    min_shrinkage: f64,
    min_angle_cos: f64,
    epsilon: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let values = py
        .detach(|| {
            zennah_geometry_core::service_thickness_at_vertices(
                &rust_vertices,
                &rust_faces,
                thickness_options(max_radius, max_iters, min_shrinkage, min_angle_cos, epsilon),
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(values.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(insphere_thickness_at_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(service_thickness_at_vertices, module)?)?;
    Ok(())
}
