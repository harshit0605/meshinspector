pub fn meshlib_transform_scene_object(
    input: &MeshlibSceneTransformInput,
) -> Result<MeshlibSceneTransformResult, String> {
    if let Some(object_index) = input
        .objects
        .iter()
        .position(|object| object.object_key == input.object_key)
    {
        let target = &input.objects[object_index];
        let vertex_start = target.vertex_range[0];
        let vertex_end = target.vertex_range[1];
        if vertex_start > vertex_end || vertex_end > input.vertices.len() {
            return Err(format!(
                "Invalid vertex range for MRU object {}",
                target.object_key
            ));
        }

        let mut vertices = input.vertices.clone();
        for vertex in vertices[vertex_start..vertex_end].iter_mut() {
            let local = target.xf.inverse_transform_point(*vertex)?;
            *vertex = input.xf.transform_point(local);
        }

        let mut objects = input.objects.clone();
        objects[object_index].xf = input.xf;
        return Ok(MeshlibSceneTransformResult {
            vertices,
            objects,
            feature_objects: input.feature_objects.clone(),
        });
    }

    if let Some(feature_index) = input
        .feature_objects
        .iter()
        .position(|object| object.object_key == input.object_key)
    {
        let mut feature_objects = input.feature_objects.clone();
        feature_objects[feature_index].xf = input.xf;
        return Ok(MeshlibSceneTransformResult {
            vertices: input.vertices.clone(),
            objects: input.objects.clone(),
            feature_objects,
        });
    }

    Err(format!("MRU scene object {} was not found", input.object_key))
}

pub fn meshlib_reparent_scene_object(
    input: &MeshlibSceneReparentInput,
) -> Result<MeshlibSceneReparentResult, String> {
    let root_key = meshlib_reparent_root_key(input);
    let object_index_by_key = meshlib_scene_object_index_by_key(&input.objects)?;
    let object_index = *object_index_by_key
        .get(&input.object_key)
        .ok_or_else(|| format!("MRU scene object {} was not found", input.object_key))?;
    let new_parent_key = if input.new_parent_key.is_empty() {
        root_key
    } else {
        input.new_parent_key.as_str()
    };
    if new_parent_key == input.object_key {
        return Err(format!(
            "MRU scene object {} cannot be parented to itself",
            input.object_key
        ));
    }
    if new_parent_key != root_key && !object_index_by_key.contains_key(new_parent_key) {
        return Err(format!(
            "MRU scene parent object {} was not found",
            new_parent_key
        ));
    }
    if new_parent_key != root_key
        && meshlib_scene_is_descendant(
            &input.objects,
            &object_index_by_key,
            new_parent_key,
            &input.object_key,
            root_key,
        )?
    {
        return Err(format!(
            "MRU scene object {} cannot be parented under its descendant {}",
            input.object_key, new_parent_key
        ));
    }

    let old_target_path = meshlib_scene_hierarchy_path_for_object(
        &input.objects,
        &object_index_by_key,
        object_index,
        root_key,
    )?;
    let new_parent_path = if new_parent_key == root_key {
        vec![root_key.to_string()]
    } else {
        let new_parent_index = *object_index_by_key
            .get(new_parent_key)
            .expect("new parent was validated");
        meshlib_scene_hierarchy_path_for_object(
            &input.objects,
            &object_index_by_key,
            new_parent_index,
            root_key,
        )?
    };
    let mut new_target_path = new_parent_path;
    new_target_path.push(input.object_key.clone());

    let mut objects = input.objects.clone();
    objects[object_index].parent_key = new_parent_key.to_string();
    for index in 0..objects.len() {
        if index == object_index
            || meshlib_scene_is_descendant(
                &input.objects,
                &object_index_by_key,
                &input.objects[index].object_key,
                &input.object_key,
                root_key,
            )?
        {
            let old_path = meshlib_scene_hierarchy_path_for_object(
                &input.objects,
                &object_index_by_key,
                index,
                root_key,
            )?;
            let suffix = old_path
                .strip_prefix(old_target_path.as_slice())
                .unwrap_or(&[]);
            let mut new_path = new_target_path.clone();
            new_path.extend(suffix.iter().cloned());
            objects[index].hierarchy_path = new_path;
            if objects[index].link.is_none() {
                objects[index].model_file = meshlib_model_file_from_hierarchy_path(
                    &objects[index].hierarchy_path,
                    &objects[index].model_extension,
                );
            }
        }
    }

    let scene_child_order = meshlib_scene_export_child_order(&objects, root_key);
    Ok(MeshlibSceneReparentResult {
        objects,
        scene_child_order,
    })
}

pub fn meshlib_set_scene_object_state(
    input: &MeshlibSceneObjectStateInput,
) -> Result<MeshlibSceneObjectStateResult, String> {
    if let Some(object_index) = input
        .objects
        .iter()
        .position(|object| object.object_key == input.object_key)
    {
        let mut objects = input.objects.clone();
        apply_scene_object_state(&mut objects[object_index], input);
        return Ok(MeshlibSceneObjectStateResult {
            objects,
            feature_objects: input.feature_objects.clone(),
        });
    }

    if let Some(feature_index) = input
        .feature_objects
        .iter()
        .position(|object| object.object_key == input.object_key)
    {
        let mut feature_objects = input.feature_objects.clone();
        apply_scene_feature_object_state(&mut feature_objects[feature_index], input);
        return Ok(MeshlibSceneObjectStateResult {
            objects: input.objects.clone(),
            feature_objects,
        });
    }

    Err(format!("MRU scene object {} was not found", input.object_key))
}

fn apply_scene_object_state(
    object: &mut MeshlibSceneExportObject,
    input: &MeshlibSceneObjectStateInput,
) {
    if let Some(visibility_mask) = input.visibility_mask {
        object.visibility_mask = visibility_mask;
    }
    if let Some(selected) = input.selected {
        object.selected = selected;
    }
    if let Some(locked) = input.locked {
        object.locked = locked;
    }
    if let Some(parent_locked) = input.parent_locked {
        object.parent_locked = parent_locked;
    }
}

fn apply_scene_feature_object_state(
    object: &mut MeshlibSceneFeatureObject,
    input: &MeshlibSceneObjectStateInput,
) {
    if let Some(visibility_mask) = input.visibility_mask {
        object.visibility_mask = visibility_mask;
    }
    if let Some(selected) = input.selected {
        object.selected = selected;
    }
    if let Some(locked) = input.locked {
        object.locked = locked;
    }
    if let Some(parent_locked) = input.parent_locked {
        object.parent_locked = parent_locked;
    }
}

pub fn meshlib_select_scene_objects(
    input: &MeshlibSceneSelectionInput,
) -> Result<MeshlibSceneSelectionResult, String> {
    let object_index_by_key = meshlib_scene_object_index_by_key(&input.objects)?;
    let feature_index_by_key = meshlib_scene_feature_object_index_by_key(&input.feature_objects)?;
    let mut target_keys = HashSet::with_capacity(input.object_keys.len());
    for key in &input.object_keys {
        if !object_index_by_key.contains_key(key) && !feature_index_by_key.contains_key(key) {
            return Err(format!("MRU scene object {key} was not found"));
        }
        target_keys.insert(key.as_str());
    }

    let mut objects = input.objects.clone();
    let mut feature_objects = input.feature_objects.clone();
    match input.mode {
        MeshlibSceneSelectionMode::SelectOne => {
            for object in &mut objects {
                object.selected = target_keys.contains(object.object_key.as_str());
            }
            for object in &mut feature_objects {
                object.selected = target_keys.contains(object.object_key.as_str());
            }
        }
        MeshlibSceneSelectionMode::Toggle => {
            for key in target_keys {
                if let Some(index) = object_index_by_key.get(key).copied() {
                    objects[index].selected = !objects[index].selected;
                } else {
                    let index = *feature_index_by_key
                        .get(key)
                        .expect("feature selection target was validated");
                    feature_objects[index].selected = !feature_objects[index].selected;
                }
            }
        }
    }

    let mut selected_object_keys = objects
        .iter()
        .filter(|object| object.selected)
        .map(|object| object.object_key.clone())
        .collect::<Vec<_>>();
    selected_object_keys.extend(
        feature_objects
            .iter()
            .filter(|object| object.selected)
            .map(|object| object.object_key.clone()),
    );
    Ok(MeshlibSceneSelectionResult {
        objects,
        feature_objects,
        selected_object_keys,
    })
}

pub fn meshlib_set_scene_feature_object_visualize_property(
    input: &MeshlibSceneFeatureVisualizePropertyInput,
) -> Result<MeshlibSceneFeatureVisualizePropertyResult, String> {
    let feature_index_by_key = meshlib_scene_feature_object_index_by_key(&input.feature_objects)?;
    let feature_index = *feature_index_by_key
        .get(&input.object_key)
        .ok_or_else(|| format!("MRU scene FeatureObject {} was not found", input.object_key))?;
    let mut feature_objects = input.feature_objects.clone();
    let feature_object = &mut feature_objects[feature_index];
    match &input.property {
        MeshlibSceneFeatureVisualizeProperty::Subfeatures => {
            feature_object.subfeature_visibility = input.viewport_mask;
        }
        MeshlibSceneFeatureVisualizeProperty::DetailsOnNameTag => {
            feature_object.details_on_name_tag = input.viewport_mask;
        }
        MeshlibSceneFeatureVisualizeProperty::Dimension(name) => {
            let Some(mask) = feature_object.dimension_visibility.get_mut(name) else {
                return Err(format!(
                    "MRU scene FeatureObject {} does not support dimension visualize property {}",
                    input.object_key, name
                ));
            };
            *mask = input.viewport_mask;
        }
    }
    Ok(MeshlibSceneFeatureVisualizePropertyResult { feature_objects })
}

pub fn meshlib_reorder_scene_children(
    input: &MeshlibSceneReorderInput,
) -> Result<MeshlibSceneReorderResult, String> {
    let root_key = if input.root_key.is_empty() {
        input
            .objects
            .iter()
            .find_map(|object| object.hierarchy_path.first().map(String::as_str))
            .unwrap_or("0_Root")
    } else {
        input.root_key.as_str()
    };
    let parent_key = if input.parent_key.is_empty() {
        root_key
    } else {
        input.parent_key.as_str()
    };
    let object_index_by_key = meshlib_scene_object_index_by_key(&input.objects)?;
    if parent_key != root_key && !object_index_by_key.contains_key(parent_key) {
        return Err(format!(
            "MRU scene parent object {parent_key} was not found"
        ));
    }

    let mut direct_child_indices = Vec::new();
    for (index, object) in input.objects.iter().enumerate() {
        if meshlib_direct_parent_key(object, root_key) == parent_key {
            direct_child_indices.push(index);
        }
    }
    if direct_child_indices.len() != input.ordered_child_keys.len() {
        return Err(format!(
            "MRU scene parent {parent_key} has {} direct children but {} keys were provided",
            direct_child_indices.len(),
            input.ordered_child_keys.len()
        ));
    }

    let mut seen = HashSet::with_capacity(input.ordered_child_keys.len());
    for key in &input.ordered_child_keys {
        if !seen.insert(key.as_str()) {
            return Err(format!("Duplicate MRU scene child key {key}"));
        }
    }
    let direct_child_keys = direct_child_indices
        .iter()
        .map(|index| input.objects[*index].object_key.as_str())
        .collect::<HashSet<_>>();
    for key in &input.ordered_child_keys {
        if !direct_child_keys.contains(key.as_str()) {
            return Err(format!(
                "MRU scene object {key} is not a direct child of {parent_key}"
            ));
        }
    }

    let mut ordered_children = Vec::with_capacity(input.ordered_child_keys.len());
    for key in &input.ordered_child_keys {
        let index = *object_index_by_key
            .get(key)
            .expect("ordered child key was validated");
        ordered_children.push(input.objects[index].clone());
    }

    let mut objects = Vec::with_capacity(input.objects.len());
    let mut inserted = false;
    for object in &input.objects {
        if meshlib_direct_parent_key(object, root_key) == parent_key {
            if !inserted {
                objects.extend(ordered_children.iter().cloned());
                inserted = true;
            }
        } else {
            objects.push(object.clone());
        }
    }
    if !inserted {
        objects.extend(ordered_children);
    }
    let scene_child_order = meshlib_scene_export_child_order(&objects, root_key);
    Ok(MeshlibSceneReorderResult {
        objects,
        scene_child_order,
    })
}
