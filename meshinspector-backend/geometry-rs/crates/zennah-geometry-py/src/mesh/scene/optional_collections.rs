fn optional_string_list(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<Vec<String>>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<Vec<String>>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be a string list")))
}

fn optional_vec3_list(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<Vec<[f64; 3]>>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    let rows = value
        .extract::<Vec<Vec<f64>>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a list of three-item number lists")))?;
    let mut points = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.len() != 3 {
            return Err(PyValueError::new_err(format!(
                "{key}[{index}] must be a three-item number list"
            )));
        }
        points.push([row[0], row[1], row[2]]);
    }
    Ok(Some(points))
}

fn optional_usize_pair_list(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<Vec<[usize; 2]>>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    let rows = value
        .extract::<Vec<Vec<usize>>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a list of two-item integer lists")))?;
    let mut pairs = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.len() != 2 {
            return Err(PyValueError::new_err(format!(
                "{key}[{index}] must be a two-item integer list"
            )));
        }
        pairs.push([row[0], row[1]]);
    }
    Ok(Some(pairs))
}

fn optional_rgba_rows(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<Vec<[u8; 4]>>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    let rows = value
        .extract::<Vec<Vec<i64>>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a list of four-channel color rows")))?;
    let mut colors = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.len() != 4 {
            return Err(PyValueError::new_err(format!(
                "{key}[{index}] must have 4 channels"
            )));
        }
        colors.push([
            clamp_u8(row[0]),
            clamp_u8(row[1]),
            clamp_u8(row[2]),
            clamp_u8(row[3]),
        ]);
    }
    Ok(Some(colors))
}

fn clamp_u8(value: i64) -> u8 {
    value.clamp(0, 255) as u8
}

fn optional_usize(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<usize>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<usize>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be an unsigned integer")))
}

fn optional_usize_triple(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<[usize; 3]>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    let values = value
        .extract::<Vec<usize>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a three-item integer list")))?;
    if values.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "{key} must be a three-item integer list"
        )));
    }
    Ok(Some([values[0], values[1], values[2]]))
}

fn optional_usize_list(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<Vec<usize>>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<Vec<usize>>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be an unsigned integer list")))
}

fn optional_f32(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<f32>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<f32>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be a number")))
}

fn optional_vec3(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<[f64; 3]>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    let values = value
        .extract::<Vec<f64>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a three-item number list")))?;
    if values.len() != 3 {
        return Err(PyValueError::new_err(format!(
            "{key} must be a three-item number list"
        )));
    }
    Ok(Some([values[0], values[1], values[2]]))
}

fn optional_vec4(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<[f64; 4]>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    let values = value
        .extract::<Vec<f64>>()
        .map_err(|_| PyValueError::new_err(format!("{key} must be a four-item number list")))?;
    if values.len() != 4 {
        return Err(PyValueError::new_err(format!(
            "{key} must be a four-item number list"
        )));
    }
    Ok(Some([values[0], values[1], values[2], values[3]]))
}

fn optional_u32_map(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<HashMap<String, u32>>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<HashMap<String, u32>>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be a string to unsigned integer map")))
}
