fn read_scene_export_objects(value: &Bound<'_, PyAny>) -> PyResult<Vec<MeshlibSceneExportObject>> {
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item
            .cast::<PyDict>()
            .map_err(|_| PyValueError::new_err("scene_objects entries must be dictionaries"))?;
        objects.push(MeshlibSceneExportObject {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            model_file: optional_string(dict, "model_file")?.unwrap_or_default(),
            model_extension: optional_string(dict, "model_extension")?
                .unwrap_or_else(|| ".ply".to_owned()),
            link: optional_string(dict, "link")?,
            shared_model_source_index: optional_usize(dict, "shared_model_source_index")?,
            vertex_range: required_usize_pair(dict, "vertex_range")?,
            face_range: required_usize_pair(dict, "face_range")?,
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_line_objects(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneObjectLines>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_line_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item.cast::<PyDict>().map_err(|_| {
            PyValueError::new_err("scene_line_objects entries must be dictionaries")
        })?;
        objects.push(MeshlibSceneObjectLines {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            points: optional_vec3_list(dict, "points")?.unwrap_or_default(),
            lines: optional_usize_pair_list(dict, "lines")?.unwrap_or_default(),
            show_points: optional_u32(dict, "show_points")?.unwrap_or(0),
            smooth_connections: optional_u32(dict, "smooth_connections")?.unwrap_or(0),
            line_width: optional_f32(dict, "line_width")?.unwrap_or(1.0),
            coloring_type: optional_string(dict, "coloring_type")?
                .unwrap_or_else(|| "Solid".to_owned()),
            line_colors: optional_rgba_rows(dict, "line_colors")?.unwrap_or_default(),
            vert_colors: optional_rgba_rows(dict, "vert_colors")?.unwrap_or_default(),
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_group_objects(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneGroupObject>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_group_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item.cast::<PyDict>().map_err(|_| {
            PyValueError::new_err("scene_group_objects entries must be dictionaries")
        })?;
        objects.push(MeshlibSceneGroupObject {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_point_objects(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneObjectPoints>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_point_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item.cast::<PyDict>().map_err(|_| {
            PyValueError::new_err("scene_point_objects entries must be dictionaries")
        })?;
        objects.push(MeshlibSceneObjectPoints {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            model_file: optional_string(dict, "model_file")?.unwrap_or_default(),
            model_extension: optional_string(dict, "model_extension")?
                .unwrap_or_else(|| ".ply".to_owned()),
            link: optional_string(dict, "link")?,
            points: optional_vec3_list(dict, "points")?.unwrap_or_default(),
            normals: optional_vec3_list(dict, "normals")?.unwrap_or_default(),
            vert_colors: optional_rgba_rows(dict, "vert_colors")?.unwrap_or_default(),
            point_size: optional_f32(dict, "point_size")?.unwrap_or(5.0),
            max_rendering_points: optional_u64(dict, "max_rendering_points")?.unwrap_or(0),
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_distance_map_objects(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneObjectDistanceMap>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_distance_map_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item.cast::<PyDict>().map_err(|_| {
            PyValueError::new_err("scene_distance_map_objects entries must be dictionaries")
        })?;
        let values = required_f32_list(dict, "values")?;
        let (valid_count, min_value, max_value) = distance_map_stats_for_py(&values);
        objects.push(MeshlibSceneObjectDistanceMap {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            model_file: optional_string(dict, "model_file")?.unwrap_or_default(),
            model_extension: optional_string(dict, "model_extension")?
                .unwrap_or_else(|| ".raw".to_owned()),
            link: optional_string(dict, "link")?,
            width: required_usize(dict, "width")?,
            height: required_usize(dict, "height")?,
            values,
            valid_count,
            min_value,
            max_value,
            origin_world: optional_vec3(dict, "origin_world")?.unwrap_or([0.0, 0.0, 0.0]),
            pixel_x_vec: optional_vec3(dict, "pixel_x_vec")?.unwrap_or([1.0, 0.0, 0.0]),
            pixel_y_vec: optional_vec3(dict, "pixel_y_vec")?.unwrap_or([0.0, 1.0, 0.0]),
            depth_vec: optional_vec3(dict, "depth_vec")?.unwrap_or([0.0, 0.0, 1.0]),
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_voxel_objects(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneObjectVoxels>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_voxel_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item.cast::<PyDict>().map_err(|_| {
            PyValueError::new_err("scene_voxel_objects entries must be dictionaries")
        })?;
        let dimensions = required_usize_triple(dict, "dimensions")?;
        let values = required_f32_list(dict, "values")?;
        let (min_value, max_value) = voxel_stats_for_py(&values);
        let model_extension = optional_string(dict, "model_extension")?
            .unwrap_or_else(|| ".raw".to_owned());
        let model_bytes = optional_string(dict, "model_bytes_base64")?
            .map(|encoded| {
                STANDARD.decode(encoded.as_bytes()).map_err(|_| {
                    PyValueError::new_err("model_bytes_base64 must be valid base64")
                })
            })
            .transpose()?
            .unwrap_or_default();
        objects.push(MeshlibSceneObjectVoxels {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            model_file: optional_string(dict, "model_file")?.unwrap_or_default(),
            model_extension,
            link: optional_string(dict, "link")?,
            model_bytes,
            dimensions,
            voxel_size: required_f32_triple(dict, "voxel_size")?,
            grid_level_set: optional_bool(dict, "grid_level_set")?.unwrap_or(false),
            values,
            min_value,
            max_value,
            min_corner: optional_usize_triple(dict, "min_corner")?.unwrap_or([0, 0, 0]),
            max_corner: optional_usize_triple(dict, "max_corner")?.unwrap_or(dimensions),
            iso_value: optional_f32(dict, "iso_value")?.unwrap_or((min_value + max_value) * 0.5),
            dual_marching_cubes: optional_bool(dict, "dual_marching_cubes")?.unwrap_or(false),
            selected_voxels: optional_usize_list(dict, "selected_voxels")?.unwrap_or_default(),
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_feature_objects(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<MeshlibSceneFeatureObject>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    let list = value
        .cast::<PyList>()
        .map_err(|_| PyValueError::new_err("scene_feature_objects must be a list"))?;
    let mut objects = Vec::with_capacity(list.len());
    for item in list.iter() {
        let dict = item.cast::<PyDict>().map_err(|_| {
            PyValueError::new_err("scene_feature_objects entries must be dictionaries")
        })?;
        objects.push(MeshlibSceneFeatureObject {
            object_name: required_string(dict, "object_name")?,
            object_key: required_string(dict, "object_key")?,
            parent_key: optional_string(dict, "parent_key")?.unwrap_or_default(),
            hierarchy_path: optional_string_list(dict, "hierarchy_path")?.unwrap_or_default(),
            feature_type: optional_string(dict, "feature_type")?
                .unwrap_or_else(|| "PlaneObject".to_owned()),
            subfeature_visibility: optional_u32(dict, "subfeature_visibility")?.unwrap_or(0),
            details_on_name_tag: optional_u32(dict, "details_on_name_tag")?.unwrap_or(0),
            decorations_color_unselected: optional_vec4(dict, "decorations_color_unselected")?
                .unwrap_or([0.6, 0.6, 0.6, 1.0]),
            decorations_color_selected: optional_vec4(dict, "decorations_color_selected")?
                .unwrap_or([1.0, 0.78, 0.22, 1.0]),
            point_size: optional_f32(dict, "point_size")?.unwrap_or(5.0),
            line_width: optional_f32(dict, "line_width")?.unwrap_or(1.0),
            sub_point_size: optional_f32(dict, "sub_point_size")?.unwrap_or(3.0),
            sub_line_width: optional_f32(dict, "sub_line_width")?.unwrap_or(1.0),
            main_alpha: optional_f32(dict, "main_alpha")?.unwrap_or(1.0),
            sub_alpha_points: optional_f32(dict, "sub_alpha_points")?.unwrap_or(1.0),
            sub_alpha_lines: optional_f32(dict, "sub_alpha_lines")?.unwrap_or(1.0),
            sub_alpha_mesh: optional_f32(dict, "sub_alpha_mesh")?.unwrap_or(1.0),
            dimension_visibility: optional_u32_map(dict, "dimension_visibility")?.unwrap_or_default(),
            xf: read_scene_xf(dict)?,
            visibility_mask: optional_u32(dict, "visibility_mask")?.unwrap_or(VIEWPORT_MASK_ALL),
            selected: optional_bool(dict, "selected")?.unwrap_or(false),
            locked: optional_bool(dict, "locked")?.unwrap_or(false),
            parent_locked: optional_bool(dict, "parent_locked")?.unwrap_or(false),
        });
    }
    Ok(objects)
}

fn read_scene_xf(dict: &Bound<'_, PyDict>) -> PyResult<MeshlibSceneXf> {
    let Some(value) = dict.get_item("xf")? else {
        return Ok(MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [0.0, 0.0, 0.0],
        });
    };
    if value.is_none() {
        return Ok(MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [0.0, 0.0, 0.0],
        });
    }
    read_scene_xf_value(&value, "scene_objects[].xf")
}

fn read_scene_xf_value(value: &Bound<'_, PyAny>, field: &str) -> PyResult<MeshlibSceneXf> {
    if value.is_none() {
        return Ok(MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [0.0, 0.0, 0.0],
        });
    }
    let xf = value
        .cast::<PyDict>()
        .map_err(|_| PyValueError::new_err(format!("{field} must be a dictionary")))?;
    Ok(MeshlibSceneXf {
        row_x: optional_vec3(xf, "row_x")?.unwrap_or([1.0, 0.0, 0.0]),
        row_y: optional_vec3(xf, "row_y")?.unwrap_or([0.0, 1.0, 0.0]),
        row_z: optional_vec3(xf, "row_z")?.unwrap_or([0.0, 0.0, 1.0]),
        b: optional_vec3(xf, "b")?.unwrap_or([0.0, 0.0, 0.0]),
    })
}
