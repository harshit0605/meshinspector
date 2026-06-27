use crate::grid::{grid_index, grid_value_count, sample_sdf_gradient, sample_sdf_value};
use crate::spatial::sdf_grid_values;
use crate::GeometryError;
use rayon::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct SdfGridSample {
    pub origin: [f64; 3],
    pub shape: [usize; 3],
    pub values: Vec<f32>,
}

pub fn sample_sdf_grid_in_bounds(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    voxel_size: f64,
    padding: f64,
    origin_phase: [f64; 3],
    winding_threshold: f64,
) -> Result<SdfGridSample, GeometryError> {
    let (origin, shape) =
        sdf_grid_origin_and_shape(bbox_min, bbox_max, voxel_size, padding, origin_phase)?;
    let values = sdf_grid_values(
        vertices,
        faces,
        origin,
        shape,
        voxel_size,
        winding_threshold,
    )?;
    Ok(SdfGridSample {
        origin,
        shape,
        values,
    })
}

pub fn combine_bounding_boxes(
    bbox_mins: &[[f64; 3]],
    bbox_maxs: &[[f64; 3]],
) -> Result<([f64; 3], [f64; 3]), GeometryError> {
    if bbox_mins.is_empty() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "bounding_boxes",
            value: "empty".to_string(),
        });
    }
    if bbox_mins.len() != bbox_maxs.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "bounding_boxes",
            value: format!("{} mins for {} maxs", bbox_mins.len(), bbox_maxs.len()),
        });
    }

    let mut combined_min = bbox_mins[0];
    let mut combined_max = bbox_maxs[0];
    for (bbox_min, bbox_max) in bbox_mins.iter().zip(bbox_maxs.iter()).skip(1) {
        for axis in 0..3 {
            combined_min[axis] = combined_min[axis].min(bbox_min[axis]);
            combined_max[axis] = combined_max[axis].max(bbox_max[axis]);
        }
    }
    Ok((combined_min, combined_max))
}

pub fn sdf_cell_values(values: &[f32], shape: [usize; 3]) -> Result<Vec<f32>, GeometryError> {
    validate_grid(values, shape, 1.0)?;
    if shape.iter().any(|dimension| *dimension < 2) {
        return Ok(Vec::new());
    }

    let cell_shape = [shape[0] - 1, shape[1] - 1, shape[2] - 1];
    let count = grid_value_count(cell_shape)?;
    let output = (0..count)
        .into_par_iter()
        .map(|index| {
            let i = index / (cell_shape[1] * cell_shape[2]);
            let remainder = index % (cell_shape[1] * cell_shape[2]);
            let j = remainder / cell_shape[2];
            let k = remainder % cell_shape[2];
            cell_average(values, shape, [i, j, k])
        })
        .collect();
    Ok(output)
}

pub fn sdf_occupancy(
    values: &[f32],
    shape: [usize; 3],
    iso_value: f32,
) -> Result<Vec<u8>, GeometryError> {
    Ok(sdf_cell_values(values, shape)?
        .into_par_iter()
        .map(|value| u8::from(value <= iso_value))
        .collect())
}

pub fn estimate_sdf_volume(
    values: &[f32],
    shape: [usize; 3],
    voxel_size: f64,
    iso_value: f32,
) -> Result<f64, GeometryError> {
    validate_grid(values, shape, voxel_size)?;
    if shape.iter().any(|dimension| *dimension < 2) {
        return Ok(0.0);
    }

    let cell_shape = [shape[0] - 1, shape[1] - 1, shape[2] - 1];
    let count = grid_value_count(cell_shape)?;
    let inside = (0..count)
        .into_par_iter()
        .filter(|index| {
            let i = index / (cell_shape[1] * cell_shape[2]);
            let remainder = index % (cell_shape[1] * cell_shape[2]);
            let j = remainder / cell_shape[2];
            let k = remainder % cell_shape[2];
            cell_average(values, shape, [i, j, k]) <= iso_value
        })
        .count();
    Ok(inside as f64 * voxel_size.powi(3))
}

pub fn sdf_grid_points(
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    validate_grid_shape(shape, voxel_size)?;
    let count = grid_value_count(shape)?;
    Ok((0..count)
        .into_par_iter()
        .map(|index| {
            let i = index / (shape[1] * shape[2]);
            let remainder = index % (shape[1] * shape[2]);
            let j = remainder / shape[2];
            let k = remainder % shape[2];
            [
                origin[0] + i as f64 * voxel_size,
                origin[1] + j as f64 * voxel_size,
                origin[2] + k as f64 * voxel_size,
            ]
        })
        .collect())
}

pub fn sdf_points_to_grid(
    origin: [f64; 3],
    voxel_size: f64,
    points: &[[f64; 3]],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    validate_voxel_size(voxel_size)?;
    Ok(points
        .par_iter()
        .map(|point| {
            [
                (point[0] - origin[0]) / voxel_size,
                (point[1] - origin[1]) / voxel_size,
                (point[2] - origin[2]) / voxel_size,
            ]
        })
        .collect())
}

pub fn sample_sdf_values_batch(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    points: &[[f64; 3]],
) -> Result<Vec<f32>, GeometryError> {
    validate_grid(values, shape, voxel_size)?;
    if shape.iter().any(|dimension| *dimension < 2) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(points
        .par_iter()
        .map(|point| sample_sdf_value(values, origin, shape, voxel_size, *point))
        .collect())
}

pub fn sample_sdf_gradients_batch(
    values: &[f32],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    points: &[[f64; 3]],
) -> Result<Vec<[f32; 3]>, GeometryError> {
    validate_grid(values, shape, voxel_size)?;
    if shape.iter().any(|dimension| *dimension < 2) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(points
        .par_iter()
        .map(|point| sample_sdf_gradient(values, origin, shape, voxel_size, *point))
        .collect())
}

pub(crate) fn sdf_grid_origin_and_shape(
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    voxel_size: f64,
    padding: f64,
    origin_phase: [f64; 3],
) -> Result<([f64; 3], [usize; 3]), GeometryError> {
    validate_voxel_size(voxel_size)?;
    let mut origin = [0.0_f64; 3];
    let mut shape = [0_usize; 3];
    for axis in 0..3 {
        origin[axis] = bbox_min[axis] - padding + origin_phase[axis] * voxel_size;
        let maximum = bbox_max[axis] + padding + origin_phase[axis] * voxel_size;
        let dimension = ((maximum - origin[axis]) / voxel_size).ceil() as usize + 1;
        shape[axis] = dimension.max(2);
    }
    Ok((origin, shape))
}

fn validate_voxel_size(voxel_size: f64) -> Result<(), GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    Ok(())
}

fn validate_grid_shape(shape: [usize; 3], voxel_size: f64) -> Result<(), GeometryError> {
    validate_voxel_size(voxel_size)?;
    if shape.iter().any(|dimension| *dimension == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    grid_value_count(shape)?;
    Ok(())
}

fn validate_grid(values: &[f32], shape: [usize; 3], voxel_size: f64) -> Result<(), GeometryError> {
    validate_grid_shape(shape, voxel_size)?;
    let expected_values = grid_value_count(shape)?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    Ok(())
}

fn cell_average(values: &[f32], shape: [usize; 3], base: [usize; 3]) -> f32 {
    let mut total = 0.0_f32;
    for dx in 0..=1 {
        for dy in 0..=1 {
            for dz in 0..=1 {
                total += values[grid_index([base[0] + dx, base[1] + dy, base[2] + dz], shape)];
            }
        }
    }
    total / 8.0
}
