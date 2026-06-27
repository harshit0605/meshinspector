fn extract_vector3(value: &Bound<'_, PyAny>, name: &str) -> PyResult<[f64; 3]> {
    let values = value
        .extract::<Vec<f64>>()
        .map_err(|_| PyValueError::new_err(format!("{name} must be a 3D vector")))?;
    vector3_from_values(&values, name)
}

fn vector3_from_values(values: &[f64], name: &str) -> PyResult<[f64; 3]> {
    if values.len() != 3 {
        return Err(PyValueError::new_err(format!("{name} must have length 3")));
    }
    Ok([values[0], values[1], values[2]])
}

fn optional_vector2_from_values(values: Option<&[f64]>, name: &str) -> PyResult<Option<[f64; 2]>> {
    let Some(values) = values else {
        return Ok(None);
    };
    if values.len() != 2 {
        return Err(PyValueError::new_err(format!("{name} must have length 2")));
    }
    Ok(Some([values[0], values[1]]))
}

fn axis_letter(axis: usize) -> char {
    match axis {
        0 => 'A',
        1 => 'B',
        2 => 'C',
        _ => unreachable!("axis ids are sanitized before serialization"),
    }
}

fn axis_json_name(axis: usize) -> &'static str {
    match axis {
        0 => "Axis A",
        1 => "Axis B",
        2 => "Axis C",
        _ => unreachable!("axis ids are sanitized before serialization"),
    }
}

fn axis_index_from_char(axis: char) -> Option<usize> {
    match axis {
        'A' => Some(0),
        'B' => Some(1),
        'C' => Some(2),
        _ => None,
    }
}
