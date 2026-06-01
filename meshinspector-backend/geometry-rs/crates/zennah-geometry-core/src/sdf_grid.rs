use crate::grid::{grid_index, grid_value_count, sample_sdf_gradient, sample_sdf_value};
use crate::GeometryError;
use rayon::prelude::*;

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

fn validate_grid(values: &[f32], shape: [usize; 3], voxel_size: f64) -> Result<(), GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
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
