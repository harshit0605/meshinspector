use crate::{GeometryError, VoxelActiveBoxResult};

pub fn voxel_active_box_values(
    values: &[f32],
    shape: [usize; 3],
    min_corner: [usize; 3],
    dimensions: [usize; 3],
) -> Result<VoxelActiveBoxResult, GeometryError> {
    validate_shape(shape)?;
    validate_shape(dimensions)?;
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
    for axis in 0..3 {
        let Some(max_excluded) = min_corner[axis].checked_add(dimensions[axis]) else {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "active_box",
                value: format!("min {min_corner:?}, dimensions {dimensions:?}"),
            });
        };
        if min_corner[axis] >= shape[axis] || max_excluded > shape[axis] {
            return Err(GeometryError::InvalidSelectionParameter {
                field: "active_box",
                value: format!("min {min_corner:?}, dimensions {dimensions:?}, shape {shape:?}"),
            });
        }
    }

    let sample_count = dimensions
        .iter()
        .try_fold(1_usize, |total, value| total.checked_mul(*value))
        .ok_or(GeometryError::GridTooLarge { shape: dimensions })?;
    let mut source_indices = Vec::with_capacity(sample_count);
    let mut coordinates = Vec::with_capacity(sample_count);
    let mut cropped_values = Vec::with_capacity(sample_count);

    for z_offset in 0..dimensions[2] {
        for y_offset in 0..dimensions[1] {
            for x_offset in 0..dimensions[0] {
                let coord = [
                    min_corner[0] + x_offset,
                    min_corner[1] + y_offset,
                    min_corner[2] + z_offset,
                ];
                let source_index = linear_index(coord, shape);
                source_indices.push(source_index);
                coordinates.push(coord);
                cropped_values.push(values[source_index]);
            }
        }
    }

    Ok(VoxelActiveBoxResult {
        min_corner,
        dimensions,
        source_indices,
        coordinates,
        values: cropped_values,
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
