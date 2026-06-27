use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::multiway::{
    multiway_registration_result_dict, read_multiway_objects, read_multiway_objects_and_normals,
};
use super::parse_icp_mode;

#[pyfunction(signature = (
    points,
    object_point_counts,
    max_group_size = 64,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_sequential_cascade_point_to_point_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let rust_objects = read_multiway_objects(points, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_sequential_cascade_point_to_point_icp(
                &rust_objects,
                max_group_size,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_point_sequential_cascade", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_group_size = 64,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_sequential_cascade_point_to_plane_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let (rust_objects, rust_object_normals) =
        read_multiway_objects_and_normals(points, normals, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_sequential_cascade_point_to_plane_icp(
                &rust_objects,
                &rust_object_normals,
                max_group_size,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_plane_sequential_cascade", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_group_size = 64,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_sequential_cascade_combined_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let (rust_objects, rust_object_normals) =
        read_multiway_objects_and_normals(points, normals, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_sequential_cascade_combined_icp(
                &rust_objects,
                &rust_object_normals,
                max_group_size,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "combined_sequential_cascade", mode)
}

#[pyfunction(signature = (
    points,
    object_point_counts,
    max_group_size = 64,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_aabb_cascade_point_to_point_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let rust_objects = read_multiway_objects(points, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_aabb_cascade_point_to_point_icp(
                &rust_objects,
                max_group_size,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_point_aabb_cascade", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_group_size = 64,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_aabb_cascade_point_to_plane_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let (rust_objects, rust_object_normals) =
        read_multiway_objects_and_normals(points, normals, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_aabb_cascade_point_to_plane_icp(
                &rust_objects,
                &rust_object_normals,
                max_group_size,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_plane_aabb_cascade", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_group_size = 64,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_aabb_cascade_combined_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_group_size: usize,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let (rust_objects, rust_object_normals) =
        read_multiway_objects_and_normals(points, normals, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_aabb_cascade_combined_icp(
                &rust_objects,
                &rust_object_normals,
                max_group_size,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "combined_aabb_cascade", mode)
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(
        multiway_sequential_cascade_point_to_point_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        multiway_sequential_cascade_point_to_plane_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        multiway_sequential_cascade_combined_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        multiway_aabb_cascade_point_to_point_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        multiway_aabb_cascade_point_to_plane_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        multiway_aabb_cascade_combined_icp,
        module
    )?)?;
    Ok(())
}
