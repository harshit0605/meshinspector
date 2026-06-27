#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MeshlibSceneTreeNodeKind {
    Mesh,
    Group,
    Lines,
    Points,
    DistanceMap,
    Voxels,
    Feature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MeshlibSceneTreeNode {
    kind: MeshlibSceneTreeNodeKind,
    index: usize,
    object_key: String,
    object_name: String,
    parent_key: String,
    selected: bool,
    visibility_mask: u32,
}

fn meshlib_scene_tree_root_key(input: &MeshlibSceneTreeRibbonActionInput) -> String {
    meshlib_scene_tree_root_key_from_parts(
        &input.root_key,
        &input.objects,
        &input.group_objects,
        &input.line_objects,
        &input.point_objects,
        &input.distance_map_objects,
        &input.voxel_objects,
        &input.feature_objects,
    )
}

fn meshlib_scene_tree_root_key_from_parts(
    input_root_key: &str,
    objects: &[MeshlibSceneExportObject],
    group_objects: &[MeshlibSceneGroupObject],
    line_objects: &[MeshlibSceneObjectLines],
    point_objects: &[MeshlibSceneObjectPoints],
    distance_map_objects: &[MeshlibSceneObjectDistanceMap],
    voxel_objects: &[MeshlibSceneObjectVoxels],
    feature_objects: &[MeshlibSceneFeatureObject],
) -> String {
    if !input_root_key.is_empty() {
        return input_root_key.to_string();
    }
    objects
        .iter()
        .find_map(|object| object.hierarchy_path.first().cloned())
        .or_else(|| {
            group_objects
                .iter()
                .find_map(|object| object.hierarchy_path.first().cloned())
        })
        .or_else(|| {
            line_objects
                .iter()
                .find_map(|object| object.hierarchy_path.first().cloned())
        })
        .or_else(|| {
            point_objects
                .iter()
                .find_map(|object| object.hierarchy_path.first().cloned())
        })
        .or_else(|| {
            distance_map_objects
                .iter()
                .find_map(|object| object.hierarchy_path.first().cloned())
        })
        .or_else(|| {
            voxel_objects
                .iter()
                .find_map(|object| object.hierarchy_path.first().cloned())
        })
        .or_else(|| {
            feature_objects
                .iter()
                .find_map(|object| object.hierarchy_path.first().cloned())
        })
        .unwrap_or_else(|| "0_Root".to_string())
}

fn meshlib_scene_tree_nodes(
    root_key: &str,
    objects: &[MeshlibSceneExportObject],
    group_objects: &[MeshlibSceneGroupObject],
    line_objects: &[MeshlibSceneObjectLines],
    point_objects: &[MeshlibSceneObjectPoints],
    distance_map_objects: &[MeshlibSceneObjectDistanceMap],
    voxel_objects: &[MeshlibSceneObjectVoxels],
    feature_objects: &[MeshlibSceneFeatureObject],
) -> Result<Vec<MeshlibSceneTreeNode>, String> {
    let mut nodes = Vec::with_capacity(
        objects.len()
            + group_objects.len()
            + line_objects.len()
            + point_objects.len()
            + distance_map_objects.len()
            + voxel_objects.len()
            + feature_objects.len(),
    );
    for (index, object) in objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::Mesh,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    for (index, object) in group_objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::Group,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    for (index, object) in line_objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::Lines,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    for (index, object) in point_objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::Points,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    for (index, object) in distance_map_objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::DistanceMap,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    for (index, object) in voxel_objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::Voxels,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    for (index, object) in feature_objects.iter().enumerate() {
        nodes.push(meshlib_scene_tree_node(
            MeshlibSceneTreeNodeKind::Feature,
            index,
            &object.object_name,
            &object.object_key,
            &object.parent_key,
            root_key,
            object.selected,
            object.visibility_mask,
        ));
    }
    meshlib_scene_tree_validate_nodes(&nodes, root_key)?;
    Ok(nodes)
}

fn meshlib_scene_tree_node(
    kind: MeshlibSceneTreeNodeKind,
    index: usize,
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    root_key: &str,
    selected: bool,
    visibility_mask: u32,
) -> MeshlibSceneTreeNode {
    MeshlibSceneTreeNode {
        kind,
        index,
        object_key: object_key.to_string(),
        object_name: object_name.to_string(),
        parent_key: if parent_key.is_empty() {
            root_key.to_string()
        } else {
            parent_key.to_string()
        },
        selected,
        visibility_mask,
    }
}

fn meshlib_scene_tree_validate_nodes(
    nodes: &[MeshlibSceneTreeNode],
    root_key: &str,
) -> Result<(), String> {
    let mut keys = HashSet::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        if node.object_key.is_empty() {
            return Err(format!(
                "MRU scene object at flattened index {index} has an empty key"
            ));
        }
        if !keys.insert(node.object_key.as_str()) {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                node.object_key
            ));
        }
    }
    for node in nodes {
        if node.parent_key != root_key && !keys.contains(node.parent_key.as_str()) {
            return Err(format!(
                "MRU scene object {} references missing parent {}",
                node.object_key, node.parent_key
            ));
        }
    }
    Ok(())
}

fn meshlib_scene_tree_index_by_key(
    nodes: &[MeshlibSceneTreeNode],
) -> Result<HashMap<String, usize>, String> {
    let mut object_index_by_key = HashMap::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        if object_index_by_key
            .insert(node.object_key.clone(), index)
            .is_some()
        {
            return Err(format!(
                "Duplicate MRU scene object key {}",
                node.object_key
            ));
        }
    }
    Ok(object_index_by_key)
}

fn meshlib_scene_tree_set_selected(
    result: &mut MeshlibSceneTreeRibbonActionResult,
    node: &MeshlibSceneTreeNode,
    selected: bool,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => result.objects[node.index].selected = selected,
        MeshlibSceneTreeNodeKind::Group => result.group_objects[node.index].selected = selected,
        MeshlibSceneTreeNodeKind::Lines => result.line_objects[node.index].selected = selected,
        MeshlibSceneTreeNodeKind::Points => result.point_objects[node.index].selected = selected,
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].selected = selected
        }
        MeshlibSceneTreeNodeKind::Voxels => result.voxel_objects[node.index].selected = selected,
        MeshlibSceneTreeNodeKind::Feature => result.feature_objects[node.index].selected = selected,
    }
}

fn meshlib_scene_tree_set_visibility(
    result: &mut MeshlibSceneTreeRibbonActionResult,
    node: &MeshlibSceneTreeNode,
    visibility_mask: u32,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => {
            result.objects[node.index].visibility_mask = visibility_mask
        }
        MeshlibSceneTreeNodeKind::Group => {
            result.group_objects[node.index].visibility_mask = visibility_mask
        }
        MeshlibSceneTreeNodeKind::Lines => {
            result.line_objects[node.index].visibility_mask = visibility_mask
        }
        MeshlibSceneTreeNodeKind::Points => {
            result.point_objects[node.index].visibility_mask = visibility_mask
        }
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].visibility_mask = visibility_mask
        }
        MeshlibSceneTreeNodeKind::Voxels => {
            result.voxel_objects[node.index].visibility_mask = visibility_mask
        }
        MeshlibSceneTreeNodeKind::Feature => {
            result.feature_objects[node.index].visibility_mask = visibility_mask
        }
    }
}

fn meshlib_scene_tree_set_name(
    result: &mut MeshlibSceneTreeRenameResult,
    node: &MeshlibSceneTreeNode,
    object_name: &str,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => {
            result.objects[node.index].object_name = object_name.to_string()
        }
        MeshlibSceneTreeNodeKind::Group => {
            result.group_objects[node.index].object_name = object_name.to_string()
        }
        MeshlibSceneTreeNodeKind::Lines => {
            result.line_objects[node.index].object_name = object_name.to_string()
        }
        MeshlibSceneTreeNodeKind::Points => {
            result.point_objects[node.index].object_name = object_name.to_string()
        }
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].object_name = object_name.to_string()
        }
        MeshlibSceneTreeNodeKind::Voxels => {
            result.voxel_objects[node.index].object_name = object_name.to_string()
        }
        MeshlibSceneTreeNodeKind::Feature => {
            result.feature_objects[node.index].object_name = object_name.to_string()
        }
    }
}
