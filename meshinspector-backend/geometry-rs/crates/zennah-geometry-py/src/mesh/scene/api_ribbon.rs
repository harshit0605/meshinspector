#[pyfunction(signature = (
    scene_objects,
    root_key,
    action,
    scene_line_objects = None,
    scene_point_objects = None,
    scene_distance_map_objects = None,
    scene_feature_objects = None,
    scene_voxel_objects = None,
    scene_group_objects = None
))]
fn meshlib_apply_scene_ribbon_action(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    root_key: &str,
    action: &str,
    scene_line_objects: Option<&Bound<'_, PyAny>>,
    scene_point_objects: Option<&Bound<'_, PyAny>>,
    scene_distance_map_objects: Option<&Bound<'_, PyAny>>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
    scene_voxel_objects: Option<&Bound<'_, PyAny>>,
    scene_group_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let action = read_scene_ribbon_action(action)?;
    let input = MeshlibSceneTreeRibbonActionInput {
        root_key: root_key.to_owned(),
        objects: read_scene_export_objects(scene_objects)?,
        group_objects: read_scene_group_objects(scene_group_objects)?,
        line_objects: read_scene_line_objects(scene_line_objects)?,
        point_objects: read_scene_point_objects(scene_point_objects)?,
        distance_map_objects: read_scene_distance_map_objects(scene_distance_map_objects)?,
        voxel_objects: read_scene_voxel_objects(scene_voxel_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
        action,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_apply_scene_tree_ribbon_action(&input))
        .map_err(PyValueError::new_err)?;
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
        "scene_line_objects",
        result
            .line_objects
            .into_iter()
            .map(|scene_object| scene_line_object_to_py(py, scene_object))
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

#[pyfunction(signature = (
    scene_objects,
    object_key,
    object_name,
    scene_line_objects = None,
    scene_point_objects = None,
    scene_distance_map_objects = None,
    scene_feature_objects = None,
    scene_voxel_objects = None,
    scene_group_objects = None
))]
fn meshlib_rename_scene_object(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    object_key: &str,
    object_name: &str,
    scene_line_objects: Option<&Bound<'_, PyAny>>,
    scene_point_objects: Option<&Bound<'_, PyAny>>,
    scene_distance_map_objects: Option<&Bound<'_, PyAny>>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
    scene_voxel_objects: Option<&Bound<'_, PyAny>>,
    scene_group_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneTreeRenameInput {
        objects: read_scene_export_objects(scene_objects)?,
        group_objects: read_scene_group_objects(scene_group_objects)?,
        line_objects: read_scene_line_objects(scene_line_objects)?,
        point_objects: read_scene_point_objects(scene_point_objects)?,
        distance_map_objects: read_scene_distance_map_objects(scene_distance_map_objects)?,
        voxel_objects: read_scene_voxel_objects(scene_voxel_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
        object_key: object_key.to_owned(),
        object_name: object_name.to_owned(),
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_rename_scene_tree_object(&input))
        .map_err(PyValueError::new_err)?;
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
        "scene_line_objects",
        result
            .line_objects
            .into_iter()
            .map(|scene_object| scene_line_object_to_py(py, scene_object))
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
    Ok(output.unbind())
}

#[pyfunction(signature = (
    scene_objects,
    root_key,
    group_key,
    scene_line_objects = None,
    scene_point_objects = None,
    scene_distance_map_objects = None,
    scene_feature_objects = None,
    scene_voxel_objects = None,
    scene_group_objects = None
))]
fn meshlib_group_scene_objects(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    root_key: &str,
    group_key: &str,
    scene_line_objects: Option<&Bound<'_, PyAny>>,
    scene_point_objects: Option<&Bound<'_, PyAny>>,
    scene_distance_map_objects: Option<&Bound<'_, PyAny>>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
    scene_voxel_objects: Option<&Bound<'_, PyAny>>,
    scene_group_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneTreeGroupInput {
        root_key: root_key.to_owned(),
        group_key: group_key.to_owned(),
        objects: read_scene_export_objects(scene_objects)?,
        group_objects: read_scene_group_objects(scene_group_objects)?,
        line_objects: read_scene_line_objects(scene_line_objects)?,
        point_objects: read_scene_point_objects(scene_point_objects)?,
        distance_map_objects: read_scene_distance_map_objects(scene_distance_map_objects)?,
        voxel_objects: read_scene_voxel_objects(scene_voxel_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_group_scene_tree_objects(&input))
        .map_err(PyValueError::new_err)?;
    scene_tree_group_result_to_py(py, result)
}

#[pyfunction(signature = (
    scene_objects,
    root_key,
    scene_line_objects = None,
    scene_point_objects = None,
    scene_distance_map_objects = None,
    scene_feature_objects = None,
    scene_voxel_objects = None,
    scene_group_objects = None
))]
fn meshlib_ungroup_scene_objects(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    root_key: &str,
    scene_line_objects: Option<&Bound<'_, PyAny>>,
    scene_point_objects: Option<&Bound<'_, PyAny>>,
    scene_distance_map_objects: Option<&Bound<'_, PyAny>>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
    scene_voxel_objects: Option<&Bound<'_, PyAny>>,
    scene_group_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneTreeUngroupInput {
        root_key: root_key.to_owned(),
        objects: read_scene_export_objects(scene_objects)?,
        group_objects: read_scene_group_objects(scene_group_objects)?,
        line_objects: read_scene_line_objects(scene_line_objects)?,
        point_objects: read_scene_point_objects(scene_point_objects)?,
        distance_map_objects: read_scene_distance_map_objects(scene_distance_map_objects)?,
        voxel_objects: read_scene_voxel_objects(scene_voxel_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_ungroup_scene_tree_objects(&input))
        .map_err(PyValueError::new_err)?;
    scene_tree_ungroup_result_to_py(py, result)
}

fn read_scene_ribbon_action(action: &str) -> PyResult<MeshlibSceneRibbonAction> {
    match action {
        "select_all" | "selectAll" | "Ribbon Scene Select all" => {
            Ok(MeshlibSceneRibbonAction::SelectAll)
        }
        "unselect_all" | "unselectAll" | "Ribbon Scene Unselect all" => {
            Ok(MeshlibSceneRibbonAction::UnselectAll)
        }
        "show_all" | "showAll" | "Ribbon Scene Show all" => Ok(MeshlibSceneRibbonAction::ShowAll),
        "hide_all" | "hideAll" | "Ribbon Scene Hide all" => Ok(MeshlibSceneRibbonAction::HideAll),
        "show_only_previous"
        | "show_only_prev"
        | "showOnlyPrevious"
        | "showOnlyPrev"
        | "Ribbon Scene Show only previous" => Ok(MeshlibSceneRibbonAction::ShowOnlyPrevious),
        "show_only_next" | "showOnlyNext" | "Ribbon Scene Show only next" => {
            Ok(MeshlibSceneRibbonAction::ShowOnlyNext)
        }
        "sort_by_name" | "sortByName" | "Ribbon Scene Sort by name" => {
            Ok(MeshlibSceneRibbonAction::SortByName)
        }
        "remove_selected" | "remove_selected_objects" | "Ribbon Scene Remove selected objects" => {
            Ok(MeshlibSceneRibbonAction::RemoveSelected)
        }
        _ => Err(PyValueError::new_err(format!(
            "Unsupported MeshLib scene ribbon action {action}"
        ))),
    }
}
