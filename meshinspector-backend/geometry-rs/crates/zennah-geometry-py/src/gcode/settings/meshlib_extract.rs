fn extract_meshlib_vector3_with_default(
    value: &Bound<'_, PyAny>,
    name: &str,
    default: [f64; 3],
) -> PyResult<[f64; 3]> {
    if let Ok(dict) = value.cast::<PyDict>() {
        return Ok([
            extract_meshlib_dict_number(dict, "x", name)?,
            extract_meshlib_dict_number(dict, "y", name)?,
            extract_meshlib_dict_number(dict, "z", name)?,
        ]);
    }
    if let Ok(text) = value.extract::<String>() {
        return parse_meshlib_vector3_string(&text, name, default);
    }
    Err(PyValueError::new_err(format!(
        "{name} must be a 3D vector object or string"
    )))
}

fn extract_meshlib_vector2_with_default(
    value: &Bound<'_, PyAny>,
    name: &str,
    default: [f64; 2],
) -> PyResult<[f64; 2]> {
    if let Ok(dict) = value.cast::<PyDict>() {
        return Ok([
            extract_meshlib_dict_number(dict, "x", name)?,
            extract_meshlib_dict_number(dict, "y", name)?,
        ]);
    }
    if let Ok(text) = value.extract::<String>() {
        return parse_meshlib_vector2_string(&text, name, default);
    }
    Err(PyValueError::new_err(format!(
        "{name} must be a 2D vector object or string"
    )))
}

fn extract_meshlib_dict_number(dict: &Bound<'_, PyDict>, key: &str, name: &str) -> PyResult<f64> {
    let value = required_dict_item(dict, key)?;
    extract_meshlib_number(&value, &format!("{name}.{key}"))
}

fn extract_meshlib_number(value: &Bound<'_, PyAny>, name: &str) -> PyResult<f64> {
    if value.is_instance_of::<PyBool>() {
        return Err(PyValueError::new_err(format!("{name} must be a number")));
    }
    value
        .extract::<f64>()
        .map_err(|_| PyValueError::new_err(format!("{name} must be a number")))
}

fn parse_meshlib_vector3_string(
    text: &str,
    name: &str,
    mut values: [f64; 3],
) -> PyResult<[f64; 3]> {
    parse_meshlib_vector_string(text, &mut values, name)?;
    Ok(values)
}

fn parse_meshlib_vector2_string(
    text: &str,
    name: &str,
    mut values: [f64; 2],
) -> PyResult<[f64; 2]> {
    parse_meshlib_vector_string(text, &mut values, name)?;
    Ok(values)
}

fn parse_meshlib_vector_string<const N: usize>(
    text: &str,
    values: &mut [f64; N],
    name: &str,
) -> PyResult<()> {
    for (index, token) in text.split_whitespace().take(N).enumerate() {
        values[index] = parse_meshlib_stream_float(token, name)?;
    }
    Ok(())
}

fn parse_meshlib_stream_float(token: &str, name: &str) -> PyResult<f64> {
    if let Ok(value) = token.parse::<f64>() {
        return Ok(value);
    }
    parse_meshlib_hex_float(token)
        .or_else(|| parse_meshlib_decimal_prefix(token))
        .or_else(|| parse_meshlib_hex_float_prefix(token))
        .ok_or_else(|| PyValueError::new_err(format!("{name} must contain numeric scalars")))
}
