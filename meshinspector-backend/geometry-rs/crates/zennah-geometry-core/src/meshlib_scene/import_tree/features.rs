pub(super) fn scene_feature_object_from_node(
    node: MeshlibSceneObjectNode,
) -> Result<MeshlibSceneFeatureObject, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "FeatureObject".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let feature_type =
        meshlib_feature_type_from_value(&object).unwrap_or_else(|| "FeatureObject".to_string());
    let feature_object = MeshlibSceneFeatureObject {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        feature_type,
        subfeature_visibility: object
            .get("SubfeatureVisibility")
            .and_then(Value::as_u64)
            .map(|value| value as u32)
            .unwrap_or(VIEWPORT_MASK_ALL),
        details_on_name_tag: object
            .get("DetailsOnNameTag")
            .and_then(Value::as_u64)
            .map(|value| value as u32)
            .unwrap_or(VIEWPORT_MASK_ALL),
        decorations_color_unselected: meshlib_json_vec4(
            object.get("DecorationsColorUnselected"),
            [0.0, 0.0, 0.0, 1.0],
        ),
        decorations_color_selected: meshlib_json_vec4(
            object.get("DecorationsColorSelected"),
            [1.0, 0.7, 0.0, 1.0],
        ),
        point_size: object
            .get("PointSize")
            .and_then(Value::as_f64)
            .unwrap_or(10.0) as f32,
        line_width: object
            .get("LineWidth")
            .and_then(Value::as_f64)
            .unwrap_or(2.0) as f32,
        sub_point_size: object
            .get("SubPointSize")
            .and_then(Value::as_f64)
            .unwrap_or(6.0) as f32,
        sub_line_width: object
            .get("SubLineWidth")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32,
        main_alpha: object
            .get("MainAlpha")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32,
        sub_alpha_points: object
            .get("SubAlphaPoints")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32,
        sub_alpha_lines: object
            .get("SubAlphaLines")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32,
        sub_alpha_mesh: object
            .get("SubAlphaMesh")
            .and_then(Value::as_f64)
            .unwrap_or(0.5) as f32,
        dimension_visibility: decode_meshlib_u32_map(object.get("DimensionVisibility")),
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    meshlib_validate_scene_feature_object(&feature_object)?;
    Ok(feature_object)
}
