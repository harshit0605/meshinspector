use crate::{GeometryError, SdfBooleanOperation, VoxelBinaryOperation};
use rayon::prelude::*;

pub const VOXEL_HISTOGRAM_BINS_NUMBER: usize = 256;

pub fn sdf_boolean_values(
    left: &[f32],
    right: &[f32],
    operation: SdfBooleanOperation,
) -> Result<Vec<f32>, GeometryError> {
    let operation = match operation {
        SdfBooleanOperation::Union => VoxelBinaryOperation::Union,
        SdfBooleanOperation::Intersection => VoxelBinaryOperation::Intersection,
        SdfBooleanOperation::Difference => VoxelBinaryOperation::Difference,
    };
    voxel_binary_values(left, right, operation)
}

pub fn voxel_binary_values(
    left: &[f32],
    right: &[f32],
    operation: VoxelBinaryOperation,
) -> Result<Vec<f32>, GeometryError> {
    if left.len() != right.len() {
        return Err(GeometryError::MismatchedSdfValueCount {
            left: left.len(),
            right: right.len(),
        });
    }

    let output = left
        .par_iter()
        .zip(right.par_iter())
        .map(|(left_value, right_value)| match operation {
            VoxelBinaryOperation::Union => (*left_value).min(*right_value),
            VoxelBinaryOperation::Intersection => (*left_value).max(*right_value),
            VoxelBinaryOperation::Difference => (*left_value).max(-*right_value),
            VoxelBinaryOperation::Max => (*left_value).max(*right_value),
            VoxelBinaryOperation::Min => (*left_value).min(*right_value),
            VoxelBinaryOperation::Sum => *left_value + *right_value,
            VoxelBinaryOperation::Multiply => *left_value * *right_value,
            VoxelBinaryOperation::Divide => *left_value / *right_value,
        })
        .collect();
    Ok(output)
}

pub fn voxel_default_iso_value(values: &[f32]) -> Result<f32, GeometryError> {
    let (min_value, max_value) = voxel_value_range(values)?;

    voxel_default_iso_value_from_min_max(min_value, max_value)
}

pub fn voxel_value_range(values: &[f32]) -> Result<(f32, f32), GeometryError> {
    if values.is_empty() {
        return Err(GeometryError::EmptyVoxelValues);
    }

    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;
    for (index, value) in values.iter().copied().enumerate() {
        if value.is_nan() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }
    Ok((min_value, max_value))
}

pub fn voxel_default_iso_value_from_min_max(
    min_value: f32,
    max_value: f32,
) -> Result<f32, GeometryError> {
    if min_value.is_nan() {
        return Err(GeometryError::InvalidVoxelValue {
            index: 0,
            value: min_value,
        });
    }
    if max_value.is_nan() {
        return Err(GeometryError::InvalidVoxelValue {
            index: 1,
            value: max_value,
        });
    }

    let bin_size = (max_value - min_value) / VOXEL_HISTOGRAM_BINS_NUMBER as f32;
    let default_bin = VOXEL_HISTOGRAM_BINS_NUMBER / 3;
    Ok(min_value + default_bin as f32 * bin_size)
}

pub fn voxel_binary_iso_value(
    left_iso: f32,
    right_iso: f32,
    operation: VoxelBinaryOperation,
) -> f32 {
    match operation {
        VoxelBinaryOperation::Union
        | VoxelBinaryOperation::Intersection
        | VoxelBinaryOperation::Difference => left_iso,
        VoxelBinaryOperation::Max => left_iso.max(right_iso),
        VoxelBinaryOperation::Min => left_iso.min(right_iso),
        VoxelBinaryOperation::Sum => left_iso + right_iso,
        VoxelBinaryOperation::Multiply => left_iso * right_iso,
        VoxelBinaryOperation::Divide => {
            if right_iso != 0.0 {
                left_iso / right_iso
            } else {
                left_iso
            }
        }
    }
}

pub fn sdf_offset_values(values: &[f32], offset_mm: f64) -> Result<Vec<f32>, GeometryError> {
    if !offset_mm.is_finite() {
        return Err(GeometryError::InvalidSdfOffset { offset_mm });
    }
    Ok(values
        .par_iter()
        .map(|value| (*value as f64 - offset_mm) as f32)
        .collect())
}

pub fn sdf_shell_values(values: &[f32], wall_thickness_mm: f64) -> Result<Vec<f32>, GeometryError> {
    if !wall_thickness_mm.is_finite() || wall_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidWallThickness { wall_thickness_mm });
    }
    Ok(values
        .par_iter()
        .map(|value| {
            let inner_void = *value as f64 + wall_thickness_mm;
            (*value as f64).max(-inner_void) as f32
        })
        .collect())
}
