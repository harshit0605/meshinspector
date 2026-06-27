use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_f32_values, read_index3_rows, read_shape3, read_vec3};

fn voxel_segmentation_payload(
    py: Python<'_>,
    result: zennah_geometry_core::VoxelSegmentationResult,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("min_corner", result.min_corner.to_vec())?;
    output.set_item("dimensions", result.dimensions.to_vec())?;
    output.set_item("source_indices", result.source_indices)?;
    output.set_item("part_indices", result.part_indices)?;
    output.set_item(
        "selected_coordinates",
        result
            .selected_coordinates
            .into_iter()
            .map(|coord| coord.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("selected_values", result.selected_values)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    values,
    shape,
    inside_seeds,
    outside_seeds,
    exponent_modifier = 3000.0,
    voxels_expansion = 25,
    include_boundary_outside = true
))]
#[allow(clippy::too_many_arguments)]
fn voxel_segmentation_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    inside_seeds: PyReadonlyArray2<'_, i64>,
    outside_seeds: PyReadonlyArray2<'_, i64>,
    exponent_modifier: f32,
    voxels_expansion: usize,
    include_boundary_outside: bool,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_inside_seeds = read_index3_rows("inside_seeds", inside_seeds)?;
    let rust_outside_seeds = read_index3_rows("outside_seeds", outside_seeds)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_segmentation_values(
                &rust_values,
                rust_shape,
                &rust_inside_seeds,
                &rust_outside_seeds,
                zennah_geometry_core::VoxelSegmentationOptions {
                    exponent_modifier,
                    voxels_expansion,
                    include_boundary_outside,
                },
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    voxel_segmentation_payload(py, result)
}

#[pyfunction(signature = (
    values,
    shape,
    inside_seeds,
    outside_seeds,
    voxel_size,
    exponent_modifier = 3000.0,
    voxels_expansion = 25,
    include_boundary_outside = true
))]
#[allow(clippy::too_many_arguments)]
fn voxel_segmentation_mesh_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    inside_seeds: PyReadonlyArray2<'_, i64>,
    outside_seeds: PyReadonlyArray2<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    exponent_modifier: f32,
    voxels_expansion: usize,
    include_boundary_outside: bool,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_inside_seeds = read_index3_rows("inside_seeds", inside_seeds)?;
    let rust_outside_seeds = read_index3_rows("outside_seeds", outside_seeds)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_segmentation_mesh_values(
                &rust_values,
                rust_shape,
                &rust_inside_seeds,
                &rust_outside_seeds,
                zennah_geometry_core::VoxelSegmentationOptions {
                    exponent_modifier,
                    voxels_expansion,
                    include_boundary_outside,
                },
                rust_voxel_size,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item(
        "segmentation",
        voxel_segmentation_payload(py, result.segmentation)?,
    )?;
    output.set_item("vertices", result.vertices)?;
    output.set_item("faces", result.faces)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    values,
    shape,
    mask_coordinates,
    voxel_size,
    mask_expansion = 25,
    smooth_band_radius = 3
))]
fn voxel_mask_to_mesh_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    mask_coordinates: PyReadonlyArray2<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    mask_expansion: usize,
    smooth_band_radius: usize,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_mask_coordinates = read_index3_rows("mask_coordinates", mask_coordinates)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_mask_to_mesh_values(
                &rust_values,
                rust_shape,
                &rust_mask_coordinates,
                rust_voxel_size,
                mask_expansion,
                smooth_band_radius,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("min_corner", result.min_corner.to_vec())?;
    output.set_item("dimensions", result.dimensions.to_vec())?;
    output.set_item("source_indices", result.source_indices)?;
    output.set_item("part_indices", result.part_indices)?;
    output.set_item(
        "selected_coordinates",
        result
            .selected_coordinates
            .into_iter()
            .map(|coord| coord.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("vertices", result.vertices)?;
    output.set_item("faces", result.faces)?;
    Ok(output.unbind())
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(voxel_segmentation_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_segmentation_mesh_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_mask_to_mesh_values, module)?)?;
    Ok(())
}
