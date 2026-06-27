pub fn meshlib_apply_scene_ribbon_action(
    input: &MeshlibSceneRibbonActionInput,
) -> Result<MeshlibSceneRibbonActionResult, String> {
    let root_key = meshlib_scene_root_key(&input.root_key, &input.objects);
    let object_index_by_key = meshlib_scene_object_index_by_key(&input.objects)?;
    let mut objects = input.objects.clone();
    let mut affected_object_keys = Vec::new();
    let mut removed_object_keys = Vec::new();

    match input.action {
        MeshlibSceneRibbonAction::SelectAll => {
            for object in &mut objects {
                object.selected = true;
                object.visibility_mask = VIEWPORT_MASK_ALL;
                affected_object_keys.push(object.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::UnselectAll => {
            for object in &mut objects {
                object.selected = false;
                affected_object_keys.push(object.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::ShowAll => {
            for object in &mut objects {
                object.visibility_mask = VIEWPORT_MASK_ALL;
                affected_object_keys.push(object.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::HideAll => {
            for object in &mut objects {
                object.visibility_mask = 0;
                affected_object_keys.push(object.object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::ShowOnlyPrevious | MeshlibSceneRibbonAction::ShowOnlyNext => {
            let is_next = input.action == MeshlibSceneRibbonAction::ShowOnlyNext;
            if let Some(target_index) =
                meshlib_scene_show_only_target_index(&input.objects, &root_key, is_next)
            {
                let target_parent_key =
                    meshlib_direct_parent_key(&input.objects[target_index], &root_key).to_string();
                for object in &mut objects {
                    if object.selected {
                        object.selected = false;
                        affected_object_keys.push(object.object_key.clone());
                    }
                    if meshlib_direct_parent_key(object, &root_key) == target_parent_key {
                        object.visibility_mask = 0;
                        affected_object_keys.push(object.object_key.clone());
                    }
                }
                objects[target_index].visibility_mask = VIEWPORT_MASK_ALL;
                objects[target_index].selected = true;
                affected_object_keys.push(objects[target_index].object_key.clone());
            }
        }
        MeshlibSceneRibbonAction::SortByName => {
            objects = meshlib_scene_sort_by_name(&input.objects, &object_index_by_key, &root_key)?;
            affected_object_keys = objects
                .iter()
                .map(|object| object.object_key.clone())
                .collect();
        }
        MeshlibSceneRibbonAction::RemoveSelected => {
            let selected_keys = input
                .objects
                .iter()
                .filter(|object| object.selected)
                .map(|object| object.object_key.clone())
                .collect::<Vec<_>>();
            let selected_key_set = selected_keys
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            let mut remove_indices = HashSet::new();
            for (index, object) in input.objects.iter().enumerate() {
                let mut should_remove = selected_key_set.contains(object.object_key.as_str());
                if !should_remove {
                    for selected_key in &selected_keys {
                        if meshlib_scene_is_descendant(
                            &input.objects,
                            &object_index_by_key,
                            &object.object_key,
                            selected_key,
                            &root_key,
                        )? {
                            should_remove = true;
                            break;
                        }
                    }
                }
                if should_remove {
                    remove_indices.insert(index);
                }
            }
            for (index, object) in input.objects.iter().enumerate() {
                if remove_indices.contains(&index) {
                    removed_object_keys.push(object.object_key.clone());
                }
            }
            objects = input
                .objects
                .iter()
                .enumerate()
                .filter(|(index, _)| !remove_indices.contains(index))
                .map(|(_, object)| object.clone())
                .collect();
            affected_object_keys = removed_object_keys.clone();
        }
    }

    Ok(meshlib_scene_ribbon_result(
        objects,
        affected_object_keys,
        removed_object_keys,
    ))
}

pub fn meshlib_rename_scene_object(
    input: &MeshlibSceneRenameInput,
) -> Result<MeshlibSceneRenameResult, String> {
    let object_index = input
        .objects
        .iter()
        .position(|object| object.object_key == input.object_key)
        .ok_or_else(|| format!("MRU scene object {} was not found", input.object_key))?;
    let mut objects = input.objects.clone();
    objects[object_index].object_name = input.object_name.clone();
    Ok(MeshlibSceneRenameResult { objects })
}

fn meshlib_scene_root_key(input_root_key: &str, objects: &[MeshlibSceneExportObject]) -> String {
    if !input_root_key.is_empty() {
        return input_root_key.to_string();
    }
    objects
        .iter()
        .find_map(|object| object.hierarchy_path.first().cloned())
        .unwrap_or_else(|| "0_Root".to_string())
}

fn meshlib_scene_ribbon_result(
    objects: Vec<MeshlibSceneExportObject>,
    affected_object_keys: Vec<String>,
    removed_object_keys: Vec<String>,
) -> MeshlibSceneRibbonActionResult {
    let selected_object_keys = objects
        .iter()
        .filter(|object| object.selected)
        .map(|object| object.object_key.clone())
        .collect();
    let visible_object_keys = objects
        .iter()
        .filter(|object| object.visibility_mask != 0)
        .map(|object| object.object_key.clone())
        .collect();
    let mut seen = HashSet::with_capacity(affected_object_keys.len());
    let affected_object_keys = affected_object_keys
        .into_iter()
        .filter(|key| seen.insert(key.clone()))
        .collect();
    MeshlibSceneRibbonActionResult {
        objects,
        affected_object_keys,
        selected_object_keys,
        visible_object_keys,
        removed_object_keys,
    }
}

fn meshlib_scene_show_only_target_index(
    objects: &[MeshlibSceneExportObject],
    root_key: &str,
    is_next: bool,
) -> Option<usize> {
    if objects.is_empty() {
        return None;
    }
    let Some(selected_index) = objects.iter().position(|object| object.selected) else {
        if is_next {
            return Some(0);
        }
        return objects
            .iter()
            .enumerate()
            .rev()
            .find(|(_, object)| meshlib_direct_parent_key(object, root_key) == root_key)
            .map(|(index, _)| index)
            .or(Some(objects.len() - 1));
    };

    let selected_parent_key = meshlib_direct_parent_key(&objects[selected_index], root_key);
    let sibling_indices = objects
        .iter()
        .enumerate()
        .filter(|(_, object)| meshlib_direct_parent_key(object, root_key) == selected_parent_key)
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

fn meshlib_scene_sort_by_name(
    objects: &[MeshlibSceneExportObject],
    object_index_by_key: &HashMap<String, usize>,
    root_key: &str,
) -> Result<Vec<MeshlibSceneExportObject>, String> {
    let mut children_by_parent: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, object) in objects.iter().enumerate() {
        let parent_key = meshlib_direct_parent_key(object, root_key).to_string();
        if parent_key != root_key && !object_index_by_key.contains_key(parent_key.as_str()) {
            return Err(format!(
                "MRU scene object {} references missing parent {}",
                object.object_key, parent_key
            ));
        }
        children_by_parent
            .entry(parent_key)
            .or_default()
            .push(index);
    }

    let mut sorted = Vec::with_capacity(objects.len());
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    meshlib_scene_collect_sorted_children(
        root_key,
        objects,
        &children_by_parent,
        &mut visiting,
        &mut visited,
        &mut sorted,
    )?;
    if sorted.len() != objects.len() {
        return Err("MRU scene object tree contains a parent cycle".to_string());
    }
    Ok(sorted)
}

fn meshlib_scene_collect_sorted_children(
    parent_key: &str,
    objects: &[MeshlibSceneExportObject],
    children_by_parent: &HashMap<String, Vec<usize>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    sorted: &mut Vec<MeshlibSceneExportObject>,
) -> Result<(), String> {
    let mut child_indices = children_by_parent
        .get(parent_key)
        .cloned()
        .unwrap_or_default();
    child_indices.sort_by(|left, right| {
        let left_object = &objects[*left];
        let right_object = &objects[*right];
        meshlib_case_insensitive_name_cmp(&left_object.object_name, &right_object.object_name)
            .then_with(|| left_object.object_key.cmp(&right_object.object_key))
    });

    for child_index in child_indices {
        let child = &objects[child_index];
        if !visiting.insert(child.object_key.clone()) {
            return Err("MRU scene object tree contains a parent cycle".to_string());
        }
        if visited.insert(child.object_key.clone()) {
            sorted.push(child.clone());
            meshlib_scene_collect_sorted_children(
                &child.object_key,
                objects,
                children_by_parent,
                visiting,
                visited,
                sorted,
            )?;
        }
        visiting.remove(&child.object_key);
    }
    Ok(())
}

fn meshlib_case_insensitive_name_cmp(left: &str, right: &str) -> Ordering {
    for (left_byte, right_byte) in left.bytes().zip(right.bytes()) {
        let left_lower = left_byte.to_ascii_lowercase();
        let right_lower = right_byte.to_ascii_lowercase();
        if left_lower != right_lower {
            return left_lower.cmp(&right_lower);
        }
    }
    left.len().cmp(&right.len())
}
