use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_f32_values, read_faces, read_i64_values, read_vertices};

#[pyfunction(signature = (
    vertices,
    faces,
    seed_indices,
    thickness_values,
    min_target_thickness_mm,
    falloff_mm,
    deficit_scale = 0.75
))]
#[allow(clippy::too_many_arguments)]
fn local_thicken_to_minimum_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    seed_indices: PyReadonlyArray1<'_, i64>,
    thickness_values: PyReadonlyArray1<'_, f32>,
    min_target_thickness_mm: f64,
    falloff_mm: f64,
    deficit_scale: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_seed_indices = read_i64_values(seed_indices);
    let rust_thickness_values = read_f32_values(thickness_values);
    let displaced = py
        .detach(|| {
            zennah_geometry_core::local_thicken_to_minimum_vertices(
                &rust_vertices,
                &rust_faces,
                &rust_seed_indices,
                &rust_thickness_values,
                min_target_thickness_mm,
                falloff_mm,
                deficit_scale,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = displaced.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(local_thicken_to_minimum_vertices, module)?)?;
    Ok(())
}
