use super::export_write::*;
use super::import_public::*;
use super::*;

pub(super) fn format_meshlib_number(value: f64) -> String {
    let mut text = format!("{value:.17}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.push('0');
    }
    text
}

pub(super) fn merge_scene_object_documents(
    root_file: String,
    root_key: String,
    objects: Vec<SceneObjectMeshDocument>,
    scene_group_objects: Vec<MeshlibSceneGroupObject>,
    scene_line_objects: Vec<MeshlibSceneObjectLines>,
    scene_point_objects: Vec<MeshlibSceneObjectPoints>,
    scene_distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    scene_voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    scene_feature_objects: Vec<MeshlibSceneFeatureObject>,
    scene_child_order: Vec<MeshlibSceneChildOrder>,
) -> MeshlibObjectMeshMruDocument {
    let first = objects.first().expect("objects is not empty");
    let first_object_name = first.object_name.clone();
    let first_object_key = first.object_key.clone();
    let first_model_file = first.model_file.clone();
    let first_model_extension = first.model_extension.clone();
    let first_diffuse_color = first.diffuse_color;
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut vertex_colors = Vec::new();
    let mut face_colors = Vec::new();
    let mut vertex_uvs = Vec::new();
    let mut vertex_normals = Vec::new();
    let mut tri_corner_uvs = Vec::new();
    let mut edges = Vec::new();
    let mut texture_files = Vec::new();
    let mut texture_images = Vec::new();
    let mut texture_per_face = Vec::new();
    let mut object_names = Vec::new();
    let mut material_names = Vec::new();
    let mut meshlib_uv_coordinates = Vec::new();
    let mut scene_objects = Vec::new();

    let mut any_vertex_colors = false;
    let mut all_vertex_colors = true;
    let mut any_face_colors = false;
    let mut all_face_colors = true;
    let mut any_vertex_uvs = false;
    let mut all_vertex_uvs = true;
    let mut any_vertex_normals = false;
    let mut all_vertex_normals = true;
    let mut any_tri_corner_uvs = false;
    let mut all_tri_corner_uvs = true;
    let mut any_texture_per_face = false;
    let mut all_texture_per_face = true;

    for object in objects {
        let vertex_start = vertices.len();
        let face_start = faces.len();
        let texture_offset = texture_images.len() as i64;
        vertices.extend(
            object
                .vertices
                .iter()
                .map(|vertex| object.xf.transform_point(*vertex)),
        );
        faces.extend(object.faces.iter().map(|face| {
            [
                face[0] + vertex_start as i64,
                face[1] + vertex_start as i64,
                face[2] + vertex_start as i64,
            ]
        }));
        edges.extend(
            object
                .edges
                .iter()
                .map(|edge| [edge[0] + vertex_start as i64, edge[1] + vertex_start as i64]),
        );

        any_vertex_colors |= !object.vertex_colors.is_empty();
        all_vertex_colors &= object.vertex_colors.len() == object.vertices.len();
        vertex_colors.extend(object.vertex_colors.iter().copied());
        any_face_colors |= !object.face_colors.is_empty();
        all_face_colors &= object.face_colors.len() == object.faces.len();
        face_colors.extend(object.face_colors.iter().copied());
        any_vertex_uvs |= !object.vertex_uvs.is_empty();
        all_vertex_uvs &= object.vertex_uvs.len() == object.vertices.len();
        vertex_uvs.extend(object.vertex_uvs.iter().copied());
        any_vertex_normals |= !object.vertex_normals.is_empty();
        all_vertex_normals &= object.vertex_normals.len() == object.vertices.len();
        vertex_normals.extend(object.vertex_normals.iter().copied());
        any_tri_corner_uvs |= !object.tri_corner_uvs.is_empty();
        all_tri_corner_uvs &= object.tri_corner_uvs.len() == object.faces.len();
        tri_corner_uvs.extend(object.tri_corner_uvs.iter().copied());
        any_texture_per_face |= !object.texture_per_face.is_empty();
        all_texture_per_face &= object.texture_per_face.len() == object.faces.len();
        texture_per_face.extend(object.texture_per_face.iter().map(|texture_id| {
            if *texture_id >= 0 {
                *texture_id + texture_offset
            } else {
                *texture_id
            }
        }));
        texture_files.extend(object.texture_files.iter().cloned());
        texture_images.extend(object.texture_images.iter().cloned());
        object_names.extend(object.object_names.iter().cloned());
        material_names.extend(object.material_names.iter().cloned());
        meshlib_uv_coordinates.extend(object.meshlib_uv_coordinates.iter().copied());

        scene_objects.push(MeshlibSceneObjectMesh {
            object_name: object.object_name,
            object_key: object.object_key,
            parent_key: object.parent_key,
            hierarchy_path: object.hierarchy_path,
            model_file: object.model_file,
            model_extension: object.model_extension,
            link: object.link,
            shared_model_source_index: object.shared_model_source_index,
            vertex_range: [vertex_start, vertices.len()],
            face_range: [face_start, faces.len()],
            xf: object.xf,
            visibility_mask: object.visibility_mask,
            selected: object.selected,
            locked: object.locked,
            parent_locked: object.parent_locked,
        });
    }

    if !any_vertex_colors || !all_vertex_colors {
        vertex_colors.clear();
    }
    if !any_face_colors || !all_face_colors {
        face_colors.clear();
    }
    if !any_vertex_uvs || !all_vertex_uvs {
        vertex_uvs.clear();
    }
    if !any_vertex_normals || !all_vertex_normals {
        vertex_normals.clear();
    }
    if !any_tri_corner_uvs || !all_tri_corner_uvs {
        tri_corner_uvs.clear();
    }
    if !any_texture_per_face || !all_texture_per_face {
        texture_per_face.clear();
    }

    MeshlibObjectMeshMruDocument {
        root_file,
        root_key,
        object_name: first_object_name,
        object_key: first_object_key,
        model_file: first_model_file,
        model_extension: first_model_extension,
        vertices,
        faces,
        vertex_colors,
        face_colors,
        vertex_uvs,
        vertex_normals,
        tri_corner_uvs,
        edges,
        texture_files,
        texture_images,
        texture_per_face,
        object_names,
        material_names,
        diffuse_color: first_diffuse_color,
        meshlib_uv_coordinates,
        scene_objects,
        scene_line_objects,
        scene_point_objects,
        scene_distance_map_objects,
        scene_voxel_objects,
        scene_feature_objects,
        scene_group_objects,
        scene_child_order,
    }
}

pub fn meshlib_object_mesh_scene_json(
    input: &MeshlibObjectMeshSceneInput,
) -> Result<String, String> {
    let payload = meshlib_object_mesh_scene_value(input);
    serde_json::to_string(&payload).map_err(|error| error.to_string())
}

pub(super) fn read_mru_root_json<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<(String, Vec<u8>), String> {
    let mut root_name = None;
    for index in 0..archive.len() {
        let entry_name = {
            let file = archive.by_index(index).map_err(|error| error.to_string())?;
            normalize_zip_name(file.name())
        };
        if !entry_name.ends_with('/')
            && !entry_name.contains('/')
            && entry_name.to_ascii_lowercase().ends_with(".json")
        {
            root_name = Some(entry_name);
            break;
        }
    }
    let root_name = root_name.ok_or_else(|| "No top-level MRU scene JSON found".to_string())?;
    let root_bytes = read_zip_file(archive, &root_name)?;
    Ok((root_name, root_bytes))
}

pub(super) fn read_zip_file<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut file = archive.by_name(name).map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

pub(super) fn find_first_object_mesh(root: &Value, parent_dir: &str) -> Option<(Value, String)> {
    let type_names = root.get("Type").and_then(Value::as_array)?;
    if type_names
        .iter()
        .any(|name| name.as_str().is_some_and(|name| name == "ObjectMesh"))
    {
        return Some((root.clone(), parent_dir.to_string()));
    }

    let key = meshlib_value_string(root.get("Key"))
        .or_else(|| meshlib_value_string(root.get("Name")))
        .unwrap_or_default();
    let child_parent_dir = join_mru_path(parent_dir, &key);
    let children = root.get("Children").and_then(Value::as_object)?;
    let mut numeric_keys = Vec::new();
    let mut string_keys = Vec::new();
    for key in children.keys() {
        match key.parse::<i64>() {
            Ok(value) => numeric_keys.push((value, key.clone())),
            Err(_) => string_keys.push(key.clone()),
        }
    }
    numeric_keys.sort_by_key(|(value, _)| *value);
    let ordered_keys = numeric_keys
        .into_iter()
        .map(|(_, key)| key)
        .chain(string_keys)
        .collect::<Vec<_>>();
    for key in ordered_keys {
        if let Some(child) = children.get(&key) {
            if let Some(found) = find_first_object_mesh(child, &child_parent_dir) {
                return Some(found);
            }
        }
    }
    None
}

pub(super) fn find_model_file<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    model_prefix: &str,
) -> Result<String, String> {
    let prefix = format!("{model_prefix}.");
    let mut candidates = Vec::new();
    for index in 0..archive.len() {
        let entry_name = {
            let file = archive.by_index(index).map_err(|error| error.to_string())?;
            normalize_zip_name(file.name())
        };
        if entry_name.starts_with(&prefix) && !entry_name.ends_with('/') {
            candidates.push(entry_name);
        }
    }
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| format!("No mesh file found: {model_prefix}"))
}

pub(super) fn find_voxel_model_file<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    model_prefix: &str,
) -> Result<String, String> {
    let prefix = format!("{model_prefix}.");
    let mut direct_candidates = Vec::new();
    for index in 0..archive.len() {
        let entry_name = {
            let file = archive.by_index(index).map_err(|error| error.to_string())?;
            normalize_zip_name(file.name())
        };
        if entry_name.starts_with(&prefix) && !entry_name.ends_with('/') {
            direct_candidates.push(entry_name);
        }
    }
    direct_candidates.sort();
    if let Some(candidate) = direct_candidates.into_iter().next() {
        return Ok(candidate);
    }

    let normalized_prefix = normalize_zip_name(model_prefix);
    let (parent_dir, object_file) = normalized_prefix
        .rsplit_once('/')
        .map_or(("", normalized_prefix.as_str()), |(parent, object)| {
            (parent, object)
        });
    let parent_prefix = if parent_dir.is_empty() {
        String::new()
    } else {
        format!("{parent_dir}/")
    };
    let expected_suffix = format!(" {object_file}.raw").to_ascii_lowercase();
    let mut raw_candidates = Vec::new();
    for index in 0..archive.len() {
        let entry_name = {
            let file = archive.by_index(index).map_err(|error| error.to_string())?;
            normalize_zip_name(file.name())
        };
        if entry_name.ends_with('/') || !entry_name.starts_with(&parent_prefix) {
            continue;
        }
        let file_name = &entry_name[parent_prefix.len()..];
        if file_name.contains('/') {
            continue;
        }
        let lower_file_name = file_name.to_ascii_lowercase();
        if lower_file_name.starts_with('w')
            && lower_file_name.ends_with(&expected_suffix)
            && parse_meshlib_raw_voxel_autoname(file_name).is_some()
        {
            raw_candidates.push(entry_name);
        }
    }
    raw_candidates.sort();
    raw_candidates
        .into_iter()
        .next()
        .ok_or_else(|| format!("No voxel file found: {model_prefix}"))
}

pub(super) fn parse_meshlib_raw_voxel_autoname(
    model_file: &str,
) -> Option<MeshlibRawVoxelAutoname> {
    let file_name = model_file.rsplit('/').next().unwrap_or(model_file);
    let (prefix, _) = file_name.rsplit_once("_F ")?;
    let fields = prefix.split('_').collect::<Vec<_>>();
    if fields.len() != 7 {
        return None;
    }
    let width = fields[0].strip_prefix('W')?.parse::<usize>().ok()?;
    let height = fields[1].strip_prefix('H')?.parse::<usize>().ok()?;
    let depth = fields[2].strip_prefix('S')?.parse::<usize>().ok()?;
    let voxel_x = fields[3].strip_prefix('V')?.parse::<f32>().ok()? / 1000.0;
    let voxel_y = fields[4].parse::<f32>().ok()? / 1000.0;
    let voxel_z = fields[5].parse::<f32>().ok()? / 1000.0;
    let grid_level_set = fields[6].strip_prefix('G')?.parse::<u32>().ok()? != 0;
    Some(MeshlibRawVoxelAutoname {
        dimensions: [width, height, depth],
        voxel_size: [voxel_x, voxel_y, voxel_z],
        grid_level_set,
    })
}

pub(super) fn decode_meshlib_i32_vector(value: Option<&Value>) -> Vec<i64> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(data) = value.get("Data").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Ok(bytes) = STANDARD.decode(data) else {
        return Vec::new();
    };
    let count = value
        .get("Size")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(bytes.len() / 4);
    bytes
        .chunks_exact(4)
        .take(count)
        .map(|chunk| i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as i64)
        .collect()
}

pub(super) fn decode_meshlib_uv_vector(value: Option<&Value>) -> Vec<[f64; 2]> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Some(data) = value.get("Data").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Ok(bytes) = STANDARD.decode(data) else {
        return Vec::new();
    };
    let count = value
        .get("Size")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(bytes.len() / 8);
    bytes
        .chunks_exact(8)
        .take(count)
        .map(|chunk| {
            let u = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64;
            let v = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as f64;
            [u, v]
        })
        .collect()
}

pub(super) fn decode_meshlib_textures(value: Option<&Value>) -> Vec<MeshlibSceneTextureImage> {
    let Some(textures) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut numeric_keys = textures
        .keys()
        .filter_map(|key| key.parse::<usize>().ok().map(|index| (index, key.clone())))
        .collect::<Vec<_>>();
    numeric_keys.sort_by_key(|(index, _)| *index);
    let mut result = Vec::new();
    for (_, key) in numeric_keys {
        let Some(texture) = textures.get(&key) else {
            continue;
        };
        let Some(data) = texture.get("Data").and_then(Value::as_str) else {
            continue;
        };
        let Ok(bytes) = STANDARD.decode(data) else {
            continue;
        };
        let pixels_rgba = bytes
            .chunks_exact(4)
            .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
            .collect::<Vec<_>>();
        if pixels_rgba.is_empty() {
            continue;
        }
        let resolution = texture.get("Resolution");
        result.push(MeshlibSceneTextureImage {
            width: resolution
                .and_then(|value| value.get("x"))
                .and_then(Value::as_u64)
                .unwrap_or(pixels_rgba.len() as u64) as u32,
            height: resolution
                .and_then(|value| value.get("y"))
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32,
            pixels_rgba,
            filter: texture
                .get("FilterType")
                .and_then(Value::as_str)
                .unwrap_or("Linear")
                .to_string(),
            wrap: texture
                .get("WrapType")
                .and_then(Value::as_str)
                .unwrap_or("Clamp")
                .to_string(),
        });
    }
    result
}

pub(super) fn meshlib_value_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToOwned::to_owned)
}

pub(super) fn join_mru_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else if child.is_empty() {
        parent.to_string()
    } else {
        format!("{parent}/{child}")
    }
}

pub(super) fn normalize_zip_name(name: &str) -> String {
    name.replace('\\', "/")
}
