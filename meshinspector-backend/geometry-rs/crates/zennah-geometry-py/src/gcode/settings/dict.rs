pub(super) fn machine_settings_from_dict(
    settings: &Bound<'_, PyDict>,
) -> PyResult<zennah_geometry_core::gcode::GcodeMachineSettings> {
    let mut parsed = zennah_geometry_core::gcode::GcodeMachineSettings::default();
    if let Some(value) = settings.get_item("home_position")? {
        parsed.home_position = extract_vector3(&value, "home_position")?;
    }
    if let Some(value) = settings.get_item("feedrate_idle")? {
        parsed.feedrate_idle = value
            .extract::<f64>()
            .map_err(|_| PyValueError::new_err("feedrate_idle must be a number"))?;
    }
    if let Some(value) = settings.get_item("rotation_axes")? {
        let axes = value
            .extract::<Vec<Vec<f64>>>()
            .map_err(|_| PyValueError::new_err("rotation_axes must be three 3D vectors"))?;
        if axes.len() != 3 {
            return Err(PyValueError::new_err(
                "rotation_axes must contain exactly three axes",
            ));
        }
        parsed.rotation_axes = [
            vector3_from_values(&axes[0], "rotation_axes[0]")?,
            vector3_from_values(&axes[1], "rotation_axes[1]")?,
            vector3_from_values(&axes[2], "rotation_axes[2]")?,
        ];
    }
    if let Some(value) = settings.get_item("rotation_order")? {
        parsed.rotation_order = value
            .extract::<Vec<usize>>()
            .map_err(|_| PyValueError::new_err("rotation_order must be a sequence of axis ids"))?;
    }
    if let Some(value) = settings.get_item("rotation_limits")? {
        let limits = value.extract::<Vec<Option<Vec<f64>>>>().map_err(|_| {
            PyValueError::new_err("rotation_limits must be null or [min, max] for each axis")
        })?;
        if limits.len() != 3 {
            return Err(PyValueError::new_err(
                "rotation_limits must contain exactly three axis entries",
            ));
        }
        parsed.rotation_limits = [
            optional_vector2_from_values(limits[0].as_deref(), "rotation_limits[0]")?,
            optional_vector2_from_values(limits[1].as_deref(), "rotation_limits[1]")?,
            optional_vector2_from_values(limits[2].as_deref(), "rotation_limits[2]")?,
        ];
    }
    Ok(parsed)
}

fn machine_settings_to_dict(
    py: Python<'_>,
    settings: &zennah_geometry_core::gcode::GcodeMachineSettings,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("home_position", settings.home_position.to_vec())?;
    output.set_item("feedrate_idle", settings.feedrate_idle)?;
    output.set_item(
        "rotation_axes",
        settings
            .rotation_axes
            .iter()
            .map(|axis| axis.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("rotation_order", settings.rotation_order.clone())?;
    output.set_item(
        "rotation_limits",
        settings
            .rotation_limits
            .iter()
            .map(|limits| limits.map(|value| value.to_vec()))
            .collect::<Vec<_>>(),
    )?;
    Ok(output.unbind())
}

fn required_dict_item<'py>(dict: &Bound<'py, PyDict>, key: &str) -> PyResult<Bound<'py, PyAny>> {
    dict.get_item(key)?
        .ok_or_else(|| PyValueError::new_err(format!("{key} must be present")))
}

fn meshlib_vector3_to_dict<'py>(py: Python<'py>, values: [f64; 3]) -> PyResult<Bound<'py, PyDict>> {
    let output = PyDict::new(py);
    output.set_item("x", values[0])?;
    output.set_item("y", values[1])?;
    output.set_item("z", values[2])?;
    Ok(output)
}

fn meshlib_vector2_to_dict<'py>(py: Python<'py>, values: [f64; 2]) -> PyResult<Bound<'py, PyDict>> {
    let output = PyDict::new(py);
    output.set_item("x", values[0])?;
    output.set_item("y", values[1])?;
    Ok(output)
}
