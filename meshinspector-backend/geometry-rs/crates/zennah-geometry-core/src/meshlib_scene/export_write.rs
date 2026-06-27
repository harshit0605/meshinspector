use super::export_validation::*;
use super::merge::*;
use super::*;

pub(super) fn meshlib_vec3_value(point: [f64; 3]) -> Value {
    json!({"x": point[0], "y": point[1], "z": point[2]})
}

pub(super) fn meshlib_vec3f32_value(point: [f32; 3]) -> Value {
    json!({"x": point[0], "y": point[1], "z": point[2]})
}

pub(super) fn meshlib_vec3usize_value(point: [usize; 3]) -> Value {
    json!({"x": point[0], "y": point[1], "z": point[2]})
}

pub(super) fn meshlib_vec4_value(point: [f64; 4]) -> Value {
    json!({"x": point[0], "y": point[1], "z": point[2], "w": point[3]})
}

pub(super) fn meshlib_rgba_rows_value(colors: &[[u8; 4]]) -> Value {
    Value::Array(
        colors
            .iter()
            .map(|color| {
                json!({
                    "x": color[0],
                    "y": color[1],
                    "z": color[2],
                    "w": color[3],
                })
            })
            .collect(),
    )
}

pub(super) fn meshlib_export_model_file(
    root_key: &str,
    object: &MeshlibSceneExportObject,
) -> String {
    let extension = normalized_extension(&object.model_extension);
    if let Some(link) = object.link.as_ref() {
        return format!("{}{}", normalize_zip_name(link), extension);
    }
    if !object.model_file.is_empty() {
        return normalize_zip_name(&object.model_file);
    }
    if object.hierarchy_path.len() >= 2 {
        return meshlib_model_file_from_hierarchy_path(&object.hierarchy_path, &extension);
    }
    format!("{root_key}/{}{}", object.object_key, extension)
}

pub(super) fn meshlib_export_point_object_model_file(
    root_key: &str,
    object: &MeshlibSceneObjectPoints,
) -> String {
    let extension = normalized_point_model_extension(&object.model_extension);
    if let Some(link) = object.link.as_ref() {
        return format!("{}{}", normalize_zip_name(link), extension);
    }
    if !object.model_file.is_empty() {
        return normalize_zip_name(&object.model_file);
    }
    if object.hierarchy_path.len() >= 2 {
        return meshlib_model_file_from_hierarchy_path(&object.hierarchy_path, &extension);
    }
    format!("{root_key}/{}{}", object.object_key, extension)
}

pub(super) fn meshlib_export_distance_map_object_model_file(
    root_key: &str,
    object: &MeshlibSceneObjectDistanceMap,
) -> String {
    let extension = normalized_distance_map_model_extension(&object.model_extension);
    if let Some(link) = object.link.as_ref() {
        return format!("{}{}", normalize_zip_name(link), extension);
    }
    if !object.model_file.is_empty() {
        return normalize_zip_name(&object.model_file);
    }
    if object.hierarchy_path.len() >= 2 {
        return meshlib_model_file_from_hierarchy_path(&object.hierarchy_path, &extension);
    }
    format!("{root_key}/{}{}", object.object_key, extension)
}

pub(super) fn meshlib_export_voxel_object_model_file(
    root_key: &str,
    object: &MeshlibSceneObjectVoxels,
) -> String {
    let extension = normalized_voxel_model_extension(&object.model_extension);
    if let Some(link) = object.link.as_ref() {
        return format!("{}{}", normalize_zip_name(link), extension);
    }
    if !object.model_file.is_empty() {
        return normalize_zip_name(&object.model_file);
    }
    if extension.eq_ignore_ascii_case(".raw") {
        let file_name = meshlib_raw_voxel_model_filename(object);
        if object.hierarchy_path.len() >= 2 {
            let parent_path = object
                .hierarchy_path
                .iter()
                .take(object.hierarchy_path.len() - 1)
                .map(|part| normalize_zip_name(part))
                .collect::<Vec<_>>()
                .join("/");
            return join_mru_path(&parent_path, &file_name);
        }
        return format!("{root_key}/{file_name}");
    }
    if object.hierarchy_path.len() >= 2 {
        return meshlib_model_file_from_hierarchy_path(&object.hierarchy_path, &extension);
    }
    format!("{root_key}/{}{}", object.object_key, extension)
}

pub(super) fn meshlib_raw_voxel_model_filename(object: &MeshlibSceneObjectVoxels) -> String {
    let voxel_x = (object.voxel_size[0] * 1000.0).round() as i64;
    let voxel_y = (object.voxel_size[1] * 1000.0).round() as i64;
    let voxel_z = (object.voxel_size[2] * 1000.0).round() as i64;
    let grid_flag = if object.grid_level_set { 1 } else { 0 };
    format!(
        "W{}_H{}_S{}_V{}_{}_{}_G{}_F {}.raw",
        object.dimensions[0],
        object.dimensions[1],
        object.dimensions[2],
        voxel_x,
        voxel_y,
        voxel_z,
        grid_flag,
        object.object_key
    )
}

pub(super) fn meshlib_model_file_from_hierarchy_path(
    hierarchy_path: &[String],
    extension: &str,
) -> String {
    format!(
        "{}{}",
        hierarchy_path
            .iter()
            .map(|part| normalize_zip_name(part))
            .collect::<Vec<_>>()
            .join("/"),
        normalized_extension(extension)
    )
}

pub(super) fn meshlib_export_object_ply(
    input: &MeshlibSceneExportInput,
    object: &MeshlibSceneExportObject,
) -> Result<Vec<u8>, String> {
    let vertex_start = object.vertex_range[0];
    let vertex_end = object.vertex_range[1];
    let face_start = object.face_range[0];
    let face_end = object.face_range[1];
    if vertex_start > vertex_end || vertex_end > input.vertices.len() {
        return Err(format!(
            "Invalid vertex range for MRU object {}",
            object.object_key
        ));
    }
    if face_start > face_end || face_end > input.faces.len() {
        return Err(format!(
            "Invalid face range for MRU object {}",
            object.object_key
        ));
    }

    let vertices = input.vertices[vertex_start..vertex_end]
        .iter()
        .map(|vertex| object.xf.inverse_transform_point(*vertex))
        .collect::<Result<Vec<_>, _>>()?;
    let faces = input.faces[face_start..face_end]
        .iter()
        .map(|face| {
            if face
                .iter()
                .any(|index| *index < vertex_start as i64 || *index >= vertex_end as i64)
            {
                return Err(format!(
                    "Face references vertex outside object range for MRU object {}",
                    object.object_key
                ));
            }
            Ok([
                face[0] - vertex_start as i64,
                face[1] - vertex_start as i64,
                face[2] - vertex_start as i64,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut output = String::new();
    output.push_str("ply\nformat ascii 1.0\n");
    output.push_str(&format!("element vertex {}\n", vertices.len()));
    output.push_str("property double x\nproperty double y\nproperty double z\n");
    output.push_str(&format!("element face {}\n", faces.len()));
    output.push_str("property list uchar int vertex_indices\nend_header\n");
    for vertex in vertices {
        output.push_str(&format!(
            "{} {} {}\n",
            format_meshlib_number(vertex[0]),
            format_meshlib_number(vertex[1]),
            format_meshlib_number(vertex[2])
        ));
    }
    for face in faces {
        output.push_str(&format!("3 {} {} {}\n", face[0], face[1], face[2]));
    }
    Ok(output.into_bytes())
}

pub(super) fn meshlib_export_point_object_ply(
    object: &MeshlibSceneObjectPoints,
) -> Result<Vec<u8>, String> {
    meshlib_validate_scene_point_object(object)?;
    let has_normals = object.normals.len() == object.points.len();
    let has_colors = object.vert_colors.len() == object.points.len();
    let mut output = Vec::new();
    let mut header = String::new();
    header.push_str("ply\n");
    header.push_str("format binary_little_endian 1.0\n");
    header.push_str("comment MeshInspector.com\n");
    header.push_str(&format!("element vertex {}\n", object.points.len()));
    header.push_str("property float x\n");
    header.push_str("property float y\n");
    header.push_str("property float z\n");
    if has_normals {
        header.push_str("property float nx\n");
        header.push_str("property float ny\n");
        header.push_str("property float nz\n");
    }
    if has_colors {
        header.push_str("property uchar red\n");
        header.push_str("property uchar green\n");
        header.push_str("property uchar blue\n");
    }
    header.push_str("end_header\n");
    output.extend_from_slice(header.as_bytes());

    for (index, point) in object.points.iter().enumerate() {
        for coordinate in point {
            push_meshlib_point_f32(&mut output, *coordinate, &object.object_key)?;
        }
        if has_normals {
            for coordinate in &object.normals[index] {
                push_meshlib_point_f32(&mut output, *coordinate, &object.object_key)?;
            }
        }
        if has_colors {
            let color = object.vert_colors[index];
            output.extend_from_slice(&color[0..3]);
        }
    }
    Ok(output)
}

pub(super) fn meshlib_export_distance_map_raw(
    object: &MeshlibSceneObjectDistanceMap,
) -> Result<Vec<u8>, String> {
    meshlib_validate_scene_distance_map_object(object)?;
    let mut output = Vec::with_capacity(16 + object.values.len() * 4);
    output.extend_from_slice(&(object.width as u64).to_le_bytes());
    output.extend_from_slice(&(object.height as u64).to_le_bytes());
    for value in &object.values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    Ok(output)
}

pub(super) fn meshlib_export_voxel_object_raw(
    object: &MeshlibSceneObjectVoxels,
) -> Result<Vec<u8>, String> {
    meshlib_validate_scene_voxel_object(object)?;
    let mut output = Vec::with_capacity(object.values.len() * 4);
    for value in &object.values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    Ok(output)
}

pub(super) fn meshlib_export_voxel_object_model(
    object: &MeshlibSceneObjectVoxels,
) -> Result<Vec<u8>, String> {
    match normalized_voxel_model_extension(&object.model_extension)
        .to_ascii_lowercase()
        .as_str()
    {
        ".raw" => meshlib_export_voxel_object_raw(object),
        ".gav" => meshlib_export_voxel_object_gav(object),
        ".vdb" => meshlib_export_voxel_object_vdb(object),
        extension => Err(format!(
            "Unsupported MRU ObjectVoxels model extension: {extension}"
        )),
    }
}

pub(super) fn meshlib_export_voxel_object_vdb(
    object: &MeshlibSceneObjectVoxels,
) -> Result<Vec<u8>, String> {
    meshlib_validate_scene_voxel_object(object)?;
    Ok(object.model_bytes.clone())
}

pub(super) fn meshlib_export_voxel_object_gav(
    object: &MeshlibSceneObjectVoxels,
) -> Result<Vec<u8>, String> {
    meshlib_validate_scene_voxel_object(object)?;
    let (min_value, max_value) = meshlib_voxel_stats(&object.values);
    let header = json!({
        "ValueType": "Float",
        "Dimensions": {
            "X": object.dimensions[0],
            "Y": object.dimensions[1],
            "Z": object.dimensions[2],
        },
        "VoxelSize": {
            "X": object.voxel_size[0],
            "Y": object.voxel_size[1],
            "Z": object.voxel_size[2],
        },
        "Range": {
            "Min": min_value,
            "Max": max_value,
        },
    });
    let header_bytes = serde_json::to_vec(&header).map_err(|error| error.to_string())?;
    let header_len = u32::try_from(header_bytes.len())
        .map_err(|_| "Gav-header size overflows uint32".to_string())?;
    let mut output = Vec::with_capacity(4 + header_bytes.len() + object.values.len() * 4);
    output.extend_from_slice(&header_len.to_le_bytes());
    output.extend_from_slice(&header_bytes);
    output.extend_from_slice(&meshlib_export_voxel_object_raw(object)?);
    Ok(output)
}

pub(super) fn push_meshlib_point_f32(
    output: &mut Vec<u8>,
    value: f64,
    object_key: &str,
) -> Result<(), String> {
    let value = value as f32;
    if !value.is_finite() {
        return Err(format!(
            "MRU ObjectPoints {object_key} coordinate must fit MeshLib Vector3f"
        ));
    }
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

pub(super) fn meshlib_scene_xf_value(xf: MeshlibSceneXf) -> Value {
    json!({
        "A": {
            "rowX": {"x": xf.row_x[0], "y": xf.row_x[1], "z": xf.row_x[2]},
            "rowY": {"x": xf.row_y[0], "y": xf.row_y[1], "z": xf.row_y[2]},
            "rowZ": {"x": xf.row_z[0], "y": xf.row_z[1], "z": xf.row_z[2]},
        },
        "b": {"x": xf.b[0], "y": xf.b[1], "z": xf.b[2]},
    })
}

pub fn meshlib_object_mesh_mru_scene_value(input: &MeshlibObjectMeshSceneInput) -> Value {
    let root_key = meshlib_scene_key("Root", 0);
    let mut child = meshlib_object_mesh_scene_value(input);
    if let Some(object) = child.as_object_mut() {
        object.remove("FormatVersion");
        object.remove("ModelFile");
        object.insert(
            "meshlib_reference".to_string(),
            json!("MR::serializeObjectTree/ObjectMeshHolder::serializeFields_"),
        );
        object.insert(
            "meshlib_source".to_string(),
            json!(
                "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp"
            ),
        );
    }

    json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Visibility": VIEWPORT_MASK_ALL,
        "Selected": false,
        "Locked": false,
        "ParentLocked": false,
        "XF": {
            "A": {
                "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
            },
            "b": {"x": 0.0, "y": 0.0, "z": 0.0},
        },
        "Type": ["Object", "RootObject"],
        "Tags": [],
        "Key": root_key,
        "Children": {
            input.child_index.to_string(): child,
        },
        "meshlib_reference": "MR::serializeObjectTree",
        "meshlib_source": "MeshLib/source/MRMesh/MRObjectSave.cpp;MeshLib/source/MRMesh/MRObject.cpp",
        "meshlib_source_language": "rust",
    })
}

pub fn meshlib_object_mesh_scene_value(input: &MeshlibObjectMeshSceneInput) -> Value {
    let key = meshlib_scene_key(&input.object_name, input.child_index);
    let extension = normalized_extension(&input.model_extension);

    let mut textures = Map::new();
    for (texture_index, texture) in input.textures.iter().enumerate() {
        if texture.width == 0 || texture.height == 0 || texture.pixels_rgba.is_empty() {
            continue;
        }
        textures.insert(
            texture_index.to_string(),
            json!({
                "FilterType": texture.filter,
                "WrapType": texture.wrap,
                "Resolution": {"x": texture.width, "y": texture.height},
                "Data": base64_rgba_pixels(&texture.pixels_rgba),
            }),
        );
    }

    json!({
        "FormatVersion": 1.0,
        "meshlib_reference": "MR::ObjectMeshHolder::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
        "meshlib_source_language": "rust",
        "Key": key,
        "ModelFile": format!("{key}{extension}"),
        "Name": input.object_name,
        "Visibility": VIEWPORT_MASK_ALL,
        "Selected": false,
        "Locked": false,
        "ParentLocked": false,
        "XF": {
            "A": {
                "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0},
            },
            "b": {"x": 0.0, "y": 0.0, "z": 0.0},
        },
        "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"],
        "Tags": [],
        "Colors": {
            "Faces": {
                "SelectedMode": {"Diffuse": {"x": 0.8, "y": 0.8, "z": 0.8, "w": 1.0}},
                "UnselectedMode": {"Diffuse": {"x": 0.8, "y": 0.8, "z": 0.8, "w": 1.0}},
                "BackFaces": {"Diffuse": {"x": 0.8, "y": 0.8, "z": 0.8, "w": 1.0}},
            },
            "GlobalAlpha": 255,
            "Edges": {"x": 0.1, "y": 0.1, "z": 0.1, "w": 1.0},
            "Points": {"x": 0.1, "y": 0.1, "z": 0.1, "w": 1.0},
            "Borders": {"x": 0.1, "y": 0.1, "z": 0.1, "w": 1.0},
            "Selection": {"Diffuse": {"x": 1.0, "y": 0.7, "z": 0.0, "w": 1.0}},
        },
        "ShowName": 0,
        "UseDefaultSceneProperties": false,
        "ShowTexture": if textures.is_empty() { 0 } else { VIEWPORT_MASK_ALL },
        "ShowFaces": VIEWPORT_MASK_ALL,
        "ShowLines": 0,
        "ShowPoints": 0,
        "ShowBordersHighlight": 0,
        "ShowSelectedEdges": VIEWPORT_MASK_ALL,
        "ShowSelectedFaces": VIEWPORT_MASK_ALL,
        "OnlyOddFragments": 0,
        "PolygonOffset": 0,
        "ShadingEnabled": VIEWPORT_MASK_ALL,
        "FaceBased": false,
        "ColoringType": "Solid",
        "TextureCount": textures.len(),
        "Textures": textures,
        "TexturePerFace": meshlib_i32_vector(&input.texture_per_face),
        "UVCoordinates": meshlib_uv_vector(input),
        "SelectionFaceBitSet": {},
        "SelectionEdgeBitSet": {},
        "MeshCreasesUndirEdgeBitSet": {},
        "PointSize": 5.0,
    })
}

pub(super) fn normalized_extension(extension: &str) -> String {
    if extension.starts_with('.') {
        extension.to_owned()
    } else {
        format!(".{extension}")
    }
}

pub(super) fn normalized_point_model_extension(extension: &str) -> String {
    if extension.is_empty() {
        ".ply".to_string()
    } else {
        normalized_extension(extension)
    }
}

pub(super) fn normalized_distance_map_model_extension(extension: &str) -> String {
    if extension.is_empty() {
        ".raw".to_string()
    } else {
        normalized_extension(extension)
    }
}

pub(super) fn normalized_voxel_model_extension(extension: &str) -> String {
    if extension.is_empty() {
        ".raw".to_string()
    } else {
        normalized_extension(extension)
    }
}

pub(super) fn meshlib_i32_vector(values: &[i64]) -> Value {
    if values.is_empty() {
        return json!({});
    }
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<i32>());
    for value in values {
        bytes.extend_from_slice(&(*value as i32).to_le_bytes());
    }
    json!({
        "Size": values.len(),
        "Data": STANDARD.encode(bytes),
    })
}

pub(super) fn meshlib_uv_vector(input: &MeshlibObjectMeshSceneInput) -> Value {
    if !input.tri_corner_uvs.is_empty() {
        let mut values = Vec::with_capacity(input.tri_corner_uvs.len() * 3);
        for triangle in &input.tri_corner_uvs {
            values.extend_from_slice(triangle);
        }
        return meshlib_f32x2_vector(&values);
    }
    meshlib_f32x2_vector(&input.vertex_uvs)
}

pub(super) fn meshlib_f32x2_vector(values: &[[f64; 2]]) -> Value {
    if values.is_empty() {
        return json!({});
    }
    let mut bytes = Vec::with_capacity(values.len() * 2 * std::mem::size_of::<f32>());
    for [u, v] in values {
        bytes.extend_from_slice(&(*u as f32).to_le_bytes());
        bytes.extend_from_slice(&(*v as f32).to_le_bytes());
    }
    json!({
        "Size": values.len(),
        "Data": STANDARD.encode(bytes),
    })
}

pub(super) fn base64_rgba_pixels(pixels: &[[u8; 4]]) -> String {
    let mut bytes = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        bytes.extend_from_slice(pixel);
    }
    STANDARD.encode(bytes)
}
