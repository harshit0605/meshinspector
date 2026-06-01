use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{
    parse_sdf_boolean_operation, read_f32_values, read_faces, read_shape3, read_vec3, read_vertices,
};

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
fn sdf_offset_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    offset_mm: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let output = py
        .detach(|| zennah_geometry_core::sdf_offset_values(&rust_values, offset_mm))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn sdf_shell_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    wall_thickness_mm: f64,
) -> PyResult<Py<PyArray1<f32>>> {
    let rust_values = read_f32_values(values);
    let output = py
        .detach(|| zennah_geometry_core::sdf_shell_values(&rust_values, wall_thickness_mm))
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, iso_value = 0.0))]
fn extract_surface_mesh_from_sdf_cells(
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
            zennah_geometry_core::extract_surface_mesh_from_sdf_cells(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (left, right, operation, origin, shape, voxel_size_mm, iso_value = 0.0))]
#[allow(clippy::too_many_arguments)]
fn sdf_boolean_marching_tetrahedra(
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
            zennah_geometry_core::sdf_boolean_marching_tetrahedra(
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

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, offset_mm, iso_value = 0.0))]
#[allow(clippy::too_many_arguments)]
fn sdf_offset_marching_tetrahedra(
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
            zennah_geometry_core::sdf_offset_marching_tetrahedra(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                offset_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, wall_thickness_mm, iso_value = 0.0))]
#[allow(clippy::too_many_arguments)]
fn sdf_shell_marching_tetrahedra(
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
            zennah_geometry_core::sdf_shell_marching_tetrahedra(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                wall_thickness_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (vertices, values, origin, shape, voxel_size_mm, iso_value = 0.0, iterations = 3))]
#[allow(clippy::too_many_arguments)]
fn project_vertices_to_sdf(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    iso_value: f64,
    iterations: i64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let projected = py
        .detach(|| {
            zennah_geometry_core::project_vertices_to_sdf(
                &rust_vertices,
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                iso_value,
                iterations,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = projected.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    vertices,
    faces,
    values,
    origin,
    shape,
    voxel_size_mm,
    iso_value = 0.0,
    smooth_iterations = 1,
    smooth_strength = 0.2,
    projection_iterations = 3
))]
#[allow(clippy::too_many_arguments)]
fn refine_vertices_with_sdf(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    faces: PyReadonlyArray2<'_, i64>,
    values: PyReadonlyArray1<'_, f32>,
    origin: PyReadonlyArray1<'_, f64>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size_mm: f64,
    iso_value: f64,
    smooth_iterations: i64,
    smooth_strength: f64,
    projection_iterations: i64,
) -> PyResult<Py<PyArray1<f64>>> {
    let rust_vertices = read_vertices(vertices)?;
    let rust_faces = read_faces(faces)?;
    let rust_values = read_f32_values(values);
    let rust_origin = read_vec3("origin", origin)?;
    let rust_shape = read_shape3(shape)?;
    let refined = py
        .detach(|| {
            zennah_geometry_core::refine_vertices_with_sdf(
                &rust_vertices,
                &rust_faces,
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                iso_value,
                smooth_iterations,
                smooth_strength,
                projection_iterations,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output: Vec<f64> = refined.into_iter().flatten().collect();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (values, origin, shape, voxel_size_mm, iso_value = 0.0))]
fn marching_tetrahedra(
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
            zennah_geometry_core::marching_tetrahedra(
                &rust_values,
                rust_origin,
                rust_shape,
                voxel_size_mm,
                iso_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;

    let vertex_values: Vec<f64> = result.vertices.into_iter().flatten().collect();
    let face_values: Vec<i64> = result.faces.into_iter().flatten().collect();
    let output = PyDict::new(py);
    output.set_item("vertices", vertex_values.into_pyarray(py))?;
    output.set_item("faces", face_values.into_pyarray(py))?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(sdf_boolean_values, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_offset_values, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_shell_values, module)?)?;
    module.add_function(wrap_pyfunction!(
        extract_surface_mesh_from_sdf_cells,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(sdf_boolean_marching_tetrahedra, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_offset_marching_tetrahedra, module)?)?;
    module.add_function(wrap_pyfunction!(sdf_shell_marching_tetrahedra, module)?)?;
    module.add_function(wrap_pyfunction!(project_vertices_to_sdf, module)?)?;
    module.add_function(wrap_pyfunction!(refine_vertices_with_sdf, module)?)?;
    module.add_function(wrap_pyfunction!(marching_tetrahedra, module)?)?;
    Ok(())
}
