pub fn meshlib_group_scene_tree_objects(
    input: &MeshlibSceneTreeGroupInput,
) -> Result<MeshlibSceneTreeGroupResult, String> {
    let root_key = meshlib_scene_tree_root_key_from_parts(
        &input.root_key,
        &input.objects,
        &input.group_objects,
        &input.line_objects,
        &input.point_objects,
        &input.distance_map_objects,
        &input.voxel_objects,
        &input.feature_objects,
    );
    if input.group_key.is_empty() {
        return Err("MRU scene group key must not be empty".to_string());
    }
    if input.group_key == root_key {
        return Err(format!(
            "MRU scene group key {} conflicts with root",
            input.group_key
        ));
    }

    let nodes = meshlib_scene_tree_nodes(
        &root_key,
        &input.objects,
        &input.group_objects,
        &input.line_objects,
        &input.point_objects,
        &input.distance_map_objects,
        &input.voxel_objects,
        &input.feature_objects,
    )?;
    let object_index_by_key = meshlib_scene_tree_index_by_key(&nodes)?;
    if object_index_by_key.contains_key(&input.group_key) {
        return Err(format!(
            "MRU scene group key {} already exists",
            input.group_key
        ));
    }

    let selected_nodes = nodes
        .iter()
        .filter(|node| node.selected)
        .cloned()
        .collect::<Vec<_>>();
    if selected_nodes.len() < 2 {
        return Err("MRU scene grouping requires at least two selected objects".to_string());
    }
    let parent_key = selected_nodes[0].parent_key.clone();
    if selected_nodes
        .iter()
        .any(|node| node.parent_key != parent_key)
    {
        return Err("MRU scene grouping requires selected sibling objects".to_string());
    }

    let mut result = MeshlibSceneTreeGroupResult {
        objects: input.objects.clone(),
        group_objects: input.group_objects.clone(),
        line_objects: input.line_objects.clone(),
        point_objects: input.point_objects.clone(),
        distance_map_objects: input.distance_map_objects.clone(),
        voxel_objects: input.voxel_objects.clone(),
        feature_objects: input.feature_objects.clone(),
        affected_object_keys: selected_nodes
            .iter()
            .map(|node| node.object_key.clone())
            .chain(std::iter::once(input.group_key.clone()))
            .collect(),
        selected_object_keys: Vec::new(),
        visible_object_keys: Vec::new(),
        removed_object_keys: Vec::new(),
        scene_child_order: Vec::new(),
    };
    result.group_objects.push(MeshlibSceneGroupObject {
        object_name: "Group".to_string(),
        object_key: input.group_key.clone(),
        parent_key: parent_key.clone(),
        hierarchy_path: Vec::new(),
        xf: MeshlibSceneXf::identity(),
        visibility_mask: VIEWPORT_MASK_ALL,
        selected: false,
        locked: false,
        parent_locked: false,
    });
    for node in &selected_nodes {
        meshlib_scene_tree_group_result_set_parent(&mut result, node, &input.group_key);
    }
    meshlib_scene_tree_refresh_hierarchy_paths_for_group_result(&mut result, &root_key)?;

    let selected_keys = selected_nodes
        .iter()
        .map(|node| node.object_key.clone())
        .collect::<HashSet<_>>();
    let parent_children = nodes
        .iter()
        .filter(|node| node.parent_key == parent_key)
        .filter(|node| !selected_keys.contains(&node.object_key))
        .map(|node| node.object_key.clone())
        .chain(std::iter::once(input.group_key.clone()))
        .collect::<Vec<_>>();
    let group_children = selected_nodes
        .iter()
        .map(|node| node.object_key.clone())
        .collect::<Vec<_>>();
    result.scene_child_order = meshlib_scene_tree_child_order_from_group_parts(
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
        &root_key,
        &parent_key,
        parent_children,
        &input.group_key,
        group_children,
    )?;
    meshlib_scene_tree_finalize_group_result(result, &root_key)
}

pub fn meshlib_ungroup_scene_tree_objects(
    input: &MeshlibSceneTreeUngroupInput,
) -> Result<MeshlibSceneTreeUngroupResult, String> {
    let root_key = meshlib_scene_tree_root_key_from_parts(
        &input.root_key,
        &input.objects,
        &input.group_objects,
        &input.line_objects,
        &input.point_objects,
        &input.distance_map_objects,
        &input.voxel_objects,
        &input.feature_objects,
    );
    let nodes = meshlib_scene_tree_nodes(
        &root_key,
        &input.objects,
        &input.group_objects,
        &input.line_objects,
        &input.point_objects,
        &input.distance_map_objects,
        &input.voxel_objects,
        &input.feature_objects,
    )?;
    let selected_groups = nodes
        .iter()
        .filter(|node| node.kind == MeshlibSceneTreeNodeKind::Group && node.selected)
        .cloned()
        .collect::<Vec<_>>();
    if selected_groups.is_empty() {
        return Err("MRU scene ungroup requires a selected group object".to_string());
    }

    let mut result = MeshlibSceneTreeUngroupResult {
        objects: input.objects.clone(),
        group_objects: input.group_objects.clone(),
        line_objects: input.line_objects.clone(),
        point_objects: input.point_objects.clone(),
        distance_map_objects: input.distance_map_objects.clone(),
        voxel_objects: input.voxel_objects.clone(),
        feature_objects: input.feature_objects.clone(),
        affected_object_keys: Vec::new(),
        selected_object_keys: Vec::new(),
        visible_object_keys: Vec::new(),
        removed_object_keys: Vec::new(),
        scene_child_order: Vec::new(),
    };

    let mut group_keys_to_remove = HashSet::new();
    let mut explicit_orders: HashMap<String, Vec<String>> = HashMap::new();
    for group in &selected_groups {
        let child_keys = nodes
            .iter()
            .filter(|node| node.parent_key == group.object_key)
            .map(|node| node.object_key.clone())
            .collect::<Vec<_>>();
        if child_keys.is_empty() {
            continue;
        }
        let parent_key = group.parent_key.clone();
        let parent_children = explicit_orders
            .entry(parent_key.clone())
            .or_insert_with(|| {
                nodes
                    .iter()
                    .filter(|node| node.parent_key == parent_key)
                    .filter(|node| node.object_key != group.object_key)
                    .map(|node| node.object_key.clone())
                    .collect()
            });
        parent_children.extend(child_keys.iter().cloned());
        for child_key in &child_keys {
            if let Some(child_node) = nodes.iter().find(|node| node.object_key == *child_key) {
                meshlib_scene_tree_ungroup_result_set_parent(&mut result, child_node, &parent_key);
            }
        }
        group_keys_to_remove.insert(group.object_key.clone());
        result.removed_object_keys.push(group.object_key.clone());
        result.affected_object_keys.push(group.object_key.clone());
        result.affected_object_keys.extend(child_keys);
    }
    if group_keys_to_remove.is_empty() {
        return Err("MRU scene selected groups do not have children to ungroup".to_string());
    }
    result
        .group_objects
        .retain(|object| !group_keys_to_remove.contains(&object.object_key));
    meshlib_scene_tree_refresh_hierarchy_paths_for_ungroup_result(&mut result, &root_key)?;

    let current_nodes = meshlib_scene_tree_nodes(
        &root_key,
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
    )?;
    result.scene_child_order = meshlib_scene_tree_child_order(&current_nodes, &root_key);
    for (parent_key, child_keys) in explicit_orders {
        if let Some(order) = result
            .scene_child_order
            .iter_mut()
            .find(|order| order.parent_key == parent_key)
        {
            order.child_keys = child_keys;
        }
    }
    meshlib_scene_tree_finalize_ungroup_result(result, &root_key)
}
