pub(super) fn meshlib_reparent_root_key(input: &MeshlibSceneReparentInput) -> &str {
    if !input.root_key.is_empty() {
        return &input.root_key;
    }
    input
        .objects
        .iter()
        .find_map(|object| object.hierarchy_path.first().map(String::as_str))
        .unwrap_or("0_Root")
}

pub(super) fn meshlib_direct_parent_key<'a>(
    object: &'a MeshlibSceneExportObject,
    root_key: &'a str,
) -> &'a str {
    if object.parent_key.is_empty() {
        root_key
    } else {
        object.parent_key.as_str()
    }
}

pub(super) fn meshlib_scene_object_index_by_key(
    objects: &[MeshlibSceneExportObject],
) -> Result<HashMap<String, usize>, String> {
    let mut object_index_by_key = HashMap::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene object at index {index} has an empty key"
            ));
        }
        if object_index_by_key
            .insert(object.object_key.clone(), index)
            .is_some()
        {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    Ok(object_index_by_key)
}

pub(super) fn meshlib_scene_feature_object_index_by_key(
    objects: &[MeshlibSceneFeatureObject],
) -> Result<HashMap<String, usize>, String> {
    let mut object_index_by_key = HashMap::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene FeatureObject at index {index} has an empty key"
            ));
        }
        if object_index_by_key
            .insert(object.object_key.clone(), index)
            .is_some()
        {
            return Err(format!(
                "Duplicate MRU scene FeatureObject key {}",
                object.object_key
            ));
        }
    }
    Ok(object_index_by_key)
}

pub(super) fn meshlib_scene_is_descendant(
    objects: &[MeshlibSceneExportObject],
    object_index_by_key: &HashMap<String, usize>,
    candidate_key: &str,
    ancestor_key: &str,
    root_key: &str,
) -> Result<bool, String> {
    let mut current_key = candidate_key;
    let mut guard = 0usize;
    while current_key != root_key && !current_key.is_empty() {
        guard += 1;
        if guard > objects.len() {
            return Err("MRU scene object tree contains a parent cycle".to_string());
        }
        let Some(index) = object_index_by_key.get(current_key).copied() else {
            return Err(format!("MRU scene object {current_key} was not found"));
        };
        let parent_key = objects[index].parent_key.as_str();
        if parent_key == ancestor_key {
            return Ok(true);
        }
        if parent_key == root_key || parent_key.is_empty() {
            return Ok(false);
        }
        current_key = parent_key;
    }
    Ok(false)
}

pub(super) fn meshlib_scene_hierarchy_path_for_object(
    objects: &[MeshlibSceneExportObject],
    object_index_by_key: &HashMap<String, usize>,
    object_index: usize,
    root_key: &str,
) -> Result<Vec<String>, String> {
    let object = &objects[object_index];
    if !object.hierarchy_path.is_empty() {
        return Ok(object.hierarchy_path.clone());
    }
    let mut reversed = vec![object.object_key.clone()];
    let mut parent_key = object.parent_key.as_str();
    let mut guard = 0usize;
    while parent_key != root_key && !parent_key.is_empty() {
        guard += 1;
        if guard > objects.len() {
            return Err("MRU scene object tree contains a parent cycle".to_string());
        }
        reversed.push(parent_key.to_string());
        let Some(parent_index) = object_index_by_key.get(parent_key).copied() else {
            return Err(format!(
                "MRU scene parent object {parent_key} was not found"
            ));
        };
        parent_key = objects[parent_index].parent_key.as_str();
    }
    reversed.push(root_key.to_string());
    reversed.reverse();
    Ok(reversed)
}
