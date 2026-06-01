use numpy::{IntoPyArray, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{parse_sdf_boolean_operation, read_f32_values, read_shape3, read_vec3};

fn mesh_arrays_to_dict(
    py: Python<'_>,
    result: zennah_geometry_core::MeshArrays,
) -> PyResult<Py<PyDict>> {
    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, iso_value = 0.0))]
fn finalized_marching_tetrahedra(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    iso_value: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::finalized_marching_tetrahedra(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (left, right, operation, origin, shape, voxel_size_mm, iso_value = 0.0))]
#[allow(clippy::too_many_arguments)]
fn finalized_sdf_boolean_marching_tetrahedra(
    py: Python<'_>,
    left: PyReadonlyArray1<'_, f32>,
    right: PyReadonlyArray1<'_, f32>,
    operation: &str,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    iso_value: f32,
) -> PyResult<Py<PyDict>> {
    let left_values = read_f32_values(left);
    let right_values = read_f32_values(right);
    let boolean_operation = parse_sdf_boolean_operation(operation)?;
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::finalized_sdf_boolean_marching_tetrahedra(
                &left_values,
                &right_values,
                boolean_operation,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, offset_mm, iso_value = 0.0))]
#[allow(clippy::too_many_arguments)]
fn finalized_sdf_offset_marching_tetrahedra(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    offset_mm: f64,
    iso_value: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::finalized_sdf_offset_marching_tetrahedra(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                offset_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, wall_thickness_mm, iso_value = 0.0))]
#[allow(clippy::too_many_arguments)]
fn finalized_sdf_shell_marching_tetrahedra(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    wall_thickness_mm: f64,
    iso_value: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::finalized_sdf_shell_marching_tetrahedra(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                wall_thickness_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    mesh_arrays_to_dict(py, result)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(finalized_marching_tetrahedra, module)?)?;
    module.add_function(wrap_pyfunction!(
        finalized_sdf_boolean_marching_tetrahedra,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        finalized_sdf_offset_marching_tetrahedra,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        finalized_sdf_shell_marching_tetrahedra,
        module
    )?)?;
    Ok(())
}
