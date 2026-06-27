use crate::{GeometryError, VoxelLineGraphResult};

pub fn voxel_line_graph_values(
    values: &[f32],
    shape: [usize; 3],
    axis: usize,
    fixed_coordinate: [usize; 3],
) -> Result<VoxelLineGraphResult, GeometryError> {
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
    if axis >= 3 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "axis",
            value: axis.to_string(),
        });
    }
    validate_coord("fixed_coordinate", fixed_coordinate, shape)?;

    let sample_count = shape[axis];
    let mut positions = Vec::with_capacity(sample_count);
    let mut voxel_indices = Vec::with_capacity(sample_count);
    let mut coordinates = Vec::with_capacity(sample_count);
    let mut sampled_values = Vec::with_capacity(sample_count);

    for position in 0..sample_count {
        let mut coord = fixed_coordinate;
        coord[axis] = position;
        let voxel_index = linear_index(coord, shape);
        positions.push(position);
        voxel_indices.push(voxel_index);
        coordinates.push(coord);
        sampled_values.push(values[voxel_index]);
    }

    Ok(VoxelLineGraphResult {
        axis,
        positions,
        voxel_indices,
        coordinates,
        values: sampled_values,
    })
}

fn validate_shape(shape: [usize; 3]) -> Result<(), GeometryError> {
    if shape.iter().any(|value| *value == 0) {
        return Err(GeometryError::InvalidSdfShape { shape });
    }
    Ok(())
}

fn validate_coord(
    field: &'static str,
    coord: [usize; 3],
    shape: [usize; 3],
) -> Result<(), GeometryError> {
    if coord
        .iter()
        .zip(shape)
        .any(|(value, bound)| *value >= bound)
    {
        return Err(GeometryError::InvalidSelectionParameter {
            field,
            value: format!("{coord:?} outside shape {shape:?}"),
        });
    }
    Ok(())
}

fn linear_index(coord: [usize; 3], shape: [usize; 3]) -> usize {
    coord[0] + coord[1] * shape[0] + coord[2] * shape[0] * shape[1]
}
