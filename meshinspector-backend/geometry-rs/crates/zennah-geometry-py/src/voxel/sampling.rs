use numpy::PyReadonlyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{
    parse_voxel_axis, parse_voxel_path_metric, parse_voxel_path_plane, read_f32_values,
    read_index3, read_shape3,
};

fn voxel_path_payload(
    py: Python<'_>,
    result: zennah_geometry_core::VoxelPathResult,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("voxel_indices", result.voxel_indices)?;
    output.set_item(
        "coordinates",
        result
            .coordinates
            .into_iter()
            .map(|coord| coord.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("total_metric", result.total_metric)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    values,
    shape,
    start,
    finish,
    metric = "difference",
    max_dist_ratio = 1.5,
    plane = "none",
    quarters_mask = 15,
    exponent_modifier = -1.0
))]
#[allow(clippy::too_many_arguments)]
fn voxel_path_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    start: PyReadonlyArray1<'_, i64>,
    finish: PyReadonlyArray1<'_, i64>,
    metric: &str,
    max_dist_ratio: f32,
    plane: &str,
    quarters_mask: u8,
    exponent_modifier: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_start = read_index3("start", start)?;
    let rust_finish = read_index3("finish", finish)?;
    let rust_metric = parse_voxel_path_metric(metric)?;
    let rust_options = zennah_geometry_core::VoxelPathOptions {
        max_dist_ratio,
        plane: parse_voxel_path_plane(plane)?,
        quarters_mask,
        exponent_modifier,
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_path_values(
                &rust_values,
                rust_shape,
                rust_start,
                rust_finish,
                rust_metric,
                rust_options,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    voxel_path_payload(py, result)
}

#[pyfunction(signature = (
    values,
    shape,
    start,
    finish,
    metric = "exponent",
    max_dist_ratio = 1.5,
    plane = "none",
    exponent_modifier = -1.0
))]
#[allow(clippy::too_many_arguments)]
fn voxel_path_build_four_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    start: PyReadonlyArray1<'_, i64>,
    finish: PyReadonlyArray1<'_, i64>,
    metric: &str,
    max_dist_ratio: f32,
    plane: &str,
    exponent_modifier: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_start = read_index3("start", start)?;
    let rust_finish = read_index3("finish", finish)?;
    let rust_metric = parse_voxel_path_metric(metric)?;
    let rust_options = zennah_geometry_core::VoxelPathOptions {
        max_dist_ratio,
        plane: parse_voxel_path_plane(plane)?,
        quarters_mask: zennah_geometry_core::VoxelPathOptions::QUARTER_ALL,
        exponent_modifier,
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_path_build_four_values(
                &rust_values,
                rust_shape,
                rust_start,
                rust_finish,
                rust_metric,
                rust_options,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    let paths = result
        .paths
        .into_iter()
        .map(|entry| {
            let entry_payload = PyDict::new(py);
            entry_payload.set_item("quarters_mask", entry.quarters_mask)?;
            entry_payload.set_item("path", voxel_path_payload(py, entry.path)?)?;
            Ok(entry_payload.unbind())
        })
        .collect::<PyResult<Vec<_>>>()?;
    output.set_item("paths", paths)?;
    Ok(output.unbind())
}

#[pyfunction]
fn voxel_slice_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    plane: &str,
    slice_index: usize,
    min_value: f32,
    max_value: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_plane = parse_voxel_path_plane(plane)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_slice_values(
                &rust_values,
                rust_shape,
                rust_plane,
                slice_index,
                min_value,
                max_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("width", result.width)?;
    output.set_item("height", result.height)?;
    output.set_item("values", result.values)?;
    output.set_item("normalized_values", result.normalized_values)?;
    output.set_item(
        "coordinates",
        result
            .coordinates
            .into_iter()
            .map(|coord| coord.to_vec())
            .collect::<Vec<_>>(),
    )?;
    Ok(output.unbind())
}

#[pyfunction]
fn voxel_line_graph_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    axis: &str,
    fixed_coordinate: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_axis = parse_voxel_axis(axis)?;
    let rust_fixed_coordinate = read_index3("fixed_coordinate", fixed_coordinate)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_line_graph_values(
                &rust_values,
                rust_shape,
                rust_axis,
                rust_fixed_coordinate,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("axis", result.axis)?;
    output.set_item("positions", result.positions)?;
    output.set_item("voxel_indices", result.voxel_indices)?;
    output.set_item(
        "coordinates",
        result
            .coordinates
            .into_iter()
            .map(|coord| coord.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("values", result.values)?;
    Ok(output.unbind())
}

#[pyfunction]
fn voxel_active_box_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    min_corner: PyReadonlyArray1<'_, i64>,
    dimensions: PyReadonlyArray1<'_, i64>,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_min_corner = read_index3("min_corner", min_corner)?;
    let rust_dimensions = read_shape3(dimensions)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_active_box_values(
                &rust_values,
                rust_shape,
                rust_min_corner,
                rust_dimensions,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("min_corner", result.min_corner.to_vec())?;
    output.set_item("dimensions", result.dimensions.to_vec())?;
    output.set_item("source_indices", result.source_indices)?;
    output.set_item(
        "coordinates",
        result
            .coordinates
            .into_iter()
            .map(|coord| coord.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("values", result.values)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(voxel_path_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_path_build_four_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_slice_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_line_graph_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_active_box_values, module)?)?;
    Ok(())
}
