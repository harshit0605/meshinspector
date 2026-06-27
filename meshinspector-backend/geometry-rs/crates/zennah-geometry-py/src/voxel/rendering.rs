use numpy::PyReadonlyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::convert::{read_f32_values, read_index3, read_shape3, read_vec3};

#[pyfunction]
fn voxel_volume_render_data_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    active_min_corner: PyReadonlyArray1<'_, i64>,
    active_dimensions: PyReadonlyArray1<'_, i64>,
    source_min_value: f32,
    source_max_value: f32,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let rust_active_min_corner = read_index3("active_min_corner", active_min_corner)?;
    let rust_active_dimensions = read_shape3(active_dimensions)?;
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_volume_render_data_values(
                &rust_values,
                rust_shape,
                rust_voxel_size,
                rust_active_min_corner,
                rust_active_dimensions,
                source_min_value,
                source_max_value,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("dimensions", result.dimensions.to_vec())?;
    output.set_item("voxel_size", result.voxel_size.to_vec())?;
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
    output.set_item("min_value", result.min_value)?;
    output.set_item("max_value", result.max_value)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (lut_type, alpha_type = "constant", alpha_limit = 10, one_color = None))]
fn voxel_volume_render_lut_values(
    py: Python<'_>,
    lut_type: &str,
    alpha_type: &str,
    alpha_limit: u8,
    one_color: Option<PyReadonlyArray1<'_, i64>>,
) -> PyResult<Py<PyDict>> {
    let rust_one_color = match one_color {
        Some(values) => Some(read_color4("one_color", values)?),
        None => None,
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_volume_render_lut_values(
                lut_type,
                alpha_type,
                alpha_limit,
                rust_one_color,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("lut_type", result.lut_type)?;
    output.set_item("alpha_type", result.alpha_type)?;
    output.set_item("alpha_limit", result.alpha_limit)?;
    output.set_item(
        "colors_rgba",
        result
            .colors_rgba
            .into_iter()
            .map(|color| color.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("meshlib_reference", result.meshlib_reference)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    values,
    shape,
    voxel_size,
    min_corner,
    ray_start,
    ray_direction,
    sampling_step,
    min_value,
    max_value,
    lut_type,
    alpha_type = "constant",
    alpha_limit = 10,
    one_color = None,
    clipping_plane = None,
    shading_mode = "none",
    light_pos_eye = None,
    ambient_strength = 0.1,
    specular_strength = 0.5,
    spec_exp = 35.0,
    active_indices = None,
    max_steps = 4096
))]
fn voxel_volume_render_ray_values(
    py: Python<'_>,
    values: PyReadonlyArray1<'_, f32>,
    shape: PyReadonlyArray1<'_, i64>,
    voxel_size: PyReadonlyArray1<'_, f64>,
    min_corner: PyReadonlyArray1<'_, i64>,
    ray_start: PyReadonlyArray1<'_, f64>,
    ray_direction: PyReadonlyArray1<'_, f64>,
    sampling_step: f64,
    min_value: f32,
    max_value: f32,
    lut_type: &str,
    alpha_type: &str,
    alpha_limit: u8,
    one_color: Option<PyReadonlyArray1<'_, i64>>,
    clipping_plane: Option<PyReadonlyArray1<'_, f64>>,
    shading_mode: &str,
    light_pos_eye: Option<PyReadonlyArray1<'_, f64>>,
    ambient_strength: f32,
    specular_strength: f32,
    spec_exp: f32,
    active_indices: Option<PyReadonlyArray1<'_, i64>>,
    max_steps: usize,
) -> PyResult<Py<PyDict>> {
    let rust_values = read_f32_values(values);
    let rust_shape = read_shape3(shape)?;
    let rust_voxel_size = read_vec3("voxel_size", voxel_size)?;
    let rust_min_corner = read_index3("min_corner", min_corner)?;
    let rust_ray_start = read_vec3("ray_start", ray_start)?;
    let rust_ray_direction = read_vec3("ray_direction", ray_direction)?;
    let rust_one_color = match one_color {
        Some(values) => Some(read_color4("one_color", values)?),
        None => None,
    };
    let rust_clipping_plane = match clipping_plane {
        Some(values) => Some(read_plane4("clipping_plane", values)?),
        None => None,
    };
    let rust_light_pos_eye = match light_pos_eye {
        Some(values) => Some(read_vec3("light_pos_eye", values)?),
        None => None,
    };
    let rust_active_indices = match active_indices {
        Some(values) => Some(read_index_values("active_indices", values)?),
        None => None,
    };
    let result = py
        .detach(|| {
            zennah_geometry_core::voxel_volume_render_ray_values(
                &rust_values,
                rust_shape,
                rust_voxel_size,
                rust_min_corner,
                rust_ray_start,
                rust_ray_direction,
                sampling_step,
                min_value,
                max_value,
                lut_type,
                alpha_type,
                alpha_limit,
                rust_one_color,
                rust_clipping_plane,
                shading_mode,
                rust_light_pos_eye,
                ambient_strength,
                specular_strength,
                spec_exp,
                rust_active_indices.as_deref(),
                max_steps,
            )
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let output = PyDict::new(py);
    output.set_item("color_rgba", result.color_rgba.to_vec())?;
    output.set_item(
        "first_opaque_world",
        result.first_opaque_world.map(|pos| pos.to_vec()),
    )?;
    output.set_item("visited_indices", result.visited_indices)?;
    output.set_item("accepted_indices", result.accepted_indices)?;
    output.set_item("meshlib_reference", result.meshlib_reference)?;
    Ok(output.unbind())
}

fn read_color4(name: &'static str, values: PyReadonlyArray1<'_, i64>) -> PyResult<[u8; 4]> {
    let values = values.as_array().to_vec();
    if values.len() != 4 {
        return Err(PyValueError::new_err(format!(
            "{name} must contain four values"
        )));
    }
    let mut color = [0_u8; 4];
    for (index, value) in values.into_iter().enumerate() {
        color[index] = u8::try_from(value).map_err(|_| {
            PyValueError::new_err(format!("{name}[{index}] must be between 0 and 255"))
        })?;
    }
    Ok(color)
}

fn read_plane4(name: &'static str, values: PyReadonlyArray1<'_, f64>) -> PyResult<[f64; 4]> {
    let values = values.as_array().to_vec();
    if values.len() != 4 {
        return Err(PyValueError::new_err(format!(
            "{name} must contain four values"
        )));
    }
    let mut plane = [0.0_f64; 4];
    for (index, value) in values.into_iter().enumerate() {
        if !value.is_finite() {
            return Err(PyValueError::new_err(format!(
                "{name}[{index}] must be finite"
            )));
        }
        plane[index] = value;
    }
    Ok(plane)
}

fn read_index_values(
    name: &'static str,
    values: PyReadonlyArray1<'_, i64>,
) -> PyResult<Vec<usize>> {
    values
        .as_array()
        .iter()
        .enumerate()
        .map(|(index, value)| {
            usize::try_from(*value)
                .map_err(|_| PyValueError::new_err(format!("{name}[{index}] must be non-negative")))
        })
        .collect()
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(voxel_volume_render_data_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_volume_render_lut_values, module)?)?;
    module.add_function(wrap_pyfunction!(voxel_volume_render_ray_values, module)?)?;
    Ok(())
}
