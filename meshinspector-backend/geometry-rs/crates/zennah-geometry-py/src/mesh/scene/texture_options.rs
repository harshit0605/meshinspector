fn read_texture_images(value: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<MeshlibSceneTextureImage>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("texture_images must be a list"))?;
    let mut textures = Vec::new();
    for item in list.iter() {
        let Ok(dict) = item.cast::<PyDict>() else {
            continue;
        };
        let Some(pixels_value) = dict.get_item("pixels_rgba")? else {
            continue;
        };
        let pixel_rows = pixels_value
            .extract::<Vec<Vec<i64>>>()
            .map_err(|_| PyValueError::new_err("texture_images[].pixels_rgba must be RGBA rows"))?;
        if pixel_rows.is_empty() {
            continue;
        }
        let mut pixels_rgba = Vec::with_capacity(pixel_rows.len());
        for row in pixel_rows {
            if row.len() != 4 {
                return Err(PyValueError::new_err(
                    "texture_images[].pixels_rgba rows must have 4 channels",
                ));
            }
            pixels_rgba.push([row[0] as u8, row[1] as u8, row[2] as u8, row[3] as u8]);
        }

        let mut width = optional_u32(dict, "width")?.unwrap_or(0);
        let mut height = optional_u32(dict, "height")?.unwrap_or(0);
        if width == 0 || height == 0 {
            height = 1;
            width = pixels_rgba.len() as u32;
        }
        textures.push(MeshlibSceneTextureImage {
            width,
            height,
            pixels_rgba,
            filter: optional_string(dict, "filter")?.unwrap_or_else(|| "Linear".to_owned()),
            wrap: optional_string(dict, "wrap")?.unwrap_or_else(|| "Clamp".to_owned()),
        });
    }
    Ok(textures)
}

fn optional_u32(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<u32>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<u32>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be an unsigned integer")))
}

fn optional_u64(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<u64>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<u64>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be an unsigned integer")))
}

fn optional_bool(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<bool>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<bool>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be a boolean")))
}

fn optional_string(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<String>> {
    let Some(value) = dict.get_item(key)? else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    value
        .extract::<String>()
        .map(Some)
        .map_err(|_| PyValueError::new_err(format!("{key} must be a string")))
}
