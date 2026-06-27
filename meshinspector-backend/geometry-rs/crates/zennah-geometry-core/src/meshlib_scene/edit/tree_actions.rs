pub fn meshlib_apply_scene_tree_ribbon_action(
    input: &MeshlibSceneTreeRibbonActionInput,
) -> Result<MeshlibSceneTreeRibbonActionResult, String> {
    let root_key = meshlib_scene_tree_root_key(input);
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
    let mut result = MeshlibSceneTreeRibbonActionResult {
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

    match input.action {
        MeshlibSceneRibbonAction::SelectAll => {
            for node in &nodes {
                meshlib_scene_tree_set_selected(&mut result, node, true);
                meshlib_scene_tree_set_visibility(&mut result, node, VIEWPORT_MASK_ALL);
                result.affected_object_keys.push(node.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::UnselectAll => {
            for node in &nodes {
                meshlib_scene_tree_set_selected(&mut result, node, false);
                result.affected_object_keys.push(node.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::ShowAll => {
            for node in &nodes {
                meshlib_scene_tree_set_visibility(&mut result, node, VIEWPORT_MASK_ALL);
                result.affected_object_keys.push(node.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::HideAll => {
            for node in &nodes {
                meshlib_scene_tree_set_visibility(&mut result, node, 0);
                result.affected_object_keys.push(node.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::ShowOnlyPrevious | MeshlibSceneRibbonAction::ShowOnlyNext => {
            let is_next = input.action == MeshlibSceneRibbonAction::ShowOnlyNext;
            if let Some(target_index) =
                meshlib_scene_tree_show_only_target_index(&nodes, &root_key, is_next)
            {
                let target_parent_key = nodes[target_index].parent_key.clone();
                for node in &nodes {
                    if node.selected {
                        meshlib_scene_tree_set_selected(&mut result, node, false);
                        result.affected_object_keys.push(node.object_key.clone());
                    }
                    if node.parent_key == target_parent_key {
                        meshlib_scene_tree_set_visibility(&mut result, node, 0);
                        result.affected_object_keys.push(node.object_key.clone());
                    }
                }
                let target = &nodes[target_index];
                meshlib_scene_tree_set_visibility(&mut result, target, VIEWPORT_MASK_ALL);
                meshlib_scene_tree_set_selected(&mut result, target, true);
                result.affected_object_keys.push(target.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::SortByName => {
            if input.line_objects.is_empty()
                && input.point_objects.is_empty()
                && input.distance_map_objects.is_empty()
                && input.voxel_objects.is_empty()
                && input.feature_objects.is_empty()
                && input.group_objects.is_empty()
            {
                result.objects =
                    meshlib_scene_sort_by_name(&input.objects, &object_index_by_key, &root_key)?;
                result.affected_object_keys = result
                    .objects
                    .iter()
                    .map(|object| object.object_key.clone())
                    .collect();
            } else {
                result.scene_child_order =
                    meshlib_scene_tree_sort_child_order(&nodes, &object_index_by_key, &root_key)?;
                result.affected_object_keys =
                    nodes.iter().map(|node| node.object_key.clone()).collect();
            }
        }
        MeshlibSceneRibbonAction::RemoveSelected => {
            let selected_keys = nodes
                .iter()
                .filter(|node| node.selected)
                .map(|node| node.object_key.clone())
                .collect::<Vec<_>>();
            let selected_key_set = selected_keys
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            let mut remove_keys = HashSet::new();
            for node in &nodes {
                let mut should_remove = selected_key_set.contains(node.object_key.as_str());
                if !should_remove {
                    for selected_key in &selected_keys {
                        if meshlib_scene_tree_is_descendant(
                            &nodes,
                            &object_index_by_key,
                            &node.object_key,
                            selected_key,
                            &root_key,
                        )? {
                            should_remove = true;
                            break;
                        }
                    }
                }
                if should_remove {
                    remove_keys.insert(node.object_key.clone());
                }
            }
            result.removed_object_keys = nodes
                .iter()
                .filter(|node| remove_keys.contains(&node.object_key))
                .map(|node| node.object_key.clone())
                .collect();
            result
                .objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result
                .group_objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result
                .line_objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result
                .point_objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result
                .distance_map_objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result
                .voxel_objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result
                .feature_objects
                .retain(|object| !remove_keys.contains(&object.object_key));
            result.affected_object_keys = result.removed_object_keys.clone();
        }
    }

    meshlib_scene_tree_complete_result(result, &root_key)
}

pub fn meshlib_rename_scene_tree_object(
    input: &MeshlibSceneTreeRenameInput,
) -> Result<MeshlibSceneTreeRenameResult, String> {
    let root_key = meshlib_scene_tree_root_key_from_parts(
        "",
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
    let node = nodes
        .iter()
        .find(|node| node.object_key == input.object_key)
        .ok_or_else(|| format!("MRU scene object {} was not found", input.object_key))?;
    let mut result = MeshlibSceneTreeRenameResult {
        objects: input.objects.clone(),
        group_objects: input.group_objects.clone(),
        line_objects: input.line_objects.clone(),
        point_objects: input.point_objects.clone(),
        distance_map_objects: input.distance_map_objects.clone(),
        voxel_objects: input.voxel_objects.clone(),
        feature_objects: input.feature_objects.clone(),
    };
    meshlib_scene_tree_set_name(&mut result, node, &input.object_name);
    Ok(result)
}
