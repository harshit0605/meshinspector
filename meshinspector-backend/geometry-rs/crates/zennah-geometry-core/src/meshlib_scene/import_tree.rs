use super::decode::*;
use super::export_validation::*;
use super::import_public::*;
use super::merge::*;
use super::*;

pub(super) fn validate_mru_format_version(root: &Value) -> Result<(), String> {
    if root
        .get("FormatVersion")
        .and_then(Value::as_f64)
        .is_some_and(|version| version >= 2.0)
    {
        return Err(
            "Unsupported version of scene file. Please update your application.".to_string(),
        );
    }
    Ok(())
}

pub(super) fn collect_scene_object_nodes(
    root: &Value,
    parent_dir: &str,
    parent_key: &str,
    hierarchy_path: &[String],
    mesh_nodes: &mut Vec<MeshlibSceneObjectNode>,
    group_nodes: &mut Vec<MeshlibSceneObjectNode>,
    line_nodes: &mut Vec<MeshlibSceneObjectNode>,
    point_nodes: &mut Vec<MeshlibSceneObjectNode>,
    distance_map_nodes: &mut Vec<MeshlibSceneObjectNode>,
    voxel_nodes: &mut Vec<MeshlibSceneObjectNode>,
    feature_nodes: &mut Vec<MeshlibSceneObjectNode>,
    scene_child_order: &mut Vec<MeshlibSceneChildOrder>,
) {
    let key = meshlib_value_string(root.get("Key"))
        .or_else(|| meshlib_value_string(root.get("Name")))
        .unwrap_or_default();
    let mut current_hierarchy_path = hierarchy_path.to_vec();
    if !key.is_empty() {
        current_hierarchy_path.push(key.clone());
    }

    if is_object_mesh(root) {
        mesh_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    } else if is_group_object(root) && !parent_key.is_empty() {
        group_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    } else if is_object_lines(root) {
        line_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    } else if is_object_points(root) {
        point_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    } else if is_object_distance_map(root) {
        distance_map_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    } else if is_object_voxels(root) {
        voxel_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    } else if is_feature_object(root) {
        feature_nodes.push(MeshlibSceneObjectNode {
            object: root.clone(),
            parent_dir: parent_dir.to_string(),
            parent_key: parent_key.to_string(),
            hierarchy_path: current_hierarchy_path.clone(),
        });
    }

    let child_parent_dir = join_mru_path(parent_dir, &key);
    let Some(children) = root.get("Children").and_then(Value::as_object) else {
        return;
    };
    let ordered_child_keys = ordered_child_keys(children);
    let child_object_keys = ordered_child_keys
        .iter()
        .filter_map(|child_key| children.get(child_key))
        .filter_map(|child| {
            meshlib_value_string(child.get("Key"))
                .or_else(|| meshlib_value_string(child.get("Name")))
        })
        .collect::<Vec<_>>();
    if !child_object_keys.is_empty() {
        scene_child_order.push(MeshlibSceneChildOrder {
            parent_key: key.clone(),
            child_keys: child_object_keys,
        });
    }
    for child_key in ordered_child_keys {
        if let Some(child) = children.get(&child_key) {
            collect_scene_object_nodes(
                child,
                &child_parent_dir,
                &key,
                &current_hierarchy_path,
                mesh_nodes,
                group_nodes,
                line_nodes,
                point_nodes,
                distance_map_nodes,
                voxel_nodes,
                feature_nodes,
                scene_child_order,
            );
        }
    }
}

pub(super) fn is_object_mesh(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names
                .iter()
                .any(|name| name.as_str().is_some_and(|name| name == "ObjectMesh"))
        })
}

pub(super) fn is_group_object(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names.len() == 1 && type_names[0].as_str().is_some_and(|name| name == "Object")
        })
}

pub(super) fn is_object_lines(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names
                .iter()
                .any(|name| name.as_str().is_some_and(|name| name == "ObjectLines"))
        })
}

pub(super) fn is_object_points(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names
                .iter()
                .any(|name| name.as_str().is_some_and(|name| name == "ObjectPoints"))
        })
}

pub(super) fn is_object_distance_map(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names.iter().any(|name| {
                name.as_str()
                    .is_some_and(|name| name == "ObjectDistanceMap")
            })
        })
}

pub(super) fn is_object_voxels(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names
                .iter()
                .any(|name| name.as_str().is_some_and(|name| name == "ObjectVoxels"))
        })
}

pub(super) fn is_feature_object(root: &Value) -> bool {
    root.get("Type")
        .and_then(Value::as_array)
        .is_some_and(|type_names| {
            type_names
                .iter()
                .any(|name| name.as_str().is_some_and(|name| name == "FeatureObject"))
        })
}

pub(super) fn ordered_child_keys(children: &Map<String, Value>) -> Vec<String> {
    let mut numeric_keys = Vec::new();
    let mut string_keys = Vec::new();
    for key in children.keys() {
        match key.parse::<i64>() {
            Ok(value) => numeric_keys.push((value, key.clone())),
            Err(_) => string_keys.push(key.clone()),
        }
    }
    numeric_keys.sort_by_key(|(value, _)| *value);
    numeric_keys
        .into_iter()
        .map(|(_, key)| key)
        .chain(string_keys)
        .collect()
}

pub(super) fn scene_object_document_from_node<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    node: MeshlibSceneObjectNode,
    object_index: usize,
    model_cache: &mut HashMap<String, CachedParsedModelMesh>,
) -> Result<SceneObjectMeshDocument, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "Object".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let link = meshlib_value_string(object.get("Link")).filter(|link| !link.is_empty());
    let model_prefix = link
        .as_deref()
        .map(normalize_zip_name)
        .unwrap_or_else(|| join_mru_path(&node.parent_dir, &object_key));
    let model_file = find_model_file(archive, &model_prefix)?;
    let model_extension = model_file
        .rsplit_once('.')
        .map(|(_, extension)| format!(".{extension}"))
        .unwrap_or_default();
    let (model, shared_model_source_index) = if let Some(cached) = model_cache.get(&model_file) {
        (cached.model.clone(), Some(cached.source_object_index))
    } else {
        let model_bytes = read_zip_file(archive, &model_file)?;
        let parsed = parse_scene_model_mesh(&model_extension, &model_bytes)?;
        model_cache.insert(
            model_file.clone(),
            CachedParsedModelMesh {
                model: parsed.clone(),
                source_object_index: object_index,
            },
        );
        (parsed, None)
    };

    let texture_per_face = decode_meshlib_i32_vector(object.get("TexturePerFace"));
    let mut document = SceneObjectMeshDocument {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        model_file,
        model_extension,
        link,
        shared_model_source_index,
        vertices: model.vertices,
        faces: model.faces,
        vertex_colors: model.vertex_colors,
        face_colors: model.face_colors,
        vertex_uvs: model.vertex_uvs,
        vertex_normals: model.vertex_normals,
        tri_corner_uvs: model.tri_corner_uvs,
        edges: model.edges,
        texture_files: model.texture_files,
        texture_images: decode_meshlib_textures(object.get("Textures")),
        texture_per_face: if texture_per_face.is_empty() {
            model.texture_per_face
        } else {
            texture_per_face
        },
        object_names: model.object_names,
        material_names: model.material_names,
        diffuse_color: model.diffuse_color,
        meshlib_uv_coordinates: Vec::new(),
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    if document.texture_images.is_empty() {
        document.texture_images = model.texture_images;
    }
    apply_scene_uv_coordinates_to_object(
        &mut document,
        decode_meshlib_uv_vector(object.get("UVCoordinates")),
    );
    Ok(document)
}

pub(super) fn scene_line_object_from_node(
    node: MeshlibSceneObjectNode,
) -> Result<MeshlibSceneObjectLines, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "ObjectLines".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let polyline = object.get("Polyline");
    let points = decode_meshlib_polyline_points(polyline.and_then(|value| value.get("Points")))?;
    let lines = decode_meshlib_polyline_lines(
        polyline.and_then(|value| value.get("Lines")),
        points.len(),
        &object_key,
    )?;
    let line_object = MeshlibSceneObjectLines {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        points,
        lines,
        show_points: object
            .get("ShowPoints")
            .and_then(Value::as_u64)
            .map(|value| value as u32)
            .unwrap_or(0),
        smooth_connections: object
            .get("SmoothConnections")
            .and_then(Value::as_u64)
            .map(|value| value as u32)
            .unwrap_or(0),
        line_width: object
            .get("LineWidth")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32,
        coloring_type: object
            .get("ColoringType")
            .and_then(Value::as_str)
            .unwrap_or("Solid")
            .to_string(),
        line_colors: decode_meshlib_color_rows(object.get("LineColors")),
        vert_colors: decode_meshlib_color_rows(object.get("VertColors")),
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    meshlib_validate_scene_line_object(&line_object)?;
    Ok(line_object)
}

pub(super) fn scene_group_object_from_node(
    node: MeshlibSceneObjectNode,
) -> Result<MeshlibSceneGroupObject, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "Object".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    Ok(MeshlibSceneGroupObject {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub(super) fn scene_point_object_from_node<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    node: MeshlibSceneObjectNode,
) -> Result<MeshlibSceneObjectPoints, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "ObjectPoints".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let link = meshlib_value_string(object.get("Link")).filter(|link| !link.is_empty());
    let model_prefix = link
        .as_deref()
        .map(normalize_zip_name)
        .unwrap_or_else(|| join_mru_path(&node.parent_dir, &object_key));
    let model_file = find_model_file(archive, &model_prefix)?;
    let model_extension = model_file
        .rsplit_once('.')
        .map(|(_, extension)| format!(".{extension}"))
        .unwrap_or_default();
    let model_bytes = read_zip_file(archive, &model_file)?;
    let (points, normals, vert_colors) = parse_scene_model_points(&model_extension, &model_bytes)?;
    let point_object = MeshlibSceneObjectPoints {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        model_file,
        model_extension,
        link,
        points,
        normals,
        vert_colors,
        point_size: object
            .get("PointSize")
            .and_then(Value::as_f64)
            .unwrap_or(5.0) as f32,
        max_rendering_points: object
            .get("MaxRenderingPoints")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    meshlib_validate_scene_point_object(&point_object)?;
    Ok(point_object)
}

pub(super) fn scene_distance_map_object_from_node<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    node: MeshlibSceneObjectNode,
) -> Result<MeshlibSceneObjectDistanceMap, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "ObjectDistanceMap".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let link = meshlib_value_string(object.get("Link")).filter(|link| !link.is_empty());
    let model_prefix = link
        .as_deref()
        .map(normalize_zip_name)
        .unwrap_or_else(|| join_mru_path(&node.parent_dir, &object_key));
    let model_file = find_model_file(archive, &model_prefix)?;
    let model_extension = model_file
        .rsplit_once('.')
        .map(|(_, extension)| format!(".{extension}"))
        .unwrap_or_default();
    let model_bytes = read_zip_file(archive, &model_file)?;
    let parsed = parse_scene_distance_map_model(&model_extension, &model_bytes)?;
    let (valid_count, min_value, max_value) = meshlib_distance_map_stats(&parsed.values);
    let distance_map_object = MeshlibSceneObjectDistanceMap {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        model_file,
        model_extension,
        link,
        width: parsed.width,
        height: parsed.height,
        values: parsed.values,
        valid_count,
        min_value,
        max_value,
        origin_world: meshlib_json_vec3(object.get("OriginWorld"), parsed.origin_world),
        pixel_x_vec: meshlib_json_vec3(object.get("PixelXVec"), parsed.pixel_x_vec),
        pixel_y_vec: meshlib_json_vec3(object.get("PixelYVec"), parsed.pixel_y_vec),
        depth_vec: meshlib_json_vec3(object.get("DepthVec"), parsed.depth_vec),
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    meshlib_validate_scene_distance_map_object(&distance_map_object)?;
    Ok(distance_map_object)
}

pub(super) fn scene_voxel_object_from_node<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    node: MeshlibSceneObjectNode,
) -> Result<MeshlibSceneObjectVoxels, String> {
    let object = node.object;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "ObjectVoxels".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let link = meshlib_value_string(object.get("Link")).filter(|link| !link.is_empty());
    let model_prefix = link
        .as_deref()
        .map(normalize_zip_name)
        .unwrap_or_else(|| join_mru_path(&node.parent_dir, &object_key));
    let model_file = find_voxel_model_file(archive, &model_prefix)?;
    let model_extension = model_file
        .rsplit_once('.')
        .map(|(_, extension)| format!(".{extension}"))
        .unwrap_or_default();
    let autoname = parse_meshlib_raw_voxel_autoname(&model_file);
    let dimensions = meshlib_json_usize_vec3(
        object.get("Dimensions"),
        autoname
            .as_ref()
            .map(|name| name.dimensions)
            .unwrap_or([0, 0, 0]),
    );
    let voxel_size = meshlib_json_f32_vec3(
        object.get("VoxelSize"),
        autoname
            .as_ref()
            .map(|name| name.voxel_size)
            .unwrap_or([1.0, 1.0, 1.0]),
    );
    let grid_level_set = autoname
        .as_ref()
        .map(|name| name.grid_level_set)
        .unwrap_or(false);
    let model_bytes = read_zip_file(archive, &model_file)?;
    let parsed = parse_scene_voxel_model(
        &model_extension,
        &model_bytes,
        dimensions,
        voxel_size,
        grid_level_set,
    )?;
    let preserved_model_bytes = if model_extension.eq_ignore_ascii_case(".vdb") {
        model_bytes
    } else {
        Vec::new()
    };
    let selected_voxels = meshlib_compact_bitset_indices(object.get("SelectionVoxels"))?;
    let voxel_object = MeshlibSceneObjectVoxels {
        object_name,
        object_key,
        parent_key: node.parent_key,
        hierarchy_path: node.hierarchy_path,
        model_file,
        model_extension,
        link,
        model_bytes: preserved_model_bytes,
        dimensions: parsed.dimensions,
        voxel_size: parsed.voxel_size,
        grid_level_set: parsed.grid_level_set,
        values: parsed.values,
        min_value: parsed.min_value,
        max_value: parsed.max_value,
        min_corner: meshlib_json_usize_vec3(object.get("MinCorner"), [0, 0, 0]),
        max_corner: meshlib_json_usize_vec3(object.get("MaxCorner"), parsed.dimensions),
        iso_value: object
            .get("IsoValue")
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or((parsed.min_value + parsed.max_value) * 0.5),
        dual_marching_cubes: object
            .get("DualMarchingCubes")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        selected_voxels,
        xf: meshlib_scene_xf_from_value(object.get("XF")),
        visibility_mask: meshlib_visibility_mask_from_value(object.get("Visibility")),
        selected: object
            .get("Selected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        locked: object
            .get("Locked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        parent_locked: object
            .get("ParentLocked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    meshlib_validate_scene_voxel_object(&voxel_object)?;
    Ok(voxel_object)
}

include!("import_tree/features.rs");
