use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::convert::{read_i64_values, read_points};

mod cascade;
mod multiway;

pub(super) fn parse_icp_mode(mode: &str) -> PyResult<zennah_geometry_core::IcpMode> {
    match mode {
        "rigid" | "any_rigid" => Ok(zennah_geometry_core::IcpMode::AnyRigidXf),
        "translation" | "translation_only" => Ok(zennah_geometry_core::IcpMode::TranslationOnly),
        _ => Err(PyValueError::new_err(
            "mode must be 'rigid' or 'translation'",
        )),
    }
}

pub(super) fn matrix_list<const ROWS: usize, const COLS: usize>(
    py: Python<'_>,
    matrix: [[f64; COLS]; ROWS],
) -> PyResult<Py<PyList>> {
    let rows = PyList::empty(py);
    for row in matrix {
        rows.append(PyList::new(py, row)?)?;
    }
    Ok(rows.unbind())
}

fn registration_result_dict(
    py: Python<'_>,
    result: zennah_geometry_core::IcpRegistrationResult,
    method: &str,
    mode: &str,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("rotation", matrix_list(py, result.rotation)?)?;
    output.set_item("translation", result.translation)?;
    output.set_item("transform", matrix_list(py, result.transform)?)?;
    output.set_item("iterations", result.iterations)?;
    output.set_item("mean_square_distance", result.mean_square_distance)?;
    output.set_item("active_pair_count", result.active_pair_count)?;
    output.set_item("method", method)?;
    output.set_item("mode", mode)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    floating_points,
    reference_points,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid"
))]
fn pairwise_point_to_point_icp(
    py: Python<'_>,
    floating_points: PyReadonlyArray2<'_, f64>,
    reference_points: PyReadonlyArray2<'_, f64>,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
) -> PyResult<Py<PyDict>> {
    let rust_floating = read_points(floating_points)?;
    let rust_reference = read_points(reference_points)?;
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::pairwise_point_to_point_icp(
                &rust_floating,
                &rust_reference,
                max_iterations,
                tolerance,
                icp_mode,
            )
        })
        .map_err(PyValueError::new_err)?;
    registration_result_dict(py, result, "point_to_point", mode)
}

#[pyfunction(signature = (
    floating_points,
    reference_points,
    reference_normals,
    max_iterations = 20,
    tolerance = 1e-8,
    mode = "rigid",
    floating_normals = None,
    max_pair_distance = None,
    cos_threshold = None,
    far_dist_factor = None,
    mutual_closest = false
))]
fn pairwise_point_to_plane_icp(
    py: Python<'_>,
    floating_points: PyReadonlyArray2<'_, f64>,
    reference_points: PyReadonlyArray2<'_, f64>,
    reference_normals: PyReadonlyArray2<'_, f64>,
    max_iterations: usize,
    tolerance: f64,
    mode: &str,
    floating_normals: Option<PyReadonlyArray2<'_, f64>>,
    max_pair_distance: Option<f64>,
    cos_threshold: Option<f64>,
    far_dist_factor: Option<f64>,
    mutual_closest: bool,
) -> PyResult<Py<PyDict>> {
    let rust_floating = read_points(floating_points)?;
    let rust_reference = read_points(reference_points)?;
    let rust_reference_normals = read_points(reference_normals)?;
    let rust_floating_normals = match floating_normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let icp_mode = parse_icp_mode(mode)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::pairwise_point_to_plane_icp_with_filters(
                &rust_floating,
                &rust_reference,
                &rust_reference_normals,
                max_iterations,
                tolerance,
                icp_mode,
                zennah_geometry_core::IcpPairFilterOptions {
                    floating_normals: rust_floating_normals.as_deref(),
                    max_pair_distance,
                    cos_threshold,
                    far_dist_factor,
                    mutual_closest,
                },
            )
        })
        .map_err(PyValueError::new_err)?;
    registration_result_dict(py, result, "point_to_plane", mode)
}

#[pyfunction(signature = (points, voxel_size, max_voxels = 500000))]
fn point_cloud_grid_sample_indices(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    voxel_size: f64,
    max_voxels: usize,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let indices = py
        .detach(|| {
            zennah_geometry_core::point_cloud_grid_sample_indices(
                &rust_points,
                voxel_size,
                max_voxels,
            )
        })
        .map_err(PyValueError::new_err)?;
    let output = indices
        .into_iter()
        .map(|index| index as i64)
        .collect::<Vec<_>>();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction(signature = (
    points,
    distance,
    min_normal_dot = 0.0,
    lexicographical_order = true,
    normals = None
))]
fn point_cloud_uniform_sample_indices(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    distance: f64,
    min_normal_dot: f64,
    lexicographical_order: bool,
    normals: Option<PyReadonlyArray2<'_, f64>>,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let indices = py
        .detach(|| {
            zennah_geometry_core::point_cloud_uniform_sample_indices(
                &rust_points,
                distance,
                min_normal_dot,
                lexicographical_order,
                rust_normals.as_deref(),
            )
        })
        .map_err(PyValueError::new_err)?;
    let output = indices
        .into_iter()
        .map(|index| index as i64)
        .collect::<Vec<_>>();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn point_cloud_nearest_projections(
    py: Python<'_>,
    query_points: PyReadonlyArray2<'_, f64>,
    reference_points: PyReadonlyArray2<'_, f64>,
    up_dist_limit_sq: f64,
    lo_dist_limit_sq: f64,
    skip_same_index: bool,
) -> PyResult<Py<PyDict>> {
    let rust_query_points = read_points(query_points)?;
    let rust_reference_points = read_points(reference_points)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::point_cloud_nearest_projections(
                &rust_query_points,
                &rust_reference_points,
                up_dist_limit_sq,
                lo_dist_limit_sq,
                skip_same_index,
            )
        })
        .map_err(PyValueError::new_err)?;
    let points = result.points.into_iter().flatten().collect::<Vec<_>>();
    let output = PyDict::new(py);
    output.set_item("points", points.into_pyarray(py))?;
    output.set_item(
        "squared_distances",
        result.squared_distances.into_pyarray(py),
    )?;
    output.set_item("vertex_indices", result.vertex_indices.into_pyarray(py))?;
    Ok(output.unbind())
}

#[pyfunction(signature = (points, num_neighbors, up_dist_limit_sq = 1.7976931348623157e308))]
fn point_cloud_n_closest_neighbors(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    num_neighbors: usize,
    up_dist_limit_sq: f64,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rows = py
        .detach(|| {
            zennah_geometry_core::point_cloud_n_closest_neighbors(
                &rust_points,
                num_neighbors,
                up_dist_limit_sq,
            )
        })
        .map_err(PyValueError::new_err)?;
    let output = rows.into_iter().flatten().collect::<Vec<_>>();
    Ok(output.into_pyarray(py).unbind())
}

#[pyfunction]
fn point_cloud_two_closest_points(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
) -> PyResult<Py<PyDict>> {
    let rust_points = read_points(points)?;
    let result = py
        .detach(|| zennah_geometry_core::point_cloud_two_closest_points(&rust_points))
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item(
        "vertex_indices",
        result.vertex_indices.to_vec().into_pyarray(py),
    )?;
    output.set_item("squared_distance", result.squared_distance)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (points, center_index, radius, normals = None, untrusted_indices = None))]
fn point_cloud_neighbors_in_radius(
    py: Python<'_>,
    points: PyReadonlyArray2<'_, f64>,
    center_index: usize,
    radius: f64,
    normals: Option<PyReadonlyArray2<'_, f64>>,
    untrusted_indices: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyArray1<i64>>> {
    let rust_points = read_points(points)?;
    let rust_normals = match normals {
        Some(normals) => Some(read_points(normals)?),
        None => None,
    };
    let rust_untrusted_indices = match untrusted_indices {
        Some(indices) => read_i64_values(indices)
            .into_iter()
            .map(|index| {
                usize::try_from(index)
                    .map_err(|_| PyValueError::new_err("untrusted_indices must be non-negative"))
            })
            .collect::<PyResult<Vec<_>>>()?,
        None => Vec::new(),
    };
    let output = py
        .detach(|| {
            zennah_geometry_core::point_cloud_neighbors_in_radius(
                &rust_points,
                center_index,
                radius,
                rust_normals.as_deref(),
                &rust_untrusted_indices,
            )
        })
        .map_err(PyValueError::new_err)?;
    Ok(output.into_pyarray(py).unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(pairwise_point_to_point_icp, module)?)?;
    module.add_function(wrap_pyfunction!(pairwise_point_to_plane_icp, module)?)?;
    multiway::register(module)?;
    cascade::register(module)?;
    module.add_function(wrap_pyfunction!(point_cloud_grid_sample_indices, module)?)?;
    module.add_function(wrap_pyfunction!(
        point_cloud_uniform_sample_indices,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(point_cloud_nearest_projections, module)?)?;
    module.add_function(wrap_pyfunction!(point_cloud_n_closest_neighbors, module)?)?;
    module.add_function(wrap_pyfunction!(point_cloud_two_closest_points, module)?)?;
    module.add_function(wrap_pyfunction!(point_cloud_neighbors_in_radius, module)?)?;
    Ok(())
}
