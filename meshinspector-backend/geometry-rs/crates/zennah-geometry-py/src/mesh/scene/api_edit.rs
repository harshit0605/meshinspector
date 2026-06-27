#[pyfunction(signature = (
    vertices,
    scene_objects,
    object_key,
    xf,
    scene_feature_objects = None
))]
fn meshlib_transform_scene_object(
    py: Python<'_>,
    vertices: PyReadonlyArray2<'_, f64>,
    scene_objects: &Bound<'_, PyAny>,
    object_key: &str,
    xf: &Bound<'_, PyAny>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneTransformInput {
        vertices: read_vertices(vertices)?,
        objects: read_scene_export_objects(scene_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
        object_key: object_key.to_owned(),
        xf: read_scene_xf_value(xf, "xf")?,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_transform_scene_object(&input))
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item("vertices", vec3_lists(result.vertices))?;
    output.set_item(
        "scene_objects",
        result
            .objects
            .into_iter()
            .map(|scene_object| scene_export_object_to_py(py, scene_object))
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

#[pyfunction]
fn meshlib_reparent_scene_object(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    root_key: &str,
    object_key: &str,
    new_parent_key: &str,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneReparentInput {
        root_key: root_key.to_owned(),
        objects: read_scene_export_objects(scene_objects)?,
        object_key: object_key.to_owned(),
        new_parent_key: new_parent_key.to_owned(),
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_reparent_scene_object(&input))
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
    visibility_mask = None,
    selected = None,
    locked = None,
    parent_locked = None,
    scene_feature_objects = None
))]
fn meshlib_set_scene_object_state(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    object_key: &str,
    visibility_mask: Option<u32>,
    selected: Option<bool>,
    locked: Option<bool>,
    parent_locked: Option<bool>,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneObjectStateInput {
        objects: read_scene_export_objects(scene_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
        object_key: object_key.to_owned(),
        visibility_mask,
        selected,
        locked,
        parent_locked,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_set_scene_object_state(&input))
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
        "scene_feature_objects",
        result
            .feature_objects
            .into_iter()
            .map(|scene_object| scene_feature_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}

#[pyfunction(signature = (scene_objects, object_keys, mode = "select_one", scene_feature_objects = None))]
fn meshlib_select_scene_objects(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    object_keys: Vec<String>,
    mode: &str,
    scene_feature_objects: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let mode = match mode {
        "select_one" | "selectOne" | "replace" => MeshlibSceneSelectionMode::SelectOne,
        "toggle" | "primary_ctrl" | "primaryCtrl" | "ctrl" => MeshlibSceneSelectionMode::Toggle,
        _ => {
            return Err(PyValueError::new_err(format!(
                "Unsupported MeshLib scene selection mode {mode}"
            )))
        }
    };
    let input = MeshlibSceneSelectionInput {
        objects: read_scene_export_objects(scene_objects)?,
        feature_objects: read_scene_feature_objects(scene_feature_objects)?,
        object_keys,
        mode,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_select_scene_objects(&input))
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
        "scene_feature_objects",
        result
            .feature_objects
            .into_iter()
            .map(|scene_object| scene_feature_object_to_py(py, scene_object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    output.set_item("selected_object_keys", result.selected_object_keys)?;
    Ok(output.unbind())
}

#[pyfunction(signature = (
    scene_feature_objects,
    object_key,
    property,
    viewport_mask,
    dimension_name = None
))]
fn meshlib_set_scene_feature_object_visualize_property(
    py: Python<'_>,
    scene_feature_objects: &Bound<'_, PyAny>,
    object_key: &str,
    property: &str,
    viewport_mask: u32,
    dimension_name: Option<&str>,
) -> PyResult<Py<PyDict>> {
    let property = read_scene_feature_visualize_property(property, dimension_name)?;
    let input = MeshlibSceneFeatureVisualizePropertyInput {
        feature_objects: read_scene_feature_objects(Some(scene_feature_objects))?,
        object_key: object_key.to_owned(),
        property,
        viewport_mask,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_set_scene_feature_object_visualize_property(&input))
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
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

fn read_scene_feature_visualize_property(
    property: &str,
    dimension_name: Option<&str>,
) -> PyResult<MeshlibSceneFeatureVisualizeProperty> {
    let normalized = property.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "subfeatures" | "subfeature_visibility" | "subfeaturevisibility" => {
            Ok(MeshlibSceneFeatureVisualizeProperty::Subfeatures)
        }
        "details_on_name_tag" | "detailsonnametag" | "details" => {
            Ok(MeshlibSceneFeatureVisualizeProperty::DetailsOnNameTag)
        }
        "dimension" | "dimension_visibility" | "dimensionvisibility" => {
            let Some(name) = dimension_name else {
                return Err(PyValueError::new_err(
                    "dimension_name is required for FeatureObject dimension visibility",
                ));
            };
            Ok(MeshlibSceneFeatureVisualizeProperty::Dimension(
                name.to_owned(),
            ))
        }
        "diameter" | "angle" | "length" => {
            let mut chars = normalized.chars();
            let name = chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default();
            Ok(MeshlibSceneFeatureVisualizeProperty::Dimension(name))
        }
        _ => Err(PyValueError::new_err(format!(
            "Unsupported MeshLib FeatureObject visualize property {property}"
        ))),
    }
}

#[pyfunction(signature = (scene_feature_objects, viewport_mask = VIEWPORT_MASK_ALL, circle_segments = 64))]
fn meshlib_scene_feature_object_render_payload(
    py: Python<'_>,
    scene_feature_objects: &Bound<'_, PyAny>,
    viewport_mask: u32,
    circle_segments: usize,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneFeatureRenderInput {
        feature_objects: read_scene_feature_objects(Some(scene_feature_objects))?,
        viewport_mask,
        circle_segments,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_scene_feature_object_render_payload(&input))
        .map_err(PyValueError::new_err)?;
    let output = PyDict::new(py);
    output.set_item(
        "objects",
        result
            .objects
            .into_iter()
            .map(|object| scene_feature_render_object_to_py(py, object))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}

#[pyfunction]
fn meshlib_reorder_scene_children(
    py: Python<'_>,
    scene_objects: &Bound<'_, PyAny>,
    root_key: &str,
    parent_key: &str,
    ordered_child_keys: Vec<String>,
) -> PyResult<Py<PyDict>> {
    let input = MeshlibSceneReorderInput {
        root_key: root_key.to_owned(),
        parent_key: parent_key.to_owned(),
        objects: read_scene_export_objects(scene_objects)?,
        ordered_child_keys,
    };
    let result = py
        .detach(|| zennah_geometry_core::meshlib_reorder_scene_children(&input))
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
        "scene_child_order",
        result
            .scene_child_order
            .into_iter()
            .map(|child_order| scene_child_order_to_py(py, child_order))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(output.unbind())
}
