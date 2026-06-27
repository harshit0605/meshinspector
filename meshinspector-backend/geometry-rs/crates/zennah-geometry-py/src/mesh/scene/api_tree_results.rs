fn scene_tree_group_result_to_py(
    py: Python<'_>,
    result: zennah_geometry_core::MeshlibSceneTreeGroupResult,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item(
        "scene_objects",
        result
            .objects
            .into_iter()
            .map(|scene_object| scene_export_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_group_objects",
        result
            .group_objects
            .into_iter()
            .map(|scene_object| scene_group_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_line_objects",
        result
            .line_objects
            .into_iter()
            .map(|scene_object| scene_line_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_point_objects",
        result
            .point_objects
            .into_iter()
            .map(|scene_object| scene_point_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_distance_map_objects",
        result
            .distance_map_objects
            .into_iter()
            .map(|scene_object| scene_distance_map_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_voxel_objects",
        result
            .voxel_objects
            .into_iter()
            .map(|scene_object| scene_voxel_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_feature_objects",
        result
            .feature_objects
            .into_iter()
            .map(|scene_object| scene_feature_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item("affected_object_keys", result.affected_object_keys)?;
    output.set_item("selected_object_keys", result.selected_object_keys)?;
    output.set_item("visible_object_keys", result.visible_object_keys)?;
    output.set_item("removed_object_keys", result.removed_object_keys)?;
    output.set_item(
        "scene_child_order",
        result
            .scene_child_order
            .into_iter()
            .map(|child_order| scene_child_order_to_py(py, child_order))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}

fn scene_tree_ungroup_result_to_py(
    py: Python<'_>,
    result: zennah_geometry_core::MeshlibSceneTreeUngroupResult,
) -> PyResult<Py<PyDict>> {
    let output = PyDict::new(py);
    output.set_item(
        "scene_objects",
        result
            .objects
            .into_iter()
            .map(|scene_object| scene_export_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_group_objects",
        result
            .group_objects
            .into_iter()
            .map(|scene_object| scene_group_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_line_objects",
        result
            .line_objects
            .into_iter()
            .map(|scene_object| scene_line_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_point_objects",
        result
            .point_objects
            .into_iter()
            .map(|scene_object| scene_point_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_distance_map_objects",
        result
            .distance_map_objects
            .into_iter()
            .map(|scene_object| scene_distance_map_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_voxel_objects",
        result
            .voxel_objects
            .into_iter()
            .map(|scene_object| scene_voxel_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item(
        "scene_feature_objects",
        result
            .feature_objects
            .into_iter()
            .map(|scene_object| scene_feature_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item("affected_object_keys", result.affected_object_keys)?;
    output.set_item("selected_object_keys", result.selected_object_keys)?;
    output.set_item("visible_object_keys", result.visible_object_keys)?;
    output.set_item("removed_object_keys", result.removed_object_keys)?;
    output.set_item(
        "scene_child_order",
        result
            .scene_child_order
            .into_iter()
            .map(|child_order| scene_child_order_to_py(py, child_order))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}
