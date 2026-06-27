use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use super::{matrix_list, parse_icp_mode};
use crate::convert::{read_i64_values, read_points};

fn icp_object_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::MultiwayIcpObjectResult,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("rotation", matrix_list(py, result.rotation)?)?;
    output.set_item("translation", result.translation)?;
    output.set_item("transform", matrix_list(py, result.transform)?)?;
    Ok(output.unbind())
}

pub(super) fn multiway_registration_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::MultiwayIcpRegistrationResult,
    method: &str,
    mode: &str,
) -> PyResult<Py<PyDict>> {
    let transforms = PyList::empty(py);
    for transform in result.transforms {
        transforms.append(icp_object_result_dict(py, transform)?)?;
    }
    let output = PyDict::new(py);
    output.set_item("transforms", transforms)?;
    output.set_item("iterations", result.iterations)?;
    output.set_item("mean_square_distance", result.mean_square_distance)?;
    output.set_item("active_pair_count", result.active_pair_count)?;
    output.set_item("fixed_object_index", result.fixed_object_index)?;
    output.set_item("method", method)?;
    output.set_item("mode", mode)?;
    Ok(output.unbind())
}

fn read_multiway_counts(
    object_point_counts: PyReadonlyArray1<'_, i64>,
    row_count: usize,
) -> PyResult<Vec<usize>> {
    let counts = read_i64_values(object_point_counts);
    if counts.is_empty() {
        return Err(PyValueError::new_err(
            "object_point_counts must contain at least one point cloud count",
        ));
    }
    let mut total = 0usize;
    let mut rust_counts = Vec::with_capacity(counts.len());
    for count in counts {
        let count = usize::try_from(count).map_err(|_| {
            PyValueError::new_err("object_point_counts values must be non-negative")
        })?;
        if count == 0 {
            return Err(PyValueError::new_err(
                "object_point_counts values must be positive",
            ));
        }
        total = total
            .checked_add(count)
            .ok_or_else(|| PyValueError::new_err("object_point_counts overflowed"))?;
        rust_counts.push(count);
    }
    if total != row_count {
        return Err(PyValueError::new_err(
            "object_point_counts total must match points row count",
        ));
    }
    Ok(rust_counts)
}

fn split_multiway_rows(rows: Vec<[f64; 3]>, counts: &[usize]) -> Vec<Vec<[f64; 3]>> {
    let mut offset = 0usize;
    let mut objects = Vec::with_capacity(counts.len());
    for count in counts {
        objects.push(rows[offset..offset + count].to_vec());
        offset += count;
    }
    objects
}

pub(super) fn read_multiway_objects(
    points: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<Vec<[f64; 3]>>> {
    let rust_points = read_points(points)?;
    let counts = read_multiway_counts(object_point_counts, rust_points.len())?;
    Ok(split_multiway_rows(rust_points, &counts))
}

pub(super) fn read_multiway_objects_and_normals(
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
) -> PyResult<(Vec<Vec<[f64; 3]>>, Vec<Vec<[f64; 3]>>)> {
    let rust_points = read_points(points)?;
    let rust_normals = read_points(normals)?;
    if rust_normals.len() != rust_points.len() {
        return Err(PyValueError::new_err(
            "normals row count must match points row count",
        ));
    }
    let counts = read_multiway_counts(object_point_counts, rust_points.len())?;
    Ok((
        split_multiway_rows(rust_points, &counts),
        split_multiway_rows(rust_normals, &counts),
    ))
}

#[pyfunction(signature = (
    points,
    object_point_counts,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_point_to_point_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let rust_objects = read_multiway_objects(points, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_point_to_point_icp(
                &rust_objects,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_point", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_point_to_plane_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
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
            zennah_geometry_core::multiway_point_to_plane_icp(
                &rust_objects,
                &rust_object_normals,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_plane", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_combined_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
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
            zennah_geometry_core::multiway_combined_icp(
                &rust_objects,
                &rust_object_normals,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "combined", mode)
}

#[pyfunction(signature = (
    points,
    object_point_counts,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_all_object_point_to_point_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    fixed_object_index: Option<usize>,
) -> PyResult<Py<PyDict>> {
    let rust_objects = read_multiway_objects(points, object_point_counts)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::multiway_all_object_point_to_point_icp(
                &rust_objects,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_point_all_object", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_all_object_point_to_plane_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
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
            zennah_geometry_core::multiway_all_object_point_to_plane_icp(
                &rust_objects,
                &rust_object_normals,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "point_to_plane_all_object", mode)
}

#[pyfunction(signature = (
    points,
    normals,
    object_point_counts,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    fixed_object_index = None
))]
fn multiway_all_object_combined_icp(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    normals: PyReadonlyArray2<'_, f64>,
    object_point_counts: PyReadonlyArray1<'_, i64>,
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
            zennah_geometry_core::multiway_all_object_combined_icp(
                &rust_objects,
                &rust_object_normals,
                max_iterations,
                tolerance,
                icp_mode,
                fixed_object_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    multiway_registration_result_dict(py, result, "combined_all_object", mode)
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(multiway_point_to_point_icp, module)?)?;
    module.add_function(wrap_pyfunction!(multiway_point_to_plane_icp, module)?)?;
    module.add_function(wrap_pyfunction!(multiway_combined_icp, module)?)?;
    module.add_function(wrap_pyfunction!(
        multiway_all_object_point_to_point_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        multiway_all_object_point_to_plane_icp,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(multiway_all_object_combined_icp, module)?)?;
    Ok(())
}
