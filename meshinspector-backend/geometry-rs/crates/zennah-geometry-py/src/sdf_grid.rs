use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::convert::{read_f32_values, read_shape3, read_vec3, read_vertices};

#[pyfunction]
fn sdf_cell_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let output = py
        .detach(|| zennah_geometry_core::sdf_cell_values(&rust_values, rust_shape))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (values, shape, iso_value = 0.0))]
fn sdf_occupancy(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    iso_value: f32,
) -> PyResult<Py<PyArray1<u8>>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let output = py
        .detach(|| zennah_geometry_core::sdf_occupancy(&rust_values, rust_shape, iso_value))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (values, shape, voxel_size_mm, iso_value = 0.0))]
fn estimate_sdf_volume(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    iso_value: f32,
) -> PyResult<f64> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    py.detach(|| {
        zennah_geometry_core::estimate_sdf_volume(
            &rust_values,
            rust_shape,
            voxel_size_mm,
            iso_value,
        )
    })
    .map_err(|error| PyValueError::new_err(error.to_string()))
}

#[pyfunction]
fn sample_sdf_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    points: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let rust_points = read_vertices(points)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::sample_sdf_values_batch(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                &rust_points,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn sample_sdf_gradients(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    points: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let rust_points = read_vertices(points)?;
    let output = py
        .detach(|| {
            zennah_geometry_core::sample_sdf_gradients_batch(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                &rust_points,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let flat: Vec<f32> = output.into_iter().flatten().collect();
    Ok(flat.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(sdf_cell_values, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_occupancy, module)?)?;
    module.add_function(wrap_pyfunction!(estimate_sdf_volume, module)?)?;
    module.add_function(wrap_pyfunction!(sample_sdf_values, module)?)?;
    module.add_function(wrap_pyfunction!(sample_sdf_gradients, module)?)?;
    Ok(())
}
