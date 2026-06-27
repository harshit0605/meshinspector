#[pyfunction]
fn gcode_machine_settings_to_meshlib_json(
    py: Python<'_>,
    settings: &Bound<'_, PyDict>,
) -> PyResult<Py<PyDict>> {
    let settings = machine_settings_from_dict(settings)?.sanitized();
    let output = PyDict::new(py);

    let mut order = String::new();
    let mut active_axes = [false; 3];
    for axis in settings
        .rotation_order
        .iter()
        .copied()
        .filter(|axis| *axis < 3)
    {
        order.push(axis_letter(axis));
        active_axes[axis] = true;
    }
    output.set_item("Axes Order", order)?;

    for axis in 0..3 {
        if !active_axes[axis] {
            continue;
        }
        let axis_dict = PyDict::new(py);
        axis_dict.set_item(
            "Direction",
            meshlib_vector3_to_dict(py, settings.rotation_axes[axis])?,
        )?;
        if let Some(limits) = settings.rotation_limits[axis] {
            axis_dict.set_item("Limits", meshlib_vector2_to_dict(py, limits)?)?;
        } else {
            axis_dict.set_item("Limits", false)?;
        }
        output.set_item(axis_json_name(axis), axis_dict)?;
    }

    output.set_item("Feedrate Idle", settings.feedrate_idle)?;
    output.set_item(
        "Home Position",
        meshlib_vector3_to_dict(py, settings.home_position)?,
    )?;
    Ok(output.unbind())
}

#[pyfunction]
fn gcode_machine_settings_from_meshlib_json(
    py: Python<'_>,
    settings_json: &Bound<'_, PyDict>,
) -> PyResult<Py<PyDict>> {
    let mut settings = zennah_geometry_core::gcode::GcodeMachineSettings::default();
    let order_value = required_dict_item(settings_json, "Axes Order")?;
    let order = order_value
        .extract::<String>()
        .map_err(|_| PyValueError::new_err("Axes Order must be a string"))?;
    let mut read_axes = [false; 3];
    let mut rotation_order = Vec::new();
    for axis in order.chars().filter_map(axis_index_from_char) {
        if read_axes[axis] {
            return Err(PyValueError::new_err(
                "Axes Order contains duplicate rotation axes",
            ));
        }
        read_axes[axis] = true;
        rotation_order.push(axis);
    }
    settings.rotation_order = rotation_order;

    for axis in 0..3 {
        if !read_axes[axis] {
            continue;
        }
        let axis_dict = required_dict_item(settings_json, axis_json_name(axis))?;
        let direction = extract_meshlib_vector3_with_default(
            &axis_dict
                .get_item("Direction")
                .map_err(|_| PyValueError::new_err("axis Direction must be present"))?,
            axis_json_name(axis),
            [0.0, 0.0, 0.0],
        )?;
        if direction == [0.0, 0.0, 0.0] {
            return Err(PyValueError::new_err("axis Direction must be non-zero"));
        }
        settings.rotation_axes[axis] = direction;

        let limits_value = axis_dict
            .get_item("Limits")
            .map_err(|_| PyValueError::new_err("axis Limits must be present"))?;
        if limits_value.is_instance_of::<PyBool>() {
            settings.rotation_limits[axis] = None;
        } else {
            let limits = extract_meshlib_vector2_with_default(
                &limits_value,
                "axis Limits",
                [180.0, -180.0],
            )?;
            if limits == [180.0, -180.0] {
                return Err(PyValueError::new_err(
                    "axis Limits must be a valid 2D vector",
                ));
            }
            settings.rotation_limits[axis] = Some(limits);
        }
    }

    let feedrate_value = required_dict_item(settings_json, "Feedrate Idle")?;
    settings.feedrate_idle = extract_meshlib_number(&feedrate_value, "Feedrate Idle")?;
    let home_value = required_dict_item(settings_json, "Home Position")?;
    settings.home_position =
        extract_meshlib_vector3_with_default(&home_value, "Home Position", [f64::MAX; 3])?;
    if settings.home_position == [f64::MAX; 3] {
        return Err(PyValueError::new_err(
            "Home Position must be a valid 3D vector",
        ));
    }

    let raw_feedrate = settings.feedrate_idle;
    let mut normalized = settings.sanitized();
    normalized.feedrate_idle = raw_feedrate;
    machine_settings_to_dict(py, &normalized)
}
