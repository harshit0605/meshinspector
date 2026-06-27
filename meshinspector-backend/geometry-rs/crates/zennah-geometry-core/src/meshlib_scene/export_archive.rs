use super::export_validation::*;
use super::export_values::*;
use super::export_write::*;
use super::*;

pub fn meshlib_multi_object_mru_scene_bytes(
    input: &MeshlibSceneExportInput,
) -> Result<Vec<u8>, String> {
    meshlib_multi_object_mru_scene_bytes_with_child_order(input, &[])
}

pub fn meshlib_multi_object_mru_scene_bytes_with_child_order(
    input: &MeshlibSceneExportInput,
    scene_child_order: &[MeshlibSceneChildOrder],
) -> Result<Vec<u8>, String> {
    if input.objects.is_empty()
        && input.group_objects.is_empty()
        && input.line_objects.is_empty()
        && input.point_objects.is_empty()
        && input.distance_map_objects.is_empty()
        && input.voxel_objects.is_empty()
        && input.feature_objects.is_empty()
    {
        return Err("MRU scene export requires at least one scene object".to_string());
    }
    let root_key = if input.root_key.is_empty() {
        meshlib_scene_key(&input.root_name, 0)
    } else {
        input.root_key.clone()
    };
    let mut root_payload = meshlib_multi_object_mru_scene_value(input, &root_key)?;
    let children_by_parent =
        meshlib_scene_export_children_by_parent(input, &root_key, scene_child_order)?;
    let mut mesh_visiting = vec![false; input.objects.len()];
    let mut mesh_visited = vec![false; input.objects.len()];
    let mut group_visiting = vec![false; input.group_objects.len()];
    let mut group_visited = vec![false; input.group_objects.len()];
    let mut line_visiting = vec![false; input.line_objects.len()];
    let mut line_visited = vec![false; input.line_objects.len()];
    let mut point_visiting = vec![false; input.point_objects.len()];
    let mut point_visited = vec![false; input.point_objects.len()];
    let mut distance_map_visiting = vec![false; input.distance_map_objects.len()];
    let mut distance_map_visited = vec![false; input.distance_map_objects.len()];
    let mut voxel_visiting = vec![false; input.voxel_objects.len()];
    let mut voxel_visited = vec![false; input.voxel_objects.len()];
    let mut feature_visiting = vec![false; input.feature_objects.len()];
    let mut feature_visited = vec![false; input.feature_objects.len()];
    let root_children = meshlib_export_scene_children(
        input,
        &root_key,
        &children_by_parent,
        &mut mesh_visiting,
        &mut mesh_visited,
        &mut group_visiting,
        &mut group_visited,
        &mut line_visiting,
        &mut line_visited,
        &mut point_visiting,
        &mut point_visited,
        &mut distance_map_visiting,
        &mut distance_map_visited,
        &mut voxel_visiting,
        &mut voxel_visited,
        &mut feature_visiting,
        &mut feature_visited,
    )?;
    root_payload["Children"] = Value::Object(root_children);
    if let Some((index, object)) = mesh_visited
        .iter()
        .zip(input.objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene object {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }
    if let Some((index, object)) = line_visited
        .iter()
        .zip(input.line_objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene ObjectLines {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }
    if let Some((index, object)) = group_visited
        .iter()
        .zip(input.group_objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene Object {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }
    if let Some((index, object)) = point_visited
        .iter()
        .zip(input.point_objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene ObjectPoints {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }
    if let Some((index, object)) = distance_map_visited
        .iter()
        .zip(input.distance_map_objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene ObjectDistanceMap {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }
    if let Some((index, object)) = feature_visited
        .iter()
        .zip(input.feature_objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene FeatureObject {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }
    if let Some((index, object)) = voxel_visited
        .iter()
        .zip(input.voxel_objects.iter())
        .enumerate()
        .find_map(|(index, (visited, object))| (!*visited).then_some((index, object)))
    {
        return Err(format!(
            "MRU scene ObjectVoxels {} at index {} is not reachable from root {}",
            object.object_key, index, root_key
        ));
    }

    let mut model_files = Vec::with_capacity(
        input.objects.len()
            + input.point_objects.len()
            + input.distance_map_objects.len()
            + input.voxel_objects.len(),
    );
    for object in &input.objects {
        let model_file = meshlib_export_model_file(&root_key, object);
        if object.shared_model_source_index.is_none() {
            model_files.push((
                model_file.clone(),
                meshlib_export_object_ply(input, object)?,
            ));
        }
    }
    for object in &input.point_objects {
        let model_file = meshlib_export_point_object_model_file(&root_key, object);
        model_files.push((model_file, meshlib_export_point_object_ply(object)?));
    }
    for object in &input.distance_map_objects {
        let model_file = meshlib_export_distance_map_object_model_file(&root_key, object);
        model_files.push((model_file, meshlib_export_distance_map_raw(object)?));
    }
    for object in &input.voxel_objects {
        let model_file = meshlib_export_voxel_object_model_file(&root_key, object);
        model_files.push((model_file, meshlib_export_voxel_object_model(object)?));
    }

    let root_json = serde_json::to_vec(&root_payload).map_err(|error| error.to_string())?;
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("Root.json", options)
        .map_err(|error| error.to_string())?;
    zip.write_all(&root_json)
        .map_err(|error| error.to_string())?;
    for (model_file, model_bytes) in model_files {
        zip.start_file(model_file, options)
            .map_err(|error| error.to_string())?;
        zip.write_all(&model_bytes)
            .map_err(|error| error.to_string())?;
    }
    let cursor = zip.finish().map_err(|error| error.to_string())?;
    Ok(cursor.into_inner())
}

include!("export_archive/tree.rs");
