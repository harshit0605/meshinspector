use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_i64_values, read_vec3, read_vertices};

#[pyfunction(signature = (vertices, scale_factor, ring_axis = None, preserve_indices = None))]
fn radial_scale_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    scale_factor: f64,
    ring_axis: Option<PyReadonlyArray1<'_, f64>>,
    preserve_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_ring_axis = ring_axis
        .map(|axis| read_vec3("ring_axis", axis))
        .transpose()?;
    let rust_preserve_indices = preserve_indices.map(read_i64_values).unwrap_or_default();
    let scaled = py
        .detach(|| {
            zennah_geometry_core::radial_scale_vertices(
                &rust_vertices,
                scale_factor,
                rust_ring_axis,
                &rust_preserve_indices,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = scaled.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (vertices, current_size, target_size, ring_axis = None, preserve_indices = None))]
fn resize_ring_vertices(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    current_size: f64,
    target_size: f64,
    ring_axis: Option<PyReadonlyArray1<'_, f64>>,
    preserve_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_ring_axis = ring_axis
        .map(|axis| read_vec3("ring_axis", axis))
        .transpose()?;
    let rust_preserve_indices = preserve_indices.map(read_i64_values).unwrap_or_default();
    let scaled = py
        .detach(|| {
            zennah_geometry_core::resize_ring_vertices(
                &rust_vertices,
                current_size,
                target_size,
                rust_ring_axis,
                &rust_preserve_indices,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = scaled.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(radial_scale_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(resize_ring_vertices, module)?)?;
    Ok(())
}
