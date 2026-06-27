fn scene_export_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneExportObject,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item("model_file", scene_object.model_file)?;
    output.set_item("model_extension", scene_object.model_extension)?;
    output.set_item("link", scene_object.link)?;
    output.set_item(
        "shared_model_source_index",
        scene_object.shared_model_source_index,
    )?;
    output.set_item("vertex_range", scene_object.vertex_range.to_vec())?;
    output.set_item("face_range", scene_object.face_range.to_vec())?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_line_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneObjectLines,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item(
        "points",
        scene_object
            .points
            .into_iter()
            .map(|point| point.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item(
        "lines",
        scene_object
            .lines
            .into_iter()
            .map(|line| line.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("show_points", scene_object.show_points)?;
    output.set_item("smooth_connections", scene_object.smooth_connections)?;
    output.set_item("line_width", scene_object.line_width)?;
    output.set_item("coloring_type", scene_object.coloring_type)?;
    output.set_item("line_colors", rgba_rows_to_py(scene_object.line_colors))?;
    output.set_item("vert_colors", rgba_rows_to_py(scene_object.vert_colors))?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_group_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneGroupObject,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_point_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneObjectPoints,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item("model_file", scene_object.model_file)?;
    output.set_item("model_extension", scene_object.model_extension)?;
    output.set_item("link", scene_object.link)?;
    output.set_item(
        "points",
        scene_object
            .points
            .into_iter()
            .map(|point| point.to_vec())
            .collect::<Vec<_>>(),
    )?;
    output.set_item("normals", vec3_lists(scene_object.normals))?;
    output.set_item("vert_colors", rgba_rows_to_py(scene_object.vert_colors))?;
    output.set_item("point_size", scene_object.point_size)?;
    output.set_item("max_rendering_points", scene_object.max_rendering_points)?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_distance_map_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneObjectDistanceMap,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item("model_file", scene_object.model_file)?;
    output.set_item("model_extension", scene_object.model_extension)?;
    output.set_item("link", scene_object.link)?;
    output.set_item("width", scene_object.width)?;
    output.set_item("height", scene_object.height)?;
    output.set_item("values", scene_object.values)?;
    output.set_item("valid_count", scene_object.valid_count)?;
    output.set_item("min_value", scene_object.min_value)?;
    output.set_item("max_value", scene_object.max_value)?;
    output.set_item("origin_world", scene_object.origin_world.to_vec())?;
    output.set_item("pixel_x_vec", scene_object.pixel_x_vec.to_vec())?;
    output.set_item("pixel_y_vec", scene_object.pixel_y_vec.to_vec())?;
    output.set_item("depth_vec", scene_object.depth_vec.to_vec())?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_voxel_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneObjectVoxels,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    let model_bytes_base64 = if scene_object.model_bytes.is_empty() {
        None
    } else {
        Some(STANDARD.encode(&scene_object.model_bytes))
    };
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item("model_file", scene_object.model_file)?;
    output.set_item("model_extension", scene_object.model_extension)?;
    if let Some(model_bytes_base64) = model_bytes_base64 {
        output.set_item("model_bytes_base64", model_bytes_base64)?;
    }
    output.set_item("link", scene_object.link)?;
    output.set_item("dimensions", scene_object.dimensions.to_vec())?;
    output.set_item("voxel_size", scene_object.voxel_size.to_vec())?;
    output.set_item("grid_level_set", scene_object.grid_level_set)?;
    output.set_item("values", scene_object.values)?;
    output.set_item("min_value", scene_object.min_value)?;
    output.set_item("max_value", scene_object.max_value)?;
    output.set_item("min_corner", scene_object.min_corner.to_vec())?;
    output.set_item("max_corner", scene_object.max_corner.to_vec())?;
    output.set_item("iso_value", scene_object.iso_value)?;
    output.set_item("dual_marching_cubes", scene_object.dual_marching_cubes)?;
    output.set_item("selected_voxels", scene_object.selected_voxels)?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_feature_object_to_py(
    py: Python<'_>,
    scene_object: MeshlibSceneFeatureObject,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("object_name", scene_object.object_name)?;
    output.set_item("object_key", scene_object.object_key)?;
    output.set_item("parent_key", scene_object.parent_key)?;
    output.set_item("hierarchy_path", scene_object.hierarchy_path)?;
    output.set_item("feature_type", scene_object.feature_type)?;
    output.set_item("subfeature_visibility", scene_object.subfeature_visibility)?;
    output.set_item("details_on_name_tag", scene_object.details_on_name_tag)?;
    output.set_item(
        "decorations_color_unselected",
        scene_object.decorations_color_unselected.to_vec(),
    )?;
    output.set_item(
        "decorations_color_selected",
        scene_object.decorations_color_selected.to_vec(),
    )?;
    output.set_item("point_size", scene_object.point_size)?;
    output.set_item("line_width", scene_object.line_width)?;
    output.set_item("sub_point_size", scene_object.sub_point_size)?;
    output.set_item("sub_line_width", scene_object.sub_line_width)?;
    output.set_item("main_alpha", scene_object.main_alpha)?;
    output.set_item("sub_alpha_points", scene_object.sub_alpha_points)?;
    output.set_item("sub_alpha_lines", scene_object.sub_alpha_lines)?;
    output.set_item("sub_alpha_mesh", scene_object.sub_alpha_mesh)?;
    output.set_item("dimension_visibility", scene_object.dimension_visibility)?;
    output.set_item("xf", scene_xf_to_py(py, scene_object.xf)?)?;
    output.set_item("visibility_mask", scene_object.visibility_mask)?;
    output.set_item("selected", scene_object.selected)?;
    output.set_item("locked", scene_object.locked)?;
    output.set_item("parent_locked", scene_object.parent_locked)?;
    Ok(output.unbind())
}

fn scene_child_order_to_py(
    py: Python<'_>,
    child_order: MeshlibSceneChildOrder,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item("parent_key", child_order.parent_key)?;
    output.set_item("child_keys", child_order.child_keys)?;
    Ok(output.unbind())
}

fn rgba_rows_to_py(colors: Vec<[u8; 4]>) -> Vec<Vec<i64>> {
    colors
        .into_iter()
        .map(|color| color.into_iter().map(i64::from).collect())
        .collect()
}
