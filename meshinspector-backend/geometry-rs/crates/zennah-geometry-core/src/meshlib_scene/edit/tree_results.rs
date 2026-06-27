fn meshlib_scene_tree_complete_result(
    mut result: MeshlibSceneTreeRibbonActionResult,
    root_key: &str,
) -> Result<MeshlibSceneTreeRibbonActionResult, String> {
    let nodes = meshlib_scene_tree_nodes(
        root_key,
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
    )?;
    result.selected_object_keys = nodes
        .iter()
        .filter(|node| node.selected)
        .map(|node| node.object_key.clone())
        .collect();
    result.visible_object_keys = nodes
        .iter()
        .filter(|node| node.visibility_mask != 0)
        .map(|node| node.object_key.clone())
        .collect();
    let mut seen = HashSet::with_capacity(result.affected_object_keys.len());
    result
        .affected_object_keys
        .retain(|key| seen.insert(key.clone()));
    if result.scene_child_order.is_empty() {
        result.scene_child_order = meshlib_scene_tree_child_order(&nodes, root_key);
    }
    Ok(result)
}

fn meshlib_scene_tree_finalize_group_result(
    mut result: MeshlibSceneTreeGroupResult,
    root_key: &str,
) -> Result<MeshlibSceneTreeGroupResult, String> {
    let nodes = meshlib_scene_tree_nodes(
        root_key,
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
    )?;
    result.selected_object_keys = nodes
        .iter()
        .filter(|node| node.selected)
        .map(|node| node.object_key.clone())
        .collect();
    result.visible_object_keys = nodes
        .iter()
        .filter(|node| node.visibility_mask != 0)
        .map(|node| node.object_key.clone())
        .collect();
    meshlib_dedup_scene_keys(&mut result.affected_object_keys);
    if result.scene_child_order.is_empty() {
        result.scene_child_order = meshlib_scene_tree_child_order(&nodes, root_key);
    }
    Ok(result)
}

fn meshlib_scene_tree_finalize_ungroup_result(
    mut result: MeshlibSceneTreeUngroupResult,
    root_key: &str,
) -> Result<MeshlibSceneTreeUngroupResult, String> {
    let nodes = meshlib_scene_tree_nodes(
        root_key,
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
    )?;
    result.selected_object_keys = nodes
        .iter()
        .filter(|node| node.selected)
        .map(|node| node.object_key.clone())
        .collect();
    result.visible_object_keys = nodes
        .iter()
        .filter(|node| node.visibility_mask != 0)
        .map(|node| node.object_key.clone())
        .collect();
    meshlib_dedup_scene_keys(&mut result.affected_object_keys);
    meshlib_dedup_scene_keys(&mut result.removed_object_keys);
    if result.scene_child_order.is_empty() {
        result.scene_child_order = meshlib_scene_tree_child_order(&nodes, root_key);
    }
    Ok(result)
}

fn meshlib_dedup_scene_keys(keys: &mut Vec<String>) {
    let mut seen = HashSet::with_capacity(keys.len());
    keys.retain(|key| seen.insert(key.clone()));
}

fn meshlib_scene_tree_group_result_set_parent(
    result: &mut MeshlibSceneTreeGroupResult,
    node: &MeshlibSceneTreeNode,
    parent_key: &str,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => {
            result.objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Group => {
            result.group_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Lines => {
            result.line_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Points => {
            result.point_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Voxels => {
            result.voxel_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Feature => {
            result.feature_objects[node.index].parent_key = parent_key.to_string()
        }
    }
}

fn meshlib_scene_tree_ungroup_result_set_parent(
    result: &mut MeshlibSceneTreeUngroupResult,
    node: &MeshlibSceneTreeNode,
    parent_key: &str,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => {
            result.objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Group => {
            result.group_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Lines => {
            result.line_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Points => {
            result.point_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Voxels => {
            result.voxel_objects[node.index].parent_key = parent_key.to_string()
        }
        MeshlibSceneTreeNodeKind::Feature => {
            result.feature_objects[node.index].parent_key = parent_key.to_string()
        }
    }
}

fn meshlib_scene_tree_refresh_hierarchy_paths_for_group_result(
    result: &mut MeshlibSceneTreeGroupResult,
    root_key: &str,
) -> Result<(), String> {
    let nodes = meshlib_scene_tree_nodes(
        root_key,
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
    )?;
    let index_by_key = meshlib_scene_tree_index_by_key(&nodes)?;
    for node in &nodes {
        let path =
            meshlib_scene_tree_hierarchy_path_for_node(&nodes, &index_by_key, node, root_key)?;
        meshlib_scene_tree_group_result_set_hierarchy_path(result, node, path);
    }
    Ok(())
}

fn meshlib_scene_tree_refresh_hierarchy_paths_for_ungroup_result(
    result: &mut MeshlibSceneTreeUngroupResult,
    root_key: &str,
) -> Result<(), String> {
    let nodes = meshlib_scene_tree_nodes(
        root_key,
        &result.objects,
        &result.group_objects,
        &result.line_objects,
        &result.point_objects,
        &result.distance_map_objects,
        &result.voxel_objects,
        &result.feature_objects,
    )?;
    let index_by_key = meshlib_scene_tree_index_by_key(&nodes)?;
    for node in &nodes {
        let path =
            meshlib_scene_tree_hierarchy_path_for_node(&nodes, &index_by_key, node, root_key)?;
        meshlib_scene_tree_ungroup_result_set_hierarchy_path(result, node, path);
    }
    Ok(())
}

fn meshlib_scene_tree_hierarchy_path_for_node(
    nodes: &[MeshlibSceneTreeNode],
    index_by_key: &HashMap<String, usize>,
    node: &MeshlibSceneTreeNode,
    root_key: &str,
) -> Result<Vec<String>, String> {
    let mut reversed = vec![node.object_key.clone()];
    let mut parent_key = node.parent_key.as_str();
    let mut guard = 0usize;
    while parent_key != root_key && !parent_key.is_empty() {
        guard += 1;
        if guard > nodes.len() {
            return Err("MRU scene object tree contains a parent cycle".to_string());
        }
        reversed.push(parent_key.to_string());
        let Some(parent_index) = index_by_key.get(parent_key).copied() else {
            return Err(format!(
                "MRU scene parent object {parent_key} was not found"
            ));
        };
        parent_key = nodes[parent_index].parent_key.as_str();
    }
    reversed.push(root_key.to_string());
    reversed.reverse();
    Ok(reversed)
}

fn meshlib_scene_tree_group_result_set_hierarchy_path(
    result: &mut MeshlibSceneTreeGroupResult,
    node: &MeshlibSceneTreeNode,
    hierarchy_path: Vec<String>,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => {
            result.objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Group => {
            result.group_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Lines => {
            result.line_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Points => {
            result.point_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Voxels => {
            result.voxel_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Feature => {
            result.feature_objects[node.index].hierarchy_path = hierarchy_path
        }
    }
}

fn meshlib_scene_tree_ungroup_result_set_hierarchy_path(
    result: &mut MeshlibSceneTreeUngroupResult,
    node: &MeshlibSceneTreeNode,
    hierarchy_path: Vec<String>,
) {
    match node.kind {
        MeshlibSceneTreeNodeKind::Mesh => {
            result.objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Group => {
            result.group_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Lines => {
            result.line_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Points => {
            result.point_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::DistanceMap => {
            result.distance_map_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Voxels => {
            result.voxel_objects[node.index].hierarchy_path = hierarchy_path
        }
        MeshlibSceneTreeNodeKind::Feature => {
            result.feature_objects[node.index].hierarchy_path = hierarchy_path
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn meshlib_scene_tree_child_order_from_group_parts(
    objects: &[MeshlibSceneExportObject],
    group_objects: &[MeshlibSceneGroupObject],
    line_objects: &[MeshlibSceneObjectLines],
    point_objects: &[MeshlibSceneObjectPoints],
    distance_map_objects: &[MeshlibSceneObjectDistanceMap],
    voxel_objects: &[MeshlibSceneObjectVoxels],
    feature_objects: &[MeshlibSceneFeatureObject],
    root_key: &str,
    parent_key: &str,
    parent_children: Vec<String>,
    group_key: &str,
    group_children: Vec<String>,
) -> Result<Vec<MeshlibSceneChildOrder>, String> {
    let nodes = meshlib_scene_tree_nodes(
        root_key,
        objects,
        group_objects,
        line_objects,
        point_objects,
        distance_map_objects,
        voxel_objects,
        feature_objects,
    )?;
    let mut child_order = meshlib_scene_tree_child_order(&nodes, root_key);
    if let Some(order) = child_order
        .iter_mut()
        .find(|order| order.parent_key == parent_key)
    {
        order.child_keys = parent_children.clone();
    }
    if let Some(order) = child_order
        .iter_mut()
        .find(|order| order.parent_key == group_key)
    {
        order.child_keys = group_children.clone();
    }
    let mut ordered = Vec::new();
    if !parent_children.is_empty() {
        ordered.push(MeshlibSceneChildOrder {
            parent_key: parent_key.to_string(),
            child_keys: parent_children,
        });
    }
    if !group_children.is_empty() {
        ordered.push(MeshlibSceneChildOrder {
            parent_key: group_key.to_string(),
            child_keys: group_children,
        });
    }
    ordered.extend(
        child_order
            .into_iter()
            .filter(|order| order.parent_key != parent_key && order.parent_key != group_key),
    );
    Ok(ordered)
}

fn meshlib_scene_tree_child_order(
    nodes: &[MeshlibSceneTreeNode],
    root_key: &str,
) -> Vec<MeshlibSceneChildOrder> {
    let mut child_order = Vec::new();
    for parent_key in meshlib_scene_tree_parent_order(nodes, root_key) {
        let child_keys = nodes
            .iter()
            .filter(|node| node.parent_key == parent_key)
            .map(|node| node.object_key.clone())
            .collect::<Vec<_>>();
        if !child_keys.is_empty() {
            child_order.push(MeshlibSceneChildOrder {
                parent_key,
                child_keys,
            });
        }
    }
    child_order
}

fn meshlib_scene_export_child_order(
    objects: &[MeshlibSceneExportObject],
    root_key: &str,
) -> Vec<MeshlibSceneChildOrder> {
    let mut parent_keys = Vec::new();
    let mut seen = HashSet::new();
    for object in objects {
        let parent_key = meshlib_direct_parent_key(object, root_key).to_string();
        if seen.insert(parent_key.clone()) {
            parent_keys.push(parent_key);
        }
    }
    if seen.insert(root_key.to_string()) {
        parent_keys.push(root_key.to_string());
    }

    let mut child_order = Vec::new();
    for parent_key in parent_keys {
        let child_keys = objects
            .iter()
            .filter(|object| meshlib_direct_parent_key(object, root_key) == parent_key)
            .map(|object| object.object_key.clone())
            .collect::<Vec<_>>();
        if !child_keys.is_empty() {
            child_order.push(MeshlibSceneChildOrder {
                parent_key,
                child_keys,
            });
        }
    }
    child_order
}

fn meshlib_scene_tree_parent_order(nodes: &[MeshlibSceneTreeNode], root_key: &str) -> Vec<String> {
    let mut parents = Vec::new();
    let mut seen = HashSet::new();
    for node in nodes {
        if seen.insert(node.parent_key.clone()) {
            parents.push(node.parent_key.clone());
        }
    }
    if seen.insert(root_key.to_string()) {
        parents.push(root_key.to_string());
    }
    parents
}

fn meshlib_scene_tree_sort_child_order(
    nodes: &[MeshlibSceneTreeNode],
    object_index_by_key: &HashMap<String, usize>,
    root_key: &str,
) -> Result<Vec<MeshlibSceneChildOrder>, String> {
    let mut children_by_parent: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.parent_key != root_key && !object_index_by_key.contains_key(&node.parent_key) {
            return Err(format!(
                "MRU scene object {} references missing parent {}",
                node.object_key, node.parent_key
            ));
        }
        children_by_parent
            .entry(node.parent_key.clone())
            .or_default()
            .push(index);
    }
    let mut child_order = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    meshlib_scene_tree_collect_sorted_child_order(
        root_key,
        nodes,
        &children_by_parent,
        &mut visiting,
        &mut visited,
        &mut child_order,
    )?;
    Ok(child_order)
}

fn meshlib_scene_tree_collect_sorted_child_order(
    parent_key: &str,
    nodes: &[MeshlibSceneTreeNode],
    children_by_parent: &HashMap<String, Vec<usize>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    child_order: &mut Vec<MeshlibSceneChildOrder>,
) -> Result<(), String> {
    let mut child_indices = children_by_parent
        .get(parent_key)
        .cloned()
        .unwrap_or_default();
    child_indices.sort_by(|left, right| {
        let left_node = &nodes[*left];
        let right_node = &nodes[*right];
        meshlib_case_insensitive_name_cmp(&left_node.object_name, &right_node.object_name)
            .then_with(|| left_node.object_key.cmp(&right_node.object_key))
    });

    for child_index in &child_indices {
        let child = &nodes[*child_index];
        if !visiting.insert(child.object_key.clone()) {
            return Err("MRU scene object tree contains a parent cycle".to_string());
        }
        if visited.insert(child.object_key.clone()) {
            meshlib_scene_tree_collect_sorted_child_order(
                &child.object_key,
                nodes,
                children_by_parent,
                visiting,
                visited,
                child_order,
            )?;
        }
        visiting.remove(&child.object_key);
    }

    if !child_indices.is_empty() {
        child_order.push(MeshlibSceneChildOrder {
            parent_key: parent_key.to_string(),
            child_keys: child_indices
                .into_iter()
                .map(|index| nodes[index].object_key.clone())
                .collect(),
        });
    }
    Ok(())
}

fn meshlib_scene_tree_show_only_target_index(
    nodes: &[MeshlibSceneTreeNode],
    root_key: &str,
    is_next: bool,
) -> Option<usize> {
    if nodes.is_empty() {
        return None;
    }
    let Some(selected_index) = nodes.iter().position(|node| node.selected) else {
        if is_next {
            return Some(0);
        }
        return nodes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, node)| node.parent_key == root_key)
            .map(|(index, _)| index)
            .or(Some(nodes.len() - 1));
    };
    let selected_parent_key = nodes[selected_index].parent_key.as_str();
    let sibling_indices = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent_key == selected_parent_key)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let current_position = sibling_indices
        .iter()
        .position(|index| *index == selected_index)
        .unwrap_or(0);
    let next_position = if is_next {
        (current_position + 1) % sibling_indices.len()
    } else {
        (current_position + sibling_indices.len() - 1) % sibling_indices.len()
    };
    sibling_indices.get(next_position).copied()
}

fn meshlib_scene_tree_is_descendant(
    nodes: &[MeshlibSceneTreeNode],
    object_index_by_key: &HashMap<String, usize>,
    candidate_key: &str,
    ancestor_key: &str,
    root_key: &str,
) -> Result<bool, String> {
    let mut current_key = candidate_key;
    let mut guard = 0usize;
    while current_key != root_key && !current_key.is_empty() {
        guard += 1;
        if guard > nodes.len() {
            return Err("MRU scene object tree contains a parent cycle".to_string());
        }
        let Some(index) = object_index_by_key.get(current_key).copied() else {
            return Err(format!("MRU scene object {current_key} was not found"));
        };
        let parent_key = nodes[index].parent_key.as_str();
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
