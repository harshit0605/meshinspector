use crate::{GeometryError, VoxelPathPlane, VoxelSliceResult};

pub fn voxel_slice_values(
    values: &[f32],
    shape: [usize; 3],
    plane: VoxelPathPlane,
    slice_index: usize,
    min_value: f32,
    max_value: f32,
) -> Result<VoxelSliceResult, GeometryError> {
    validate_shape(shape)?;
    let expected_values = shape
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape })?;
    if values.len() != expected_values {
        return Err(GeometryError::SdfValueCountDoesNotMatchShape {
            values: values.len(),
            shape,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if value.is_nan() {
            return Err(GeometryError::InvalidVoxelValue { index, value });
        }
    }
    let axis = match plane {
        VoxelPathPlane::YZ => 0,
        VoxelPathPlane::ZX => 1,
        VoxelPathPlane::XY => 2,
        VoxelPathPlane::None => {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "plane",
                value: "none".to_string(),
            });
        }
    };
    if slice_index >= shape[axis] {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "slice_index",
            value: format!("{slice_index} outside shape {shape:?}"),
        });
    }
    if !min_value.is_finite() || !max_value.is_finite() || max_value <= min_value {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "min_max",
            value: format!("{min_value}..{max_value}"),
        });
    }

    let width_axis = (axis + 1) % 3;
    let height_axis = (axis + 2) % 3;
    let width = shape[width_axis];
    let height = shape[height_axis];
    let mut raw_values = Vec::with_capacity(width * height);
    let mut normalized_values = Vec::with_capacity(width * height);
    let mut coordinates = Vec::with_capacity(width * height);
    let denominator = max_value - min_value;

    for pixel in 0..(width * height) {
        let mut coord = [0_usize; 3];
        coord[axis] = slice_index;
        coord[width_axis] = pixel % width;
        coord[height_axis] = pixel / width;
        let value = values[linear_index(coord, shape)];
        raw_values.push(value);
        normalized_values.push((value - min_value) / denominator);
        coordinates.push(coord);
    }

    Ok(VoxelSliceResult {
        width,
        height,
        values: raw_values,
        normalized_values,
        coordinates,
    })
}

fn validate_shape(shape: [usize; 3]) -> Result<(), GeometryError> {
    if shape.iter().any(|value| *value == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(())
}

fn linear_index(coord: [usize; 3], shape: [usize; 3]) -> usize {
    coord[0] + coord[1] * shape[0] + coord[2] * shape[0] * shape[1]
}
