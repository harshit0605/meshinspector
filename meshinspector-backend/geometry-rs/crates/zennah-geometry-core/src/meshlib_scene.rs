use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

mod bitset;
mod decode;
mod edit;
mod export_archive;
mod export_validation;
mod export_values;
mod export_write;
mod import_public;
mod import_tree;
mod merge;
mod render_features;
mod voxel_dual;
mod voxel_gav;
mod voxel_vdb;

use decode::{dot3, invert3};

pub(super) use bitset::{meshlib_compact_bitset_indices, meshlib_compact_bitset_value};
pub use edit::{
    meshlib_apply_scene_ribbon_action, meshlib_apply_scene_tree_ribbon_action,
    meshlib_group_scene_tree_objects, meshlib_rename_scene_object,
    meshlib_rename_scene_tree_object, meshlib_reorder_scene_children,
    meshlib_reparent_scene_object, meshlib_select_scene_objects,
    meshlib_set_scene_feature_object_visualize_property, meshlib_set_scene_object_state,
    meshlib_transform_scene_object, meshlib_ungroup_scene_tree_objects,
};
pub use export_archive::{
    meshlib_multi_object_mru_scene_bytes, meshlib_multi_object_mru_scene_bytes_with_child_order,
};
pub use export_values::meshlib_object_mesh_mru_scene_bytes;
pub use export_write::{meshlib_object_mesh_mru_scene_value, meshlib_object_mesh_scene_value};
pub use import_public::{
    meshlib_object_mesh_document_from_mru_scene_bytes, meshlib_object_mesh_from_mru_scene_bytes,
};
pub use merge::meshlib_object_mesh_scene_json;
pub use render_features::meshlib_scene_feature_object_render_payload;
pub use voxel_dual::{
    meshlib_vdb_payload_to_dual_mesh, meshlib_vdb_payload_to_dual_mesh_with_settings,
};

pub const VIEWPORT_MASK_ALL: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibSceneTextureImage {
    pub width: u32,
    pub height: u32,
    pub pixels_rgba: Vec<[u8; 4]>,
    pub filter: String,
    pub wrap: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibObjectMeshSceneInput {
    pub object_name: String,
    pub child_index: usize,
    pub model_extension: String,
    pub textures: Vec<MeshlibSceneTextureImage>,
    pub texture_per_face: Vec<i64>,
    pub tri_corner_uvs: Vec<[[f64; 2]; 3]>,
    pub vertex_uvs: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibObjectMeshMruScene {
    pub root_file: String,
    pub root_key: String,
    pub object_name: String,
    pub object_key: String,
    pub model_file: String,
    pub model_extension: String,
    pub model_bytes: Vec<u8>,
    pub texture_per_face: Vec<i64>,
    pub uv_coordinates: Vec<[f64; 2]>,
    pub textures: Vec<MeshlibSceneTextureImage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibObjectMeshMruDocument {
    pub root_file: String,
    pub root_key: String,
    pub object_name: String,
    pub object_key: String,
    pub model_file: String,
    pub model_extension: String,
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub vertex_colors: Vec<[u8; 4]>,
    pub face_colors: Vec<[u8; 4]>,
    pub vertex_uvs: Vec<[f64; 2]>,
    pub vertex_normals: Vec<[f64; 3]>,
    pub tri_corner_uvs: Vec<[[f64; 2]; 3]>,
    pub edges: Vec<[i64; 2]>,
    pub texture_files: Vec<String>,
    pub texture_images: Vec<MeshlibSceneTextureImage>,
    pub texture_per_face: Vec<i64>,
    pub object_names: Vec<String>,
    pub material_names: Vec<String>,
    pub diffuse_color: Option<[u8; 4]>,
    pub meshlib_uv_coordinates: Vec<[f64; 2]>,
    pub scene_objects: Vec<MeshlibSceneObjectMesh>,
    pub scene_line_objects: Vec<MeshlibSceneObjectLines>,
    pub scene_point_objects: Vec<MeshlibSceneObjectPoints>,
    pub scene_distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub scene_voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub scene_feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub scene_group_objects: Vec<MeshlibSceneGroupObject>,
    pub scene_child_order: Vec<MeshlibSceneChildOrder>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshlibSceneChildOrder {
    pub parent_key: String,
    pub child_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneGroupObject {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshlibSceneXf {
    pub row_x: [f64; 3],
    pub row_y: [f64; 3],
    pub row_z: [f64; 3],
    pub b: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectMesh {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub model_file: String,
    pub model_extension: String,
    pub link: Option<String>,
    pub shared_model_source_index: Option<usize>,
    pub vertex_range: [usize; 2],
    pub face_range: [usize; 2],
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectLines {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub points: Vec<[f64; 3]>,
    pub lines: Vec<[usize; 2]>,
    pub show_points: u32,
    pub smooth_connections: u32,
    pub line_width: f32,
    pub coloring_type: String,
    pub line_colors: Vec<[u8; 4]>,
    pub vert_colors: Vec<[u8; 4]>,
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectPoints {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub model_file: String,
    pub model_extension: String,
    pub link: Option<String>,
    pub points: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub vert_colors: Vec<[u8; 4]>,
    pub point_size: f32,
    pub max_rendering_points: u64,
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectDistanceMap {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub model_file: String,
    pub model_extension: String,
    pub link: Option<String>,
    pub width: usize,
    pub height: usize,
    pub values: Vec<f32>,
    pub valid_count: usize,
    pub min_value: f32,
    pub max_value: f32,
    pub origin_world: [f64; 3],
    pub pixel_x_vec: [f64; 3],
    pub pixel_y_vec: [f64; 3],
    pub depth_vec: [f64; 3],
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectVoxels {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub model_file: String,
    pub model_extension: String,
    pub link: Option<String>,
    pub model_bytes: Vec<u8>,
    pub dimensions: [usize; 3],
    pub voxel_size: [f32; 3],
    pub grid_level_set: bool,
    pub values: Vec<f32>,
    pub min_value: f32,
    pub max_value: f32,
    pub min_corner: [usize; 3],
    pub max_corner: [usize; 3],
    pub iso_value: f32,
    pub dual_marching_cubes: bool,
    pub selected_voxels: Vec<usize>,
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureObject {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub feature_type: String,
    pub subfeature_visibility: u32,
    pub details_on_name_tag: u32,
    pub decorations_color_unselected: [f64; 4],
    pub decorations_color_selected: [f64; 4],
    pub point_size: f32,
    pub line_width: f32,
    pub sub_point_size: f32,
    pub sub_line_width: f32,
    pub main_alpha: f32,
    pub sub_alpha_points: f32,
    pub sub_alpha_lines: f32,
    pub sub_alpha_mesh: f32,
    pub dimension_visibility: HashMap<String, u32>,
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneExportInput {
    pub root_name: String,
    pub root_key: String,
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[i64; 3]>,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneExportObject {
    pub object_name: String,
    pub object_key: String,
    pub parent_key: String,
    pub hierarchy_path: Vec<String>,
    pub model_file: String,
    pub model_extension: String,
    pub link: Option<String>,
    pub shared_model_source_index: Option<usize>,
    pub vertex_range: [usize; 2],
    pub face_range: [usize; 2],
    pub xf: MeshlibSceneXf,
    pub visibility_mask: u32,
    pub selected: bool,
    pub locked: bool,
    pub parent_locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTransformInput {
    pub vertices: Vec<[f64; 3]>,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub object_key: String,
    pub xf: MeshlibSceneXf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTransformResult {
    pub vertices: Vec<[f64; 3]>,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneReparentInput {
    pub root_key: String,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub object_key: String,
    pub new_parent_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneReparentResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub scene_child_order: Vec<MeshlibSceneChildOrder>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectStateInput {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub object_key: String,
    pub visibility_mask: Option<u32>,
    pub selected: Option<bool>,
    pub locked: Option<bool>,
    pub parent_locked: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneObjectStateResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshlibSceneSelectionMode {
    SelectOne,
    Toggle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneSelectionInput {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub object_keys: Vec<String>,
    pub mode: MeshlibSceneSelectionMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneSelectionResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub selected_object_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshlibSceneFeatureVisualizeProperty {
    Subfeatures,
    DetailsOnNameTag,
    Dimension(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureVisualizePropertyInput {
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub object_key: String,
    pub property: MeshlibSceneFeatureVisualizeProperty,
    pub viewport_mask: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureVisualizePropertyResult {
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureRenderInput {
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub viewport_mask: u32,
    pub circle_segments: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureRenderPayload {
    pub objects: Vec<MeshlibSceneFeatureRenderObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureRenderObject {
    pub object_key: String,
    pub object_name: String,
    pub feature_type: String,
    pub selected: bool,
    pub label: String,
    pub primary_points: Vec<[f64; 3]>,
    pub primary_polylines: Vec<MeshlibSceneFeatureRenderPolyline>,
    pub primary_mesh_vertices: Vec<[f64; 3]>,
    pub primary_mesh_faces: Vec<[i64; 3]>,
    pub subfeature_points: Vec<[f64; 3]>,
    pub subfeature_polylines: Vec<MeshlibSceneFeatureRenderPolyline>,
    pub dimensions: Vec<MeshlibSceneFeatureRenderDimension>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureRenderPolyline {
    pub points: Vec<[f64; 3]>,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneFeatureRenderDimension {
    pub kind: String,
    pub points: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneReorderInput {
    pub root_key: String,
    pub parent_key: String,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub ordered_child_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneReorderResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub scene_child_order: Vec<MeshlibSceneChildOrder>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshlibSceneRibbonAction {
    SelectAll,
    UnselectAll,
    ShowAll,
    HideAll,
    ShowOnlyPrevious,
    ShowOnlyNext,
    SortByName,
    RemoveSelected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneRibbonActionInput {
    pub root_key: String,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub action: MeshlibSceneRibbonAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneRibbonActionResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub affected_object_keys: Vec<String>,
    pub selected_object_keys: Vec<String>,
    pub visible_object_keys: Vec<String>,
    pub removed_object_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneRenameInput {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub object_key: String,
    pub object_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneRenameResult {
    pub objects: Vec<MeshlibSceneExportObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeRibbonActionInput {
    pub root_key: String,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub action: MeshlibSceneRibbonAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeRibbonActionResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub affected_object_keys: Vec<String>,
    pub selected_object_keys: Vec<String>,
    pub visible_object_keys: Vec<String>,
    pub removed_object_keys: Vec<String>,
    pub scene_child_order: Vec<MeshlibSceneChildOrder>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeRenameInput {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub object_key: String,
    pub object_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeRenameResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeGroupInput {
    pub root_key: String,
    pub group_key: String,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeGroupResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub affected_object_keys: Vec<String>,
    pub selected_object_keys: Vec<String>,
    pub visible_object_keys: Vec<String>,
    pub removed_object_keys: Vec<String>,
    pub scene_child_order: Vec<MeshlibSceneChildOrder>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeUngroupInput {
    pub root_key: String,
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshlibSceneTreeUngroupResult {
    pub objects: Vec<MeshlibSceneExportObject>,
    pub group_objects: Vec<MeshlibSceneGroupObject>,
    pub line_objects: Vec<MeshlibSceneObjectLines>,
    pub point_objects: Vec<MeshlibSceneObjectPoints>,
    pub distance_map_objects: Vec<MeshlibSceneObjectDistanceMap>,
    pub voxel_objects: Vec<MeshlibSceneObjectVoxels>,
    pub feature_objects: Vec<MeshlibSceneFeatureObject>,
    pub affected_object_keys: Vec<String>,
    pub selected_object_keys: Vec<String>,
    pub visible_object_keys: Vec<String>,
    pub removed_object_keys: Vec<String>,
    pub scene_child_order: Vec<MeshlibSceneChildOrder>,
}

impl MeshlibSceneXf {
    fn identity() -> Self {
        Self {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [0.0, 0.0, 0.0],
        }
    }

    fn transform_point(&self, point: [f64; 3]) -> [f64; 3] {
        [
            dot3(self.row_x, point) + self.b[0],
            dot3(self.row_y, point) + self.b[1],
            dot3(self.row_z, point) + self.b[2],
        ]
    }

    fn inverse_transform_point(&self, point: [f64; 3]) -> Result<[f64; 3], String> {
        let shifted = [
            point[0] - self.b[0],
            point[1] - self.b[1],
            point[2] - self.b[2],
        ];
        let inverse = invert3([self.row_x, self.row_y, self.row_z])
            .ok_or_else(|| "MRU scene object transform is not invertible".to_string())?;
        Ok([
            dot3(inverse[0], shifted),
            dot3(inverse[1], shifted),
            dot3(inverse[2], shifted),
        ])
    }
}

pub fn meshlib_scene_key(object_name: &str, child_index: usize) -> String {
    let sanitized = object_name
        .chars()
        .map(|ch| {
            if matches!(ch, '?' | '*' | '/' | '\\' | '"' | '<' | '>' | ':' | '|') || ch.is_control()
            {
                '_'
            } else {
                ch
            }
        })
        .take(12)
        .collect::<String>();
    format!("{child_index}_{sanitized}")
}
