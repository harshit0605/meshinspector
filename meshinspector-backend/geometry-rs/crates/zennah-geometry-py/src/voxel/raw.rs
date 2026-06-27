use numpy::{IntoPyArray, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{parse_raw_voxel_scalar_type, read_f32_values, read_shape3, read_vec3};

const DEFAULT_ISO_VALUE_SOURCE: &str =
    "MR::ObjectVoxels::histogram().getBinMinMax(bins.size() / 3).first";

fn raw_scalar_type_name(scalar_type: zennah_geometry_core::RawVoxelScalarType) -> &'static str {
    match scalar_type {
        zennah_geometry_core::RawVoxelScalarType::UInt8 => "uint8",
        zennah_geometry_core::RawVoxelScalarType::Int8 => "int8",
        zennah_geometry_core::RawVoxelScalarType::UInt16 => "uint16",
        zennah_geometry_core::RawVoxelScalarType::Int16 => "int16",
        zennah_geometry_core::RawVoxelScalarType::UInt32 => "uint32",
        zennah_geometry_core::RawVoxelScalarType::Int32 => "int32",
        zennah_geometry_core::RawVoxelScalarType::UInt64 => "uint64",
        zennah_geometry_core::RawVoxelScalarType::Int64 => "int64",
        zennah_geometry_core::RawVoxelScalarType::Float32 => "float32",
        zennah_geometry_core::RawVoxelScalarType::Float64 => "float64",
        zennah_geometry_core::RawVoxelScalarType::Float32_4 => "float32_4",
    }
}

fn raw_voxel_volume_payload(
    py: Python<'_>,
    volume: zennah_geometry_core::RawVoxelVolume,
) -> PyResult<Py<PyDict>> {
    let default_iso_value =
        zennah_geometry_core::voxel_default_iso_value_from_min_max(volume.min, volume.max)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("values", volume.values.into_pyarray(py))?;
    output.set_item("dimensions", volume.dimensions.to_vec())?;
    output.set_item("voxel_size", volume.voxel_size.to_vec())?;
    output.set_item("grid_level_set", volume.grid_level_set)?;
    output.set_item("scalar_type", raw_scalar_type_name(volume.scalar_type))?;
    output.set_item("min", volume.min)?;
    output.set_item("max", volume.max)?;
    output.set_item("source_path", volume.source_path)?;
    output.set_item("default_iso_value", default_iso_value)?;
    output.set_item("default_iso_value_source", DEFAULT_ISO_VALUE_SOURCE)?;
    Ok(output.unbind())
}

fn tiff_voxel_volume_payload(
    py: Python<'_>,
    volume: zennah_geometry_core::TiffVoxelVolume,
) -> PyResult<Py<PyDict>> {
    let default_iso_value =
        zennah_geometry_core::voxel_default_iso_value_from_min_max(volume.min, volume.max)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("values", volume.values.into_pyarray(py))?;
    output.set_item("dimensions", volume.dimensions.to_vec())?;
    output.set_item("voxel_size", volume.voxel_size.to_vec())?;
    output.set_item("grid_level_set", volume.grid_level_set)?;
    output.set_item("scalar_type", "tiff")?;
    output.set_item("min", volume.min)?;
    output.set_item("max", volume.max)?;
    output.set_item("source_path", volume.source_path)?;
    output.set_item("source_files", volume.source_files)?;
    output.set_item("default_iso_value", default_iso_value)?;
    output.set_item("default_iso_value_source", DEFAULT_ISO_VALUE_SOURCE)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (path, dimensions, voxel_size, scalar_type, grid_level_set = false))]
fn load_raw_voxels(
    py: Python<'_>,
    path: &str,
    dimensions: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    scalar_type: &str,
    grid_level_set: bool,
) -> PyResult<Py<PyDict>> {
    let dimensions = read_shape3(dimensions)?;
    let voxel_size = read_vec3("voxel_size", voxel_size)?;
    let scalar_type = parse_raw_voxel_scalar_type(scalar_type)?;
    let parameters = zennah_geometry_core::RawVoxelParameters {
        dimensions,
        voxel_size: [
            voxel_size[0] as f32,
            voxel_size[1] as f32,
            voxel_size[2] as f32,
        ],
        grid_level_set,
        scalar_type,
    };
    let volume = py
        .detach(|| zennah_geometry_core::load_raw_voxels(path, parameters))
        .map_err(PyValueError::new_err)?;
    raw_voxel_volume_payload(py, volume)
}

#[pyfunction]
fn load_raw_voxels_auto(py: Python<'_>, path: &str) -> PyResult<Py<PyDict>> {
    let volume = py
        .detach(|| zennah_geometry_core::load_raw_voxels_auto(path))
        .map_err(PyValueError::new_err)?;
    raw_voxel_volume_payload(py, volume)
}

#[pyfunction(signature = (directory, voxel_size, grid_level_set = false))]
fn load_tiff_voxels_dir(
    py: Python<'_>,
    directory: &str,
    voxel_size: PyReadonlyArray1<'_, f64>,
    grid_level_set: bool,
) -> PyResult<Py<PyDict>> {
    let voxel_size = read_vec3("voxel_size", voxel_size)?;
    let volume = py
        .detach(|| {
            zennah_geometry_core::load_tiff_voxels_dir(
                directory,
                [
                    voxel_size[0] as f32,
                    voxel_size[1] as f32,
                    voxel_size[2] as f32,
                ],
                grid_level_set,
            )
        })
        .map_err(PyValueError::new_err)?;
    tiff_voxel_volume_payload(py, volume)
}

#[pyfunction]
fn voxel_default_iso_value(py: Python<'_>, values: PyReadonlyArray1<'_, f32>) -> PyResult<f32> {
    let rust_values = read_f32_values(values);
    py.detach(|| zennah_geometry_core::voxel_default_iso_value(&rust_values))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(load_raw_voxels, module)?)?;
    module.add_function(wrap_pyfunction!(load_raw_voxels_auto, module)?)?;
    module.add_function(wrap_pyfunction!(load_tiff_voxels_dir, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_default_iso_value, module)?)?;
    Ok(())
}
