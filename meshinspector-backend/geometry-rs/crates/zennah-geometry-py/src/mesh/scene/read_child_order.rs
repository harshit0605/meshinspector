fn read_scene_child_order(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneChildOrder>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_child_order must be a list"))?;
    let mut child_order = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item
            .cast::<PyDict>()
            .map_err(|_| PyValueError::new_err("scene_child_order entries must be dictionaries"))?;
        child_order.push(MeshlibSceneChildOrder {
            parent_key: required_string(dict, "parent_key")?,
            child_keys: optional_string_list(dict, "child_keys")?.unwrap_or_default(),
        });
    }
    Ok(child_order)
}
