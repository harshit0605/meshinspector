fn required_string(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    optional_string(dict, key)?
        .ok_or_else(|| PyValueError::new_err(format!("{key} is required")))
}

fn required_usize(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<usize> {
    optional_usize(dict, key)?
        .ok_or_else(|| PyValueError::new_err(format!("{key} is required")))
}

fn required_usize_pair(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<[usize; 2]> {
    let Some(value) = dict.get_item(key)? else {
        return Err(PyValueError::new_err(format!("{key} is required")));
    };
    let values = value
        .extract::<Vec<usize>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a two-item integer list")))?;
    if values.len() != 2 {
        return Err(PyValueError::new_err(format!(
            "{key} must be a two-item integer list"
        )));
    }
    Ok([values[0], values[1]])
}

fn required_usize_triple(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<[usize; 3]> {
    let Some(value) = dict.get_item(key)? else {
        return Err(PyValueError::new_err(format!("{key} is required")));
    };
    let values = value
        .extract::<Vec<usize>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a three-item integer list")))?;
    if values.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "{key} must be a three-item integer list"
        )));
    }
    Ok([values[0], values[1], values[2]])
}

fn required_f32_triple(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<[f32; 3]> {
    let Some(value) = dict.get_item(key)? else {
        return Err(PyValueError::new_err(format!("{key} is required")));
    };
    let values = value
        .extract::<Vec<f32>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a three-item number list")))?;
    if values.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "{key} must be a three-item number list"
        )));
    }
    Ok([values[0], values[1], values[2]])
}

fn required_f32_list(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Vec<f32>> {
    let Some(value) = dict.get_item(key)? else {
        return Err(PyValueError::new_err(format!("{key} is required")));
    };
    if value.is_none() {
        return Err(PyValueError::new_err(format!("{key} is required")));
    }
    value
        .extract::<Vec<f32>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a list of numbers")))
}

fn distance_map_stats_for_py(values: &[f32]) -> (usize, f32, f32) {
    let mut valid_count = 0usize;
    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;
    for value in values {
        if *value == zennah_geometry_core::DISTANCE_MAP_NOT_VALID_VALUE {
            continue;
        }
        valid_count += 1;
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    if valid_count == 0 {
        (0, 0.0, 0.0)
    } else {
        (valid_count, min_value, max_value)
    }
}

fn voxel_stats_for_py(values: &[f32]) -> (f32, f32) {
    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;
    for value in values {
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    if values.is_empty() {
        (0.0, 0.0)
    } else {
        (min_value, max_value)
    }
}
