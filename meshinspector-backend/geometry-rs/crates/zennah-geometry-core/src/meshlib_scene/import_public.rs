use super::import_tree::*;
use super::merge::*;
use super::*;

pub fn meshlib_object_mesh_document_from_mru_scene_bytes(
    source: &[u8],
) -> Result<MeshlibObjectMeshMruDocument, String> {
    let mut archive = ZipArchive::new(Cursor::new(source)).map_err(|error| error.to_string())?;
    let (root_file, root_json) = read_mru_root_json(&mut archive)?;
    let root: Value = serde_json::from_slice(&root_json).map_err(|error| error.to_string())?;
    validate_mru_format_version(&root)?;

    let root_key = meshlib_value_string(root.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(root.get("Name")).unwrap_or_else(|| "Root".to_string())
    });
    let mut mesh_nodes = Vec::new();
    let mut group_nodes = Vec::new();
    let mut line_nodes = Vec::new();
    let mut point_nodes = Vec::new();
    let mut distance_map_nodes = Vec::new();
    let mut voxel_nodes = Vec::new();
    let mut feature_nodes = Vec::new();
    let mut scene_child_order = Vec::new();
    collect_scene_object_nodes(
        &root,
        "",
        "",
        &[],
        &mut mesh_nodes,
        &mut group_nodes,
        &mut line_nodes,
        &mut point_nodes,
        &mut distance_map_nodes,
        &mut voxel_nodes,
        &mut feature_nodes,
        &mut scene_child_order,
    );
    if mesh_nodes.is_empty() {
        return Err("No ObjectMesh node found in MRU scene".to_string());
    }

    let mut model_cache = HashMap::new();
    let mut objects = Vec::with_capacity(mesh_nodes.len());
    for (object_index, node) in mesh_nodes.into_iter().enumerate() {
        objects.push(scene_object_document_from_node(
            &mut archive,
            node,
            object_index,
            &mut model_cache,
        )?);
    }
    let mut line_objects = Vec::with_capacity(line_nodes.len());
    for node in line_nodes {
        line_objects.push(scene_line_object_from_node(node)?);
    }
    let mut group_objects = Vec::with_capacity(group_nodes.len());
    for node in group_nodes {
        group_objects.push(scene_group_object_from_node(node)?);
    }
    let mut point_objects = Vec::with_capacity(point_nodes.len());
    for node in point_nodes {
        point_objects.push(scene_point_object_from_node(&mut archive, node)?);
    }
    let mut distance_map_objects = Vec::with_capacity(distance_map_nodes.len());
    for node in distance_map_nodes {
        distance_map_objects.push(scene_distance_map_object_from_node(&mut archive, node)?);
    }
    let mut voxel_objects = Vec::with_capacity(voxel_nodes.len());
    for node in voxel_nodes {
        voxel_objects.push(scene_voxel_object_from_node(&mut archive, node)?);
    }
    let mut feature_objects = Vec::with_capacity(feature_nodes.len());
    for node in feature_nodes {
        feature_objects.push(scene_feature_object_from_node(node)?);
    }

    Ok(merge_scene_object_documents(
        root_file,
        root_key,
        objects,
        group_objects,
        line_objects,
        point_objects,
        distance_map_objects,
        voxel_objects,
        feature_objects,
        scene_child_order,
    ))
}

pub fn meshlib_object_mesh_from_mru_scene_bytes(
    source: &[u8],
) -> Result<MeshlibObjectMeshMruScene, String> {
    let mut archive = ZipArchive::new(Cursor::new(source)).map_err(|error| error.to_string())?;
    let (root_file, root_json) = read_mru_root_json(&mut archive)?;
    let root: Value = serde_json::from_slice(&root_json).map_err(|error| error.to_string())?;
    validate_mru_format_version(&root)?;

    let root_key = meshlib_value_string(root.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(root.get("Name")).unwrap_or_else(|| "Root".to_string())
    });
    let (object, parent_dir) = find_first_object_mesh(&root, "")
        .ok_or_else(|| "No ObjectMesh node found in MRU scene".to_string())?;
    let object_key = meshlib_value_string(object.get("Key")).unwrap_or_else(|| {
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| "Object".to_string())
    });
    let object_name =
        meshlib_value_string(object.get("Name")).unwrap_or_else(|| object_key.clone());
    let model_prefix = join_mru_path(&parent_dir, &object_key);
    let model_file = find_model_file(&mut archive, &model_prefix)?;
    let model_extension = model_file
        .rsplit_once('.')
        .map(|(_, extension)| format!(".{extension}"))
        .unwrap_or_default();
    let model_bytes = read_zip_file(&mut archive, &model_file)?;

    Ok(MeshlibObjectMeshMruScene {
        root_file,
        root_key,
        object_name,
        object_key,
        model_file,
        model_extension,
        model_bytes,
        texture_per_face: decode_meshlib_i32_vector(object.get("TexturePerFace")),
        uv_coordinates: decode_meshlib_uv_vector(object.get("UVCoordinates")),
        textures: decode_meshlib_textures(object.get("Textures")),
    })
}

#[derive(Debug, Clone)]
pub(super) struct MeshlibSceneObjectNode {
    pub(super) object: Value,
    pub(super) parent_dir: String,
    pub(super) parent_key: String,
    pub(super) hierarchy_path: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SceneObjectMeshDocument {
    pub(super) object_name: String,
    pub(super) object_key: String,
    pub(super) parent_key: String,
    pub(super) hierarchy_path: Vec<String>,
    pub(super) model_file: String,
    pub(super) model_extension: String,
    pub(super) link: Option<String>,
    pub(super) shared_model_source_index: Option<usize>,
    pub(super) vertices: Vec<[f64; 3]>,
    pub(super) faces: Vec<[i64; 3]>,
    pub(super) vertex_colors: Vec<[u8; 4]>,
    pub(super) face_colors: Vec<[u8; 4]>,
    pub(super) vertex_uvs: Vec<[f64; 2]>,
    pub(super) vertex_normals: Vec<[f64; 3]>,
    pub(super) tri_corner_uvs: Vec<[[f64; 2]; 3]>,
    pub(super) edges: Vec<[i64; 2]>,
    pub(super) texture_files: Vec<String>,
    pub(super) texture_images: Vec<MeshlibSceneTextureImage>,
    pub(super) texture_per_face: Vec<i64>,
    pub(super) object_names: Vec<String>,
    pub(super) material_names: Vec<String>,
    pub(super) diffuse_color: Option<[u8; 4]>,
    pub(super) meshlib_uv_coordinates: Vec<[f64; 2]>,
    pub(super) xf: MeshlibSceneXf,
    pub(super) visibility_mask: u32,
    pub(super) selected: bool,
    pub(super) locked: bool,
    pub(super) parent_locked: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedModelMesh {
    pub(super) vertices: Vec<[f64; 3]>,
    pub(super) faces: Vec<[i64; 3]>,
    pub(super) vertex_colors: Vec<[u8; 4]>,
    pub(super) face_colors: Vec<[u8; 4]>,
    pub(super) vertex_uvs: Vec<[f64; 2]>,
    pub(super) vertex_normals: Vec<[f64; 3]>,
    pub(super) tri_corner_uvs: Vec<[[f64; 2]; 3]>,
    pub(super) edges: Vec<[i64; 2]>,
    pub(super) texture_files: Vec<String>,
    pub(super) texture_images: Vec<MeshlibSceneTextureImage>,
    pub(super) texture_per_face: Vec<i64>,
    pub(super) object_names: Vec<String>,
    pub(super) material_names: Vec<String>,
    pub(super) diffuse_color: Option<[u8; 4]>,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedDistanceMapModel {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) values: Vec<f32>,
    pub(super) origin_world: [f64; 3],
    pub(super) pixel_x_vec: [f64; 3],
    pub(super) pixel_y_vec: [f64; 3],
    pub(super) depth_vec: [f64; 3],
}

#[derive(Debug, Clone)]
pub(super) struct ParsedVoxelModel {
    pub(super) dimensions: [usize; 3],
    pub(super) voxel_size: [f32; 3],
    pub(super) origin: [i32; 3],
    pub(super) grid_level_set: bool,
    pub(super) active_mask_compressed: bool,
    pub(super) background_value: f32,
    pub(super) values: Vec<f32>,
    pub(super) min_value: f32,
    pub(super) max_value: f32,
}

#[derive(Debug, Clone)]
pub(super) struct MeshlibRawVoxelAutoname {
    pub(super) dimensions: [usize; 3],
    pub(super) voxel_size: [f32; 3],
    pub(super) grid_level_set: bool,
}

#[derive(Debug, Clone)]
pub(super) struct CachedParsedModelMesh {
    pub(super) model: ParsedModelMesh,
    pub(super) source_object_index: usize,
}
