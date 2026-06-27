#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeshlibSceneExportNodeRef {
    Mesh(usize),
    Group(usize),
    Lines(usize),
    Points(usize),
    DistanceMap(usize),
    Voxels(usize),
    Feature(usize),
}

pub(super) fn meshlib_scene_export_children_by_parent(
    input: &MeshlibSceneExportInput,
    root_key: &str,
    scene_child_order: &[MeshlibSceneChildOrder],
) -> Result<HashMap<String, Vec<MeshlibSceneExportNodeRef>>, String> {
    let mut object_keys = HashSet::with_capacity(
        input.objects.len()
            + input.group_objects.len()
            + input.line_objects.len()
            + input.point_objects.len()
            + input.distance_map_objects.len()
            + input.voxel_objects.len()
            + input.feature_objects.len(),
    );
    for (index, object) in input.objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene object at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    for (index, object) in input.group_objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene Object at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    for (index, object) in input.line_objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene ObjectLines at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    for (index, object) in input.point_objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene ObjectPoints at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    for (index, object) in input.distance_map_objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene ObjectDistanceMap at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    for (index, object) in input.feature_objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene FeatureObject at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }
    for (index, object) in input.voxel_objects.iter().enumerate() {
        if object.object_key.is_empty() {
            return Err(format!(
                "MRU scene ObjectVoxels at index {index} has an empty key"
            ));
        }
        if !object_keys.insert(object.object_key.clone()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                object.object_key
            ));
        }
    }

    let mut children_by_parent: HashMap<String, Vec<MeshlibSceneExportNodeRef>> = HashMap::new();
    for (index, object) in input.objects.iter().enumerate() {
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene object {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::Mesh(index));
    }
    for (index, object) in input.group_objects.iter().enumerate() {
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene Object {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::Group(index));
    }
    for (index, object) in input.line_objects.iter().enumerate() {
        meshlib_validate_scene_line_object(object)?;
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene ObjectLines {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::Lines(index));
    }
    for (index, object) in input.point_objects.iter().enumerate() {
        meshlib_validate_scene_point_object(object)?;
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene ObjectPoints {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::Points(index));
    }
    for (index, object) in input.distance_map_objects.iter().enumerate() {
        meshlib_validate_scene_distance_map_object(object)?;
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene ObjectDistanceMap {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::DistanceMap(index));
    }
    for (index, object) in input.voxel_objects.iter().enumerate() {
        meshlib_validate_scene_voxel_object(object)?;
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene ObjectVoxels {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::Voxels(index));
    }
    for (index, object) in input.feature_objects.iter().enumerate() {
        meshlib_validate_scene_feature_object(object)?;
        let parent_key = if object.parent_key.is_empty() {
            root_key
        } else {
            object.parent_key.as_str()
        };
        if parent_key != root_key && !object_keys.contains(parent_key) {
            return Err(format!(
                "MRU scene FeatureObject {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key.to_string())
            .or_default()
            .push(MeshlibSceneExportNodeRef::Feature(index));
    }
    meshlib_apply_scene_export_child_order(
        input,
        root_key,
        scene_child_order,
        &mut children_by_parent,
    )?;
    Ok(children_by_parent)
}

fn meshlib_apply_scene_export_child_order(
    input: &MeshlibSceneExportInput,
    root_key: &str,
    scene_child_order: &[MeshlibSceneChildOrder],
    children_by_parent: &mut HashMap<String, Vec<MeshlibSceneExportNodeRef>>,
) -> Result<(), String> {
    let mut seen_parent_keys = HashSet::new();
    for child_order in scene_child_order {
        let parent_key = if child_order.parent_key.is_empty() {
            root_key
        } else {
            child_order.parent_key.as_str()
        };
        if !seen_parent_keys.insert(parent_key.to_string()) {
            return Err(format!(
                "Duplicate MRU scene child order entry for parent {parent_key}"
            ));
        }
        let existing_children = children_by_parent
            .get(parent_key)
            .cloned()
            .ok_or_else(|| format!("MRU scene child order parent {parent_key} has no children"))?;
        if existing_children.len() != child_order.child_keys.len() {
            return Err(format!(
                "MRU scene child order for parent {parent_key} has {} keys but {} direct children exist",
                child_order.child_keys.len(),
                existing_children.len()
            ));
        }
        let mut children_by_key = HashMap::with_capacity(existing_children.len());
        for child_ref in existing_children {
            let child_key = meshlib_scene_export_node_key(input, child_ref).to_string();
            if children_by_key
                .insert(child_key.clone(), child_ref)
                .is_some()
            {
                return Err(format!("Duplicate MRU scene child key {child_key}"));
            }
        }
        let mut ordered_children = Vec::with_capacity(child_order.child_keys.len());
        let mut seen_child_keys = HashSet::with_capacity(child_order.child_keys.len());
        for child_key in &child_order.child_keys {
            if !seen_child_keys.insert(child_key.as_str()) {
                return Err(format!(
                    "Duplicate MRU scene child order key {child_key} under parent {parent_key}"
                ));
            }
            let Some(child_ref) = children_by_key.get(child_key).copied() else {
                return Err(format!(
                    "MRU scene child order key {child_key} is not a direct child of {parent_key}"
                ));
            };
            ordered_children.push(child_ref);
        }
        children_by_parent.insert(parent_key.to_string(), ordered_children);
    }
    Ok(())
}

fn meshlib_scene_export_node_key(
    input: &MeshlibSceneExportInput,
    node_ref: MeshlibSceneExportNodeRef,
) -> &str {
    match node_ref {
        MeshlibSceneExportNodeRef::Mesh(index) => &input.objects[index].object_key,
        MeshlibSceneExportNodeRef::Group(index) => &input.group_objects[index].object_key,
        MeshlibSceneExportNodeRef::Lines(index) => &input.line_objects[index].object_key,
        MeshlibSceneExportNodeRef::Points(index) => &input.point_objects[index].object_key,
        MeshlibSceneExportNodeRef::DistanceMap(index) => {
            &input.distance_map_objects[index].object_key
        }
        MeshlibSceneExportNodeRef::Voxels(index) => &input.voxel_objects[index].object_key,
        MeshlibSceneExportNodeRef::Feature(index) => &input.feature_objects[index].object_key,
    }
}

pub(super) fn meshlib_export_scene_children(
    input: &MeshlibSceneExportInput,
    parent_key: &str,
    children_by_parent: &HashMap<String, Vec<MeshlibSceneExportNodeRef>>,
    mesh_visiting: &mut [bool],
    mesh_visited: &mut [bool],
    group_visiting: &mut [bool],
    group_visited: &mut [bool],
    line_visiting: &mut [bool],
    line_visited: &mut [bool],
    point_visiting: &mut [bool],
    point_visited: &mut [bool],
    distance_map_visiting: &mut [bool],
    distance_map_visited: &mut [bool],
    voxel_visiting: &mut [bool],
    voxel_visited: &mut [bool],
    feature_visiting: &mut [bool],
    feature_visited: &mut [bool],
) -> Result<Map<String, Value>, String> {
    let mut children = Map::new();
    let Some(child_nodes) = children_by_parent.get(parent_key) else {
        return Ok(children);
    };
    for (child_ordinal, child_node) in child_nodes.iter().copied().enumerate() {
        children.insert(
            child_ordinal.to_string(),
            meshlib_export_scene_object_tree(
                input,
                child_node,
                children_by_parent,
                mesh_visiting,
                mesh_visited,
                group_visiting,
                group_visited,
                line_visiting,
                line_visited,
                point_visiting,
                point_visited,
                distance_map_visiting,
                distance_map_visited,
                voxel_visiting,
                voxel_visited,
                feature_visiting,
                feature_visited,
            )?,
        );
    }
    Ok(children)
}

pub(super) fn meshlib_export_scene_object_tree(
    input: &MeshlibSceneExportInput,
    object_ref: MeshlibSceneExportNodeRef,
    children_by_parent: &HashMap<String, Vec<MeshlibSceneExportNodeRef>>,
    mesh_visiting: &mut [bool],
    mesh_visited: &mut [bool],
    group_visiting: &mut [bool],
    group_visited: &mut [bool],
    line_visiting: &mut [bool],
    line_visited: &mut [bool],
    point_visiting: &mut [bool],
    point_visited: &mut [bool],
    distance_map_visiting: &mut [bool],
    distance_map_visited: &mut [bool],
    voxel_visiting: &mut [bool],
    voxel_visited: &mut [bool],
    feature_visiting: &mut [bool],
    feature_visited: &mut [bool],
) -> Result<Value, String> {
    let (object_key, mut value) = match object_ref {
        MeshlibSceneExportNodeRef::Mesh(object_index) => {
            if mesh_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.objects[object_index].object_key
                ));
            }
            mesh_visiting[object_index] = true;
            let object = &input.objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_object_scene_value(object),
            )
        }
        MeshlibSceneExportNodeRef::Group(object_index) => {
            if group_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.group_objects[object_index].object_key
                ));
            }
            group_visiting[object_index] = true;
            let object = &input.group_objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_group_object_scene_value(object),
            )
        }
        MeshlibSceneExportNodeRef::Lines(object_index) => {
            if line_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.line_objects[object_index].object_key
                ));
            }
            line_visiting[object_index] = true;
            let object = &input.line_objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_line_object_scene_value(object),
            )
        }
        MeshlibSceneExportNodeRef::Points(object_index) => {
            if point_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.point_objects[object_index].object_key
                ));
            }
            point_visiting[object_index] = true;
            let object = &input.point_objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_point_object_scene_value(object),
            )
        }
        MeshlibSceneExportNodeRef::DistanceMap(object_index) => {
            if distance_map_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.distance_map_objects[object_index].object_key
                ));
            }
            distance_map_visiting[object_index] = true;
            let object = &input.distance_map_objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_distance_map_object_scene_value(object),
            )
        }
        MeshlibSceneExportNodeRef::Voxels(object_index) => {
            if voxel_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.voxel_objects[object_index].object_key
                ));
            }
            voxel_visiting[object_index] = true;
            let object = &input.voxel_objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_voxel_object_scene_value(object),
            )
        }
        MeshlibSceneExportNodeRef::Feature(object_index) => {
            if feature_visiting[object_index] {
                return Err(format!(
                    "MRU scene object tree contains a parent cycle at {}",
                    input.feature_objects[object_index].object_key
                ));
            }
            feature_visiting[object_index] = true;
            let object = &input.feature_objects[object_index];
            (
                object.object_key.clone(),
                meshlib_export_feature_object_scene_value(object),
            )
        }
    };
    let children = meshlib_export_scene_children(
        input,
        &object_key,
        children_by_parent,
        mesh_visiting,
        mesh_visited,
        group_visiting,
        group_visited,
        line_visiting,
        line_visited,
        point_visiting,
        point_visited,
        distance_map_visiting,
        distance_map_visited,
        voxel_visiting,
        voxel_visited,
        feature_visiting,
        feature_visited,
    )?;
    if !children.is_empty() {
        value["Children"] = Value::Object(children);
    }
    match object_ref {
        MeshlibSceneExportNodeRef::Mesh(object_index) => {
            mesh_visiting[object_index] = false;
            mesh_visited[object_index] = true;
        }
        MeshlibSceneExportNodeRef::Group(object_index) => {
            group_visiting[object_index] = false;
            group_visited[object_index] = true;
        }
        MeshlibSceneExportNodeRef::Lines(object_index) => {
            line_visiting[object_index] = false;
            line_visited[object_index] = true;
        }
        MeshlibSceneExportNodeRef::Points(object_index) => {
            point_visiting[object_index] = false;
            point_visited[object_index] = true;
        }
        MeshlibSceneExportNodeRef::DistanceMap(object_index) => {
            distance_map_visiting[object_index] = false;
            distance_map_visited[object_index] = true;
        }
        MeshlibSceneExportNodeRef::Voxels(object_index) => {
            voxel_visiting[object_index] = false;
            voxel_visited[object_index] = true;
        }
        MeshlibSceneExportNodeRef::Feature(object_index) => {
            feature_visiting[object_index] = false;
            feature_visited[object_index] = true;
        }
    }
    Ok(value)
}
