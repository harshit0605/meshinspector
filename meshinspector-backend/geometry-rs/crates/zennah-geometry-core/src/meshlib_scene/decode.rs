use super::export_validation::*;
use super::import_public::*;
use super::voxel_gav::parse_meshlib_gav_voxel_model;
use super::voxel_vdb::parse_meshlib_vdb_voxel_model;
use super::*;

pub(super) fn decode_meshlib_polyline_points(
    value: Option<&Value>,
) -> Result<Vec<[f64; 3]>, String> {
    let Some(points) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    points
        .iter()
        .enumerate()
        .map(|(index, value)| {
            meshlib_json_vec3_result(value).map_err(|error| {
                format!("Invalid MRU ObjectLines Polyline.Points[{index}]: {error}")
            })
        })
        .collect()
}

pub(super) fn decode_meshlib_polyline_lines(
    value: Option<&Value>,
    point_count: usize,
    object_key: &str,
) -> Result<Vec<[usize; 2]>, String> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    if values.len() % 2 != 0 {
        return Err(format!(
            "MRU ObjectLines {object_key} Polyline.Lines must contain pairs of point indices"
        ));
    }
    let mut lines = Vec::with_capacity(values.len() / 2);
    for pair_index in 0..values.len() / 2 {
        let start = values[pair_index * 2].as_u64().ok_or_else(|| {
            format!(
                "MRU ObjectLines {object_key} Polyline.Lines[{}] must be an unsigned integer",
                pair_index * 2
            )
        })? as usize;
        let end = values[pair_index * 2 + 1].as_u64().ok_or_else(|| {
            format!(
                "MRU ObjectLines {object_key} Polyline.Lines[{}] must be an unsigned integer",
                pair_index * 2 + 1
            )
        })? as usize;
        if start >= point_count || end >= point_count {
            return Err(format!(
                "MRU ObjectLines {object_key} Polyline.Lines[{pair_index}] references a missing point"
            ));
        }
        lines.push([start, end]);
    }
    Ok(lines)
}

pub(super) fn meshlib_json_vec3_result(value: &Value) -> Result<[f64; 3], String> {
    if let Some(array) = value.as_array() {
        if array.len() != 3 {
            return Err("point array must have exactly three coordinates".to_string());
        }
        return Ok([
            array[0]
                .as_f64()
                .ok_or_else(|| "x coordinate must be numeric".to_string())?,
            array[1]
                .as_f64()
                .ok_or_else(|| "y coordinate must be numeric".to_string())?,
            array[2]
                .as_f64()
                .ok_or_else(|| "z coordinate must be numeric".to_string())?,
        ]);
    }
    if value.is_object() {
        return Ok([
            value
                .get("x")
                .and_then(Value::as_f64)
                .ok_or_else(|| "x coordinate must be numeric".to_string())?,
            value
                .get("y")
                .and_then(Value::as_f64)
                .ok_or_else(|| "y coordinate must be numeric".to_string())?,
            value
                .get("z")
                .and_then(Value::as_f64)
                .ok_or_else(|| "z coordinate must be numeric".to_string())?,
        ]);
    }
    Err("point must be a vector object or three-item array".to_string())
}

pub(super) fn decode_meshlib_color_rows(value: Option<&Value>) -> Vec<[u8; 4]> {
    let Some(rows) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            if let Some(array) = row.as_array() {
                if array.len() != 4 {
                    return None;
                }
                return Some([
                    array[0].as_u64()? as u8,
                    array[1].as_u64()? as u8,
                    array[2].as_u64()? as u8,
                    array[3].as_u64()? as u8,
                ]);
            }
            Some([
                meshlib_color_component(row.get("x"))?,
                meshlib_color_component(row.get("y"))?,
                meshlib_color_component(row.get("z"))?,
                meshlib_color_component(row.get("w"))?,
            ])
        })
        .collect()
}

pub(super) fn meshlib_color_component(value: Option<&Value>) -> Option<u8> {
    value
        .and_then(Value::as_u64)
        .map(|value| value.min(255) as u8)
}

pub(super) fn meshlib_visibility_mask_from_value(value: Option<&Value>) -> u32 {
    let Some(mask) = value.and_then(Value::as_u64).map(|value| value as u32) else {
        return VIEWPORT_MASK_ALL;
    };
    if mask == 1 {
        VIEWPORT_MASK_ALL
    } else {
        mask
    }
}

pub(super) fn parse_scene_model_mesh(
    extension: &str,
    model_bytes: &[u8],
) -> Result<ParsedModelMesh, String> {
    match extension.to_ascii_lowercase().as_str() {
        ".ply" => {
            let mesh = crate::mesh_from_ply(model_bytes)?;
            Ok(ParsedModelMesh {
                vertices: mesh.vertices,
                faces: mesh.faces,
                vertex_colors: mesh.vertex_colors,
                face_colors: mesh.face_colors,
                vertex_uvs: mesh.vertex_uvs,
                vertex_normals: mesh.vertex_normals,
                tri_corner_uvs: mesh.tri_corner_uvs,
                edges: mesh.edges,
                texture_files: mesh.texture_files,
                texture_images: mesh
                    .texture_images
                    .into_iter()
                    .map(|texture| MeshlibSceneTextureImage {
                        width: texture.width,
                        height: texture.height,
                        pixels_rgba: texture.pixels_rgba,
                        filter: texture.filter,
                        wrap: texture.wrap,
                    })
                    .collect(),
                texture_per_face: Vec::new(),
                object_names: Vec::new(),
                material_names: Vec::new(),
                diffuse_color: None,
            })
        }
        ".obj" => {
            let mesh = crate::mesh_from_obj(model_bytes)?;
            Ok(ParsedModelMesh {
                vertices: mesh.vertices,
                faces: mesh.faces,
                vertex_colors: Vec::new(),
                face_colors: Vec::new(),
                vertex_uvs: Vec::new(),
                vertex_normals: Vec::new(),
                tri_corner_uvs: mesh.tri_corner_uvs,
                edges: Vec::new(),
                texture_files: mesh.texture_files,
                texture_images: mesh
                    .texture_images
                    .into_iter()
                    .map(|texture| MeshlibSceneTextureImage {
                        width: texture.width,
                        height: texture.height,
                        pixels_rgba: texture.pixels_rgba,
                        filter: texture.filter,
                        wrap: texture.wrap,
                    })
                    .collect(),
                texture_per_face: mesh.texture_per_face,
                object_names: mesh.object_names,
                material_names: mesh.material_names,
                diffuse_color: mesh.diffuse_color,
            })
        }
        _ => Err(format!(
            "Unsupported MRU ObjectMesh model extension: {extension}"
        )),
    }
}

pub(super) fn parse_scene_model_points(
    extension: &str,
    model_bytes: &[u8],
) -> Result<(Vec<[f64; 3]>, Vec<[f64; 3]>, Vec<[u8; 4]>), String> {
    match extension.to_ascii_lowercase().as_str() {
        ".ply" => {
            let points = crate::mesh_from_ply(model_bytes)?;
            Ok((points.vertices, points.vertex_normals, points.vertex_colors))
        }
        _ => Err(format!(
            "Unsupported MRU ObjectPoints model extension: {extension}"
        )),
    }
}

pub(super) fn parse_scene_distance_map_model(
    extension: &str,
    model_bytes: &[u8],
) -> Result<ParsedDistanceMapModel, String> {
    match extension.to_ascii_lowercase().as_str() {
        ".raw" => parse_meshlib_raw_distance_map(
            model_bytes,
            0,
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ),
        ".mrdistancemap" => {
            if model_bytes.len() < 64 {
                return Err("MRU ObjectDistanceMap .mrdistancemap payload is truncated".to_string());
            }
            let origin_world = read_meshlib_f32_vec3(model_bytes, 0)?; // DistanceMapToWorld::orgPoint
            let pixel_x_vec = read_meshlib_f32_vec3(model_bytes, 12)?;
            let pixel_y_vec = read_meshlib_f32_vec3(model_bytes, 24)?;
            let depth_vec = read_meshlib_f32_vec3(model_bytes, 36)?;
            parse_meshlib_raw_distance_map(
                model_bytes,
                48,
                origin_world,
                pixel_x_vec,
                pixel_y_vec,
                depth_vec,
            )
        }
        _ => Err(format!(
            "Unsupported MRU ObjectDistanceMap model extension: {extension}"
        )),
    }
}

pub(super) fn parse_scene_voxel_model(
    extension: &str,
    model_bytes: &[u8],
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
    grid_level_set: bool,
) -> Result<ParsedVoxelModel, String> {
    match extension.to_ascii_lowercase().as_str() {
        ".raw" => {
            parse_meshlib_raw_voxel_model(model_bytes, dimensions, voxel_size, grid_level_set)
        }
        ".gav" => parse_meshlib_gav_voxel_model(model_bytes),
        ".vdb" => parse_meshlib_vdb_voxel_model(model_bytes, dimensions, voxel_size),
        _ => Err(format!(
            "Unsupported MRU ObjectVoxels model extension: {extension}"
        )),
    }
}

pub(super) fn parse_meshlib_raw_distance_map(
    model_bytes: &[u8],
    offset: usize,
    origin_world: [f64; 3],
    pixel_x_vec: [f64; 3],
    pixel_y_vec: [f64; 3],
    depth_vec: [f64; 3],
) -> Result<ParsedDistanceMapModel, String> {
    if model_bytes.len() < offset + 16 {
        return Err("MRU ObjectDistanceMap .raw payload is truncated".to_string());
    }
    let width = read_meshlib_u64(model_bytes, offset)? as usize;
    let height = read_meshlib_u64(model_bytes, offset + 8)? as usize;
    let value_count = width
        .checked_mul(height)
        .ok_or_else(|| "MRU ObjectDistanceMap dimensions overflow".to_string())?;
    let values_offset = offset + 16;
    let expected_len = values_offset
        .checked_add(
            value_count
                .checked_mul(4)
                .ok_or_else(|| "MRU ObjectDistanceMap value byte count overflows".to_string())?,
        )
        .ok_or_else(|| "MRU ObjectDistanceMap value byte count overflows".to_string())?;
    if model_bytes.len() != expected_len {
        return Err(format!(
            "MRU ObjectDistanceMap payload length mismatch: expected {expected_len} bytes, got {}",
            model_bytes.len()
        ));
    }
    let mut values = Vec::with_capacity(value_count);
    for chunk in model_bytes[values_offset..].chunks_exact(4) {
        values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(ParsedDistanceMapModel {
        width,
        height,
        values,
        origin_world,
        pixel_x_vec,
        pixel_y_vec,
        depth_vec,
    })
}

pub(super) fn parse_meshlib_raw_voxel_model(
    model_bytes: &[u8],
    dimensions: [usize; 3],
    voxel_size: [f32; 3],
    grid_level_set: bool,
) -> Result<ParsedVoxelModel, String> {
    if dimensions.iter().any(|dimension| *dimension == 0) {
        return Err("MRU ObjectVoxels dimensions must be positive".to_string());
    }
    let value_count = dimensions
        .iter()
        .try_fold(1usize, |product, dimension| product.checked_mul(*dimension))
        .ok_or_else(|| "MRU ObjectVoxels dimensions overflow".to_string())?;
    let expected_len = value_count
        .checked_mul(4)
        .ok_or_else(|| "MRU ObjectVoxels value byte count overflows".to_string())?;
    if model_bytes.len() != expected_len {
        return Err(format!(
            "MRU ObjectVoxels payload length mismatch: expected {expected_len} bytes, got {}",
            model_bytes.len()
        ));
    }
    let mut values = Vec::with_capacity(value_count);
    for chunk in model_bytes.chunks_exact(4) {
        values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    let (min_value, max_value) = meshlib_voxel_stats(&values);
    Ok(ParsedVoxelModel {
        dimensions,
        voxel_size,
        origin: [0, 0, 0],
        grid_level_set,
        active_mask_compressed: false,
        background_value: max_value,
        values,
        min_value,
        max_value,
    })
}

pub(super) fn read_meshlib_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let chunk = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| "MRU ObjectDistanceMap payload is truncated".to_string())?;
    Ok(u64::from_le_bytes([
        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
    ]))
}

pub(super) fn read_meshlib_f32_vec3(bytes: &[u8], offset: usize) -> Result<[f64; 3], String> {
    let mut values = [0.0; 3];
    for (index, value) in values.iter_mut().enumerate() {
        let start = offset + index * 4;
        let chunk = bytes
            .get(start..start + 4)
            .ok_or_else(|| "MRU ObjectDistanceMap DistanceMapToWorld is truncated".to_string())?;
        *value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64;
    }
    Ok(values)
}

pub(super) fn apply_scene_uv_coordinates_to_object(
    document: &mut SceneObjectMeshDocument,
    uv_coordinates: Vec<[f64; 2]>,
) {
    if uv_coordinates.is_empty() {
        return;
    }
    if uv_coordinates.len() == document.faces.len() * 3 {
        document.tri_corner_uvs = uv_coordinates
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
            .collect();
    } else if uv_coordinates.len() == document.vertices.len() {
        document.vertex_uvs = uv_coordinates;
    } else {
        document.meshlib_uv_coordinates = uv_coordinates;
    }
}

pub(super) fn meshlib_scene_xf_from_value(value: Option<&Value>) -> MeshlibSceneXf {
    let Some(value) = value else {
        return MeshlibSceneXf::identity();
    };
    let a = value.get("A");
    MeshlibSceneXf {
        row_x: meshlib_json_vec3(a.and_then(|value| value.get("rowX")), [1.0, 0.0, 0.0]),
        row_y: meshlib_json_vec3(a.and_then(|value| value.get("rowY")), [0.0, 1.0, 0.0]),
        row_z: meshlib_json_vec3(a.and_then(|value| value.get("rowZ")), [0.0, 0.0, 1.0]),
        b: meshlib_json_vec3(value.get("b"), [0.0, 0.0, 0.0]),
    }
}

pub(super) fn meshlib_json_vec3(value: Option<&Value>, default: [f64; 3]) -> [f64; 3] {
    let Some(value) = value else {
        return default;
    };
    if let Some(array) = value.as_array() {
        if array.len() == 3 {
            return [
                array[0].as_f64().unwrap_or(default[0]),
                array[1].as_f64().unwrap_or(default[1]),
                array[2].as_f64().unwrap_or(default[2]),
            ];
        }
    }
    [
        value.get("x").and_then(Value::as_f64).unwrap_or(default[0]),
        value.get("y").and_then(Value::as_f64).unwrap_or(default[1]),
        value.get("z").and_then(Value::as_f64).unwrap_or(default[2]),
    ]
}

pub(super) fn meshlib_json_usize_vec3(value: Option<&Value>, default: [usize; 3]) -> [usize; 3] {
    let Some(value) = value else {
        return default;
    };
    if let Some(array) = value.as_array() {
        if array.len() == 3 {
            return [
                array[0]
                    .as_u64()
                    .map(|value| value as usize)
                    .unwrap_or(default[0]),
                array[1]
                    .as_u64()
                    .map(|value| value as usize)
                    .unwrap_or(default[1]),
                array[2]
                    .as_u64()
                    .map(|value| value as usize)
                    .unwrap_or(default[2]),
            ];
        }
    }
    if let Some(scalar) = value.as_u64() {
        let scalar = scalar as usize;
        return [scalar, scalar, scalar];
    }
    [
        value
            .get("x")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(default[0]),
        value
            .get("y")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(default[1]),
        value
            .get("z")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(default[2]),
    ]
}

pub(super) fn meshlib_json_f32_vec3(value: Option<&Value>, default: [f32; 3]) -> [f32; 3] {
    let Some(value) = value else {
        return default;
    };
    if let Some(array) = value.as_array() {
        if array.len() == 3 {
            return [
                array[0]
                    .as_f64()
                    .map(|value| value as f32)
                    .unwrap_or(default[0]),
                array[1]
                    .as_f64()
                    .map(|value| value as f32)
                    .unwrap_or(default[1]),
                array[2]
                    .as_f64()
                    .map(|value| value as f32)
                    .unwrap_or(default[2]),
            ];
        }
    }
    if let Some(scalar) = value.as_f64() {
        let scalar = scalar as f32;
        return [scalar, scalar, scalar];
    }
    [
        value
            .get("x")
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or(default[0]),
        value
            .get("y")
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or(default[1]),
        value
            .get("z")
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or(default[2]),
    ]
}

pub(super) fn meshlib_json_vec4(value: Option<&Value>, default: [f64; 4]) -> [f64; 4] {
    let Some(value) = value else {
        return default;
    };
    [
        value.get("x").and_then(Value::as_f64).unwrap_or(default[0]),
        value.get("y").and_then(Value::as_f64).unwrap_or(default[1]),
        value.get("z").and_then(Value::as_f64).unwrap_or(default[2]),
        value.get("w").and_then(Value::as_f64).unwrap_or(default[3]),
    ]
}

pub(super) fn decode_meshlib_u32_map(value: Option<&Value>) -> HashMap<String, u32> {
    let Some(map) = value.and_then(Value::as_object) else {
        return HashMap::new();
    };
    map.iter()
        .filter_map(|(key, value)| value.as_u64().map(|value| (key.clone(), value as u32)))
        .collect()
}

pub(super) fn meshlib_feature_type_from_value(root: &Value) -> Option<String> {
    root.get("Type")
        .and_then(Value::as_array)
        .and_then(|type_names| {
            type_names.iter().find_map(|name| {
                let name = name.as_str()?;
                meshlib_is_supported_feature_type(name).then(|| name.to_string())
            })
        })
}

pub(super) fn meshlib_is_supported_feature_type(name: &str) -> bool {
    matches!(
        name,
        "PointObject"
            | "LineObject"
            | "PlaneObject"
            | "SphereObject"
            | "CircleObject"
            | "CylinderObject"
            | "ConeObject"
    )
}

pub(super) fn dot3(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    lhs[0] * rhs[0] + lhs[1] * rhs[1] + lhs[2] * rhs[2]
}

pub(super) fn invert3(matrix: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let [a, b, c] = matrix[0];
    let [d, e, f] = matrix[1];
    let [g, h, i] = matrix[2];
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() <= f64::EPSILON {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        [
            (e * i - f * h) * inv_det,
            (c * h - b * i) * inv_det,
            (b * f - c * e) * inv_det,
        ],
        [
            (f * g - d * i) * inv_det,
            (a * i - c * g) * inv_det,
            (c * d - a * f) * inv_det,
        ],
        [
            (d * h - e * g) * inv_det,
            (b * g - a * h) * inv_det,
            (a * e - b * d) * inv_det,
        ],
    ])
}
