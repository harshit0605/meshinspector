use numpy::PyReadonlyArray2;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub(crate) fn read_distance_map(
    values: PyReadonlyArray2<'_, f32>,
    origin: (f64, f64),
    pixel_size: (f64, f64),
    model_transform: Option<Vec<f64>>,
) -> PyResult<zennah_geometry_core::DistanceMapGrid> {
    let array = values.as_array();
    let shape = array.shape();
    if shape.len() != 2 {
        return Err(PyValueError::new_err(
            "values must have shape (height, width)",
        ));
    }
    let height = shape[0];
    let width = shape[1];
    let map_values = (0..height)
        .flat_map(|row| (0..width).map(move |column| array[[row, column]]))
        .collect::<Vec<_>>();
    let (valid_count, min_value, max_value) = distance_map_stats(&map_values);
    Ok(zennah_geometry_core::DistanceMapGrid {
        width,
        height,
        origin: [origin.0, origin.1],
        pixel_size: [pixel_size.0, pixel_size.1],
        model_transform: read_model_transform(model_transform)?,
        valid_count,
        values: map_values,
        min_value,
        max_value,
    })
}

fn read_model_transform(model_transform: Option<Vec<f64>>) -> PyResult<Option<[f64; 16]>> {
    let Some(values) = model_transform else {
        return Ok(None);
    };
    if values.len() != 16 {
        return Err(PyValueError::new_err(
            "model_transform must contain 16 row-major values",
        ));
    }
    let mut transform = [0.0; 16];
    for (index, value) in values.into_iter().enumerate() {
        if !value.is_finite() {
            return Err(PyValueError::new_err(
                "model_transform values must be finite",
            ));
        }
        transform[index] = value;
    }
    Ok(Some(transform))
}

fn distance_map_stats(values: &[f32]) -> (usize, f32, f32) {
    let mut valid_count = 0;
    let mut min_value = f32::MAX;
    let mut max_value = -f32::MAX;
    for value in values {
        if *value == zennah_geometry_core::DISTANCE_MAP_NOT_VALID_VALUE {
            continue;
        }
        valid_count += 1;
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    (valid_count, min_value, max_value)
}
