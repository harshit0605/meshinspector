use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{parse_sdf_boolean_operation, parse_voxel_binary_operation, read_f32_values};

#[pyfunction]
fn sdf_boolean_values(
    py: Python<'_>,
    left: PyReadonlyArray1<'_, f32>,
    right: PyReadonlyArray1<'_, f32>,
    operation: &str,
) -> PyResult<Py<PyArray1<f32>>> {
    let left_values = read_f32_values(left);
    let right_values = read_f32_values(right);
    let boolean_operation = parse_sdf_boolean_operation(operation)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::sdf_boolean_values(&left_values, &right_values, boolean_operation)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn voxel_binary_values(
    py: Python<'_>,
    left: PyReadonlyArray1<'_, f32>,
    right: PyReadonlyArray1<'_, f32>,
    operation: &str,
) -> PyResult<Py<PyArray1<f32>>> {
    let left_values = read_f32_values(left);
    let right_values = read_f32_values(right);
    let binary_operation = parse_voxel_binary_operation(operation)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::voxel_binary_values(&left_values, &right_values, binary_operation)
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn voxel_binary_iso_value(left_iso: f32, right_iso: f32, operation: &str) -> PyResult<f32> {
    let binary_operation = parse_voxel_binary_operation(operation)?;
    Ok(zennah_geometry_core::voxel_binary_iso_value(
        left_iso,
        right_iso,
        binary_operation,
    ))
}

#[pyfunction]
fn voxel_value_range(py: Python<'_>, values: PyReadonlyArray1<'_, f32>) -> PyResult<(f32, f32)> {
    let rust_values = read_f32_values(values);
    py.detach(|| zennah_geometry_core::voxel_value_range(&rust_values))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(sdf_boolean_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_binary_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_binary_iso_value, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_value_range, module)?)?;
    Ok(())
}
