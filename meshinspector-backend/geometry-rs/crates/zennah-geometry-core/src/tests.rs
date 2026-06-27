use super::*;
use crate::math::{add, dot, scale};
use std::collections::HashMap;
use std::time::{Duration, Instant};

fn cube() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    (vertices, faces)
}

fn meshlib_reference_triangle_aspect_ratio(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let bc = distance(c, b);
    let ca = distance(a, c);
    let ab = distance(b, a);
    let half_perimeter = (bc + ca + ab) / 2.0;
    let denominator = 8.0 * (half_perimeter - bc) * (half_perimeter - ca) * (half_perimeter - ab);
    if denominator <= 0.0 {
        return f64::MAX;
    }
    bc * ca * ab / denominator
}

#[test]
fn select_overhang_faces_matches_meshlib_layer_basement_and_normal_contract() {
    let vertices = vec![
        [0.0, 0.0, 2.0],
        [1.0, 0.0, 2.0],
        [0.0, 1.0, 2.0],
        [3.0, 0.0, 2.0],
        [4.0, 0.0, 2.0],
        [3.0, 1.0, 2.0],
        [6.0, 0.0, 0.0],
        [7.0, 0.0, 0.0],
        [6.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [3, 4, 5], [6, 8, 7]];

    let selected = select_overhang_faces(&vertices, &faces, [0.0, 0.0, 1.0], 0.5, 0.5, 0).unwrap();

    assert_eq!(selected, vec![0]);
}

#[test]
fn extract_selected_faces_as_mesh_matches_meshlib_clone_region_packing() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 0.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3], [1, 4, 3]];

    let result = extract_selected_faces_as_mesh(&vertices, &faces, &[2, 0, 2]).unwrap();

    assert_eq!(result.source_face_indices, vec![0, 2]);
    assert_eq!(result.source_vertex_indices, vec![0, 1, 2, 4, 3]);
    assert_eq!(
        result.vertices,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ]
    );
    assert_eq!(result.faces, vec![[0, 1, 2], [1, 3, 4]]);
}

#[test]
fn extract_selected_faces_as_mesh_remaps_meshlib_clone_region_visual_attributes() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 0.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3], [1, 4, 3]];
    let attributes = MeshSelectionAttributes {
        vertex_uvs: Some(vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [2.0, 0.0],
        ]),
        vertex_colors: Some(vec![
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
            [100, 110, 120, 255],
            [130, 140, 150, 255],
        ]),
        face_colors: Some(vec![[1, 2, 3, 255], [4, 5, 6, 255], [7, 8, 9, 255]]),
        texture_per_face: Some(vec![0, 1, 0]),
    };

    let result =
        extract_selected_faces_as_mesh_with_attributes(&vertices, &faces, &[2, 0, 2], attributes)
            .unwrap();

    assert_eq!(result.source_face_indices, vec![0, 2]);
    assert_eq!(result.source_vertex_indices, vec![0, 1, 2, 4, 3]);
    assert_eq!(
        result.vertex_uvs,
        Some(vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [2.0, 0.0],
            [1.0, 1.0]
        ])
    );
    assert_eq!(
        result.vertex_colors,
        Some(vec![
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
            [130, 140, 150, 255],
            [100, 110, 120, 255],
        ])
    );
    assert_eq!(
        result.face_colors,
        Some(vec![[1, 2, 3, 255], [7, 8, 9, 255]])
    );
    assert_eq!(result.texture_per_face, Some(vec![0, 0]));
}

#[test]
fn apply_meshlib_selection_modifier_matches_primary_ctrl_toggle_contract() {
    assert_eq!(
        apply_meshlib_selection_modifier(&[0, 2, 2], &[2, 3, 3], "toggle", Some(5)).unwrap(),
        vec![0, 3]
    );
    assert_eq!(
        apply_meshlib_selection_modifier(&[0, 2], &[2, 3], "replace", Some(5)).unwrap(),
        vec![2, 3]
    );
    assert_eq!(
        apply_meshlib_selection_modifier(&[0, 2], &[2, 3], "add", Some(5)).unwrap(),
        vec![0, 2, 3]
    );
    assert_eq!(
        apply_meshlib_selection_modifier(&[0, 2, 4], &[2, 3], "subtract", Some(5)).unwrap(),
        vec![0, 4]
    );
}

fn scene_export_object_for_selection(
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneExportObject {
    MeshlibSceneExportObject {
        object_name: object_key.to_string(),
        object_key: object_key.to_string(),
        parent_key: parent_key.to_string(),
        hierarchy_path: vec![parent_key.to_string(), object_key.to_string()],
        model_file: format!("{parent_key}/{object_key}.ply"),
        model_extension: ".ply".to_string(),
        link: None,
        shared_model_source_index: None,
        vertex_range: [0, 0],
        face_range: [0, 0],
        xf: MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [0.0, 0.0, 0.0],
        },
        visibility_mask: VIEWPORT_MASK_ALL,
        selected,
        locked: false,
        parent_locked: false,
    }
}

fn scene_export_object_named(
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneExportObject {
    let mut object = scene_export_object_for_selection(object_key, parent_key, selected);
    object.object_name = object_name.to_string();
    object.hierarchy_path = if parent_key == "0_Root" {
        vec!["0_Root".to_string(), object_key.to_string()]
    } else {
        vec![
            "0_Root".to_string(),
            parent_key.to_string(),
            object_key.to_string(),
        ]
    };
    object.model_file = format!("{}.ply", object.hierarchy_path.join("/"));
    object
}

fn scene_xf_identity_for_tests() -> MeshlibSceneXf {
    MeshlibSceneXf {
        row_x: [1.0, 0.0, 0.0],
        row_y: [0.0, 1.0, 0.0],
        row_z: [0.0, 0.0, 1.0],
        b: [0.0, 0.0, 0.0],
    }
}

fn scene_path_for_tests(parent_key: &str, object_key: &str) -> Vec<String> {
    if parent_key == "0_Root" {
        vec!["0_Root".to_string(), object_key.to_string()]
    } else {
        vec![
            "0_Root".to_string(),
            parent_key.to_string(),
            object_key.to_string(),
        ]
    }
}

fn scene_line_object_named(
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneObjectLines {
    MeshlibSceneObjectLines {
        object_name: object_name.to_string(),
        object_key: object_key.to_string(),
        parent_key: parent_key.to_string(),
        hierarchy_path: scene_path_for_tests(parent_key, object_key),
        points: Vec::new(),
        lines: Vec::new(),
        show_points: 0,
        smooth_connections: 0,
        line_width: 1.0,
        coloring_type: "VertsColorMap".to_string(),
        line_colors: Vec::new(),
        vert_colors: Vec::new(),
        xf: scene_xf_identity_for_tests(),
        visibility_mask: VIEWPORT_MASK_ALL,
        selected,
        locked: false,
        parent_locked: false,
    }
}

fn scene_point_object_named(
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneObjectPoints {
    MeshlibSceneObjectPoints {
        object_name: object_name.to_string(),
        object_key: object_key.to_string(),
        parent_key: parent_key.to_string(),
        hierarchy_path: scene_path_for_tests(parent_key, object_key),
        model_file: format!("{}/{}.ply", parent_key, object_key),
        model_extension: ".ply".to_string(),
        link: None,
        points: Vec::new(),
        normals: Vec::new(),
        vert_colors: Vec::new(),
        point_size: 1.0,
        max_rendering_points: 0,
        xf: scene_xf_identity_for_tests(),
        visibility_mask: VIEWPORT_MASK_ALL,
        selected,
        locked: false,
        parent_locked: false,
    }
}

fn scene_distance_map_object_named(
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneObjectDistanceMap {
    MeshlibSceneObjectDistanceMap {
        object_name: object_name.to_string(),
        object_key: object_key.to_string(),
        parent_key: parent_key.to_string(),
        hierarchy_path: scene_path_for_tests(parent_key, object_key),
        model_file: format!("{}/{}.mrdistancemap", parent_key, object_key),
        model_extension: ".mrdistancemap".to_string(),
        link: None,
        width: 1,
        height: 1,
        values: vec![0.0],
        valid_count: 1,
        min_value: 0.0,
        max_value: 0.0,
        origin_world: [0.0, 0.0, 0.0],
        pixel_x_vec: [1.0, 0.0, 0.0],
        pixel_y_vec: [0.0, 1.0, 0.0],
        depth_vec: [0.0, 0.0, 1.0],
        xf: scene_xf_identity_for_tests(),
        visibility_mask: VIEWPORT_MASK_ALL,
        selected,
        locked: false,
        parent_locked: false,
    }
}

fn scene_voxel_object_named(
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneObjectVoxels {
    MeshlibSceneObjectVoxels {
        object_name: object_name.to_string(),
        object_key: object_key.to_string(),
        parent_key: parent_key.to_string(),
        hierarchy_path: scene_path_for_tests(parent_key, object_key),
        model_file: format!("{}/{}.raw", parent_key, object_key),
        model_extension: ".raw".to_string(),
        link: None,
        model_bytes: Vec::new(),
        dimensions: [1, 1, 1],
        voxel_size: [1.0, 1.0, 1.0],
        grid_level_set: false,
        values: vec![0.0],
        min_value: 0.0,
        max_value: 0.0,
        min_corner: [0, 0, 0],
        max_corner: [1, 1, 1],
        iso_value: 0.0,
        dual_marching_cubes: false,
        selected_voxels: Vec::new(),
        xf: scene_xf_identity_for_tests(),
        visibility_mask: VIEWPORT_MASK_ALL,
        selected,
        locked: false,
        parent_locked: false,
    }
}

fn scene_feature_object_named(
    object_name: &str,
    object_key: &str,
    parent_key: &str,
    selected: bool,
) -> MeshlibSceneFeatureObject {
    MeshlibSceneFeatureObject {
        object_name: object_name.to_string(),
        object_key: object_key.to_string(),
        parent_key: parent_key.to_string(),
        hierarchy_path: scene_path_for_tests(parent_key, object_key),
        feature_type: "PointObject".to_string(),
        subfeature_visibility: 0,
        details_on_name_tag: 0,
        decorations_color_unselected: [1.0, 1.0, 1.0, 1.0],
        decorations_color_selected: [1.0, 0.8, 0.0, 1.0],
        point_size: 1.0,
        line_width: 1.0,
        sub_point_size: 1.0,
        sub_line_width: 1.0,
        main_alpha: 1.0,
        sub_alpha_points: 1.0,
        sub_alpha_lines: 1.0,
        sub_alpha_mesh: 1.0,
        dimension_visibility: HashMap::new(),
        xf: scene_xf_identity_for_tests(),
        visibility_mask: VIEWPORT_MASK_ALL,
        selected,
        locked: false,
        parent_locked: false,
    }
}

#[test]
fn meshlib_scene_tree_group_and_ungroup_match_official_new_object_workflow() {
    let mesh_object = scene_export_object_named("Mesh", "0_Mesh", "0_Root", true);
    let line_object = scene_line_object_named("Line", "1_Line", "0_Root", true);
    let point_object = scene_point_object_named("Point", "2_Point", "0_Root", false);

    let grouped = meshlib_group_scene_tree_objects(&MeshlibSceneTreeGroupInput {
        root_key: "0_Root".to_string(),
        group_key: "3_Group".to_string(),
        objects: vec![mesh_object],
        group_objects: Vec::new(),
        line_objects: vec![line_object],
        point_objects: vec![point_object],
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    })
    .unwrap();

    assert_eq!(
        grouped.group_objects,
        vec![MeshlibSceneGroupObject {
            object_name: "Group".to_string(),
            object_key: "3_Group".to_string(),
            parent_key: "0_Root".to_string(),
            hierarchy_path: vec!["0_Root".to_string(), "3_Group".to_string()],
            xf: scene_xf_identity_for_tests(),
            visibility_mask: VIEWPORT_MASK_ALL,
            selected: false,
            locked: false,
            parent_locked: false,
        }]
    );
    assert_eq!(grouped.objects[0].parent_key, "3_Group");
    assert_eq!(grouped.line_objects[0].parent_key, "3_Group");
    assert_eq!(grouped.point_objects[0].parent_key, "0_Root");
    assert_eq!(
        grouped.scene_child_order,
        vec![
            MeshlibSceneChildOrder {
                parent_key: "0_Root".to_string(),
                child_keys: vec!["2_Point".to_string(), "3_Group".to_string()],
            },
            MeshlibSceneChildOrder {
                parent_key: "3_Group".to_string(),
                child_keys: vec!["0_Mesh".to_string(), "1_Line".to_string()],
            },
        ]
    );

    let archive = meshlib_multi_object_mru_scene_bytes_with_child_order(
        &MeshlibSceneExportInput {
            root_name: "Root".to_string(),
            root_key: "0_Root".to_string(),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            objects: grouped.objects.clone(),
            group_objects: grouped.group_objects.clone(),
            line_objects: grouped.line_objects.clone(),
            point_objects: grouped.point_objects.clone(),
            distance_map_objects: grouped.distance_map_objects.clone(),
            voxel_objects: grouped.voxel_objects.clone(),
            feature_objects: grouped.feature_objects.clone(),
        },
        &grouped.scene_child_order,
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
    let root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    assert_eq!(root["Children"]["0"]["Name"], "Point");
    assert_eq!(root["Children"]["1"]["Name"], "Group");
    assert_eq!(root["Children"]["1"]["Type"], serde_json::json!(["Object"]));
    assert_eq!(root["Children"]["1"]["Children"]["0"]["Name"], "Mesh");
    assert_eq!(root["Children"]["1"]["Children"]["1"]["Name"], "Line");

    let ungrouped = meshlib_ungroup_scene_tree_objects(&MeshlibSceneTreeUngroupInput {
        root_key: "0_Root".to_string(),
        objects: grouped.objects,
        group_objects: vec![MeshlibSceneGroupObject {
            selected: true,
            ..grouped.group_objects[0].clone()
        }],
        line_objects: grouped.line_objects,
        point_objects: grouped.point_objects,
        distance_map_objects: grouped.distance_map_objects,
        voxel_objects: grouped.voxel_objects,
        feature_objects: grouped.feature_objects,
    })
    .unwrap();

    assert!(ungrouped.group_objects.is_empty());
    assert_eq!(ungrouped.objects[0].parent_key, "0_Root");
    assert_eq!(ungrouped.line_objects[0].parent_key, "0_Root");
    assert_eq!(
        ungrouped.scene_child_order,
        vec![MeshlibSceneChildOrder {
            parent_key: "0_Root".to_string(),
            child_keys: vec![
                "2_Point".to_string(),
                "0_Mesh".to_string(),
                "1_Line".to_string(),
            ],
        }]
    );
}

#[test]
fn meshlib_scene_selection_modifier_matches_name_tag_select_one_and_toggle() {
    let objects = vec![
        scene_export_object_for_selection("0_Base_A", "0_Root", true),
        scene_export_object_for_selection("1_Child_B", "0_Base_A", false),
        scene_export_object_for_selection("2_Cover_C", "0_Root", true),
    ];

    let select_one = meshlib_select_scene_objects(&MeshlibSceneSelectionInput {
        objects: objects.clone(),
        feature_objects: Vec::new(),
        object_keys: vec!["1_Child_B".to_string()],
        mode: MeshlibSceneSelectionMode::SelectOne,
    })
    .unwrap();

    assert_eq!(select_one.selected_object_keys, vec!["1_Child_B"]);
    assert_eq!(
        select_one
            .objects
            .iter()
            .map(|object| (object.object_key.as_str(), object.selected))
            .collect::<Vec<_>>(),
        vec![
            ("0_Base_A", false),
            ("1_Child_B", true),
            ("2_Cover_C", false)
        ]
    );

    let toggle = meshlib_select_scene_objects(&MeshlibSceneSelectionInput {
        objects,
        feature_objects: Vec::new(),
        object_keys: vec!["1_Child_B".to_string(), "2_Cover_C".to_string()],
        mode: MeshlibSceneSelectionMode::Toggle,
    })
    .unwrap();

    assert_eq!(toggle.selected_object_keys, vec!["0_Base_A", "1_Child_B"]);
    assert_eq!(
        toggle
            .objects
            .iter()
            .map(|object| (object.object_key.as_str(), object.selected))
            .collect::<Vec<_>>(),
        vec![
            ("0_Base_A", true),
            ("1_Child_B", true),
            ("2_Cover_C", false)
        ]
    );
}

#[test]
fn meshlib_set_scene_object_state_updates_feature_object_state_without_touching_mesh_objects() {
    let mesh_object = scene_export_object_for_selection("0_Base_A", "0_Root", true);
    let feature_object = MeshlibSceneFeatureObject {
        decorations_color_selected: [0.1, 0.2, 0.3, 1.0],
        selected: false,
        locked: false,
        parent_locked: false,
        visibility_mask: VIEWPORT_MASK_ALL,
        ..scene_feature_object_named("Plane", "1_PlaneFeature", "0_Root", false)
    };
    let result = meshlib_set_scene_object_state(&MeshlibSceneObjectStateInput {
        objects: vec![mesh_object.clone()],
        feature_objects: vec![feature_object.clone()],
        object_key: "1_PlaneFeature".to_string(),
        visibility_mask: Some(0),
        selected: Some(true),
        locked: Some(true),
        parent_locked: Some(true),
    })
    .unwrap();

    assert_eq!(result.objects, vec![mesh_object]);
    assert_eq!(result.feature_objects.len(), 1);
    let updated = &result.feature_objects[0];
    assert_eq!(updated.visibility_mask, 0);
    assert!(updated.selected);
    assert!(updated.locked);
    assert!(updated.parent_locked);
    assert_eq!(
        updated.decorations_color_selected,
        feature_object.decorations_color_selected
    );
}

#[test]
fn meshlib_select_scene_objects_includes_feature_objects_in_name_tag_selection() {
    let objects = vec![
        scene_export_object_for_selection("0_Base_A", "0_Root", true),
        scene_export_object_for_selection("1_Child_B", "0_Base_A", false),
    ];
    let feature_objects = vec![
        MeshlibSceneFeatureObject {
            selected: true,
            ..scene_feature_object_named("Plane A", "2_Plane_A", "0_Root", true)
        },
        MeshlibSceneFeatureObject {
            selected: false,
            ..scene_feature_object_named("Plane B", "3_Plane_B", "0_Root", false)
        },
    ];

    let select_one = meshlib_select_scene_objects(&MeshlibSceneSelectionInput {
        objects: objects.clone(),
        feature_objects: feature_objects.clone(),
        object_keys: vec!["3_Plane_B".to_string()],
        mode: MeshlibSceneSelectionMode::SelectOne,
    })
    .unwrap();

    assert_eq!(select_one.selected_object_keys, vec!["3_Plane_B"]);
    assert_eq!(
        select_one
            .objects
            .iter()
            .map(|object| (object.object_key.as_str(), object.selected))
            .collect::<Vec<_>>(),
        vec![("0_Base_A", false), ("1_Child_B", false)]
    );
    assert_eq!(
        select_one
            .feature_objects
            .iter()
            .map(|object| (object.object_key.as_str(), object.selected))
            .collect::<Vec<_>>(),
        vec![("2_Plane_A", false), ("3_Plane_B", true)]
    );

    let toggle = meshlib_select_scene_objects(&MeshlibSceneSelectionInput {
        objects,
        feature_objects,
        object_keys: vec!["1_Child_B".to_string(), "2_Plane_A".to_string()],
        mode: MeshlibSceneSelectionMode::Toggle,
    })
    .unwrap();

    assert_eq!(toggle.selected_object_keys, vec!["0_Base_A", "1_Child_B"]);
    assert_eq!(
        toggle
            .feature_objects
            .iter()
            .map(|object| (object.object_key.as_str(), object.selected))
            .collect::<Vec<_>>(),
        vec![("2_Plane_A", false), ("3_Plane_B", false)]
    );
}

#[test]
fn meshlib_set_scene_feature_object_visualize_property_updates_feature_masks() {
    let feature_object = MeshlibSceneFeatureObject {
        subfeature_visibility: VIEWPORT_MASK_ALL,
        details_on_name_tag: 0,
        dimension_visibility: HashMap::from([
            ("Length".to_string(), VIEWPORT_MASK_ALL),
            ("Diameter".to_string(), VIEWPORT_MASK_ALL),
        ]),
        ..scene_feature_object_named("Cylinder", "4_CylinderFeature", "0_Root", false)
    };

    let subfeatures = meshlib_set_scene_feature_object_visualize_property(
        &MeshlibSceneFeatureVisualizePropertyInput {
            feature_objects: vec![feature_object.clone()],
            object_key: "4_CylinderFeature".to_string(),
            property: MeshlibSceneFeatureVisualizeProperty::Subfeatures,
            viewport_mask: 0,
        },
    )
    .unwrap();
    assert_eq!(subfeatures.feature_objects[0].subfeature_visibility, 0);
    assert_eq!(subfeatures.feature_objects[0].details_on_name_tag, 0);
    assert_eq!(
        subfeatures.feature_objects[0]
            .dimension_visibility
            .get("Length"),
        Some(&VIEWPORT_MASK_ALL)
    );

    let details = meshlib_set_scene_feature_object_visualize_property(
        &MeshlibSceneFeatureVisualizePropertyInput {
            feature_objects: subfeatures.feature_objects.clone(),
            object_key: "4_CylinderFeature".to_string(),
            property: MeshlibSceneFeatureVisualizeProperty::DetailsOnNameTag,
            viewport_mask: VIEWPORT_MASK_ALL,
        },
    )
    .unwrap();
    assert_eq!(
        details.feature_objects[0].details_on_name_tag,
        VIEWPORT_MASK_ALL
    );

    let dimensions = meshlib_set_scene_feature_object_visualize_property(
        &MeshlibSceneFeatureVisualizePropertyInput {
            feature_objects: details.feature_objects,
            object_key: "4_CylinderFeature".to_string(),
            property: MeshlibSceneFeatureVisualizeProperty::Dimension("Length".to_string()),
            viewport_mask: 3,
        },
    )
    .unwrap();

    assert_eq!(
        dimensions.feature_objects[0]
            .dimension_visibility
            .get("Length"),
        Some(&3)
    );
    assert_eq!(
        dimensions.feature_objects[0]
            .dimension_visibility
            .get("Diameter"),
        Some(&VIEWPORT_MASK_ALL)
    );
    assert_eq!(
        dimensions.feature_objects[0].decorations_color_selected,
        feature_object.decorations_color_selected
    );
}

#[test]
fn meshlib_scene_feature_object_render_payload_matches_point_line_plane_primitives() {
    let point = MeshlibSceneFeatureObject {
        feature_type: "PointObject".to_string(),
        details_on_name_tag: VIEWPORT_MASK_ALL,
        xf: MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [1.0, 2.0, 3.0],
        },
        ..scene_feature_object_named("Point", "0_PointFeature", "0_Root", false)
    };
    let line = MeshlibSceneFeatureObject {
        feature_type: "LineObject".to_string(),
        details_on_name_tag: VIEWPORT_MASK_ALL,
        xf: MeshlibSceneXf {
            row_x: [2.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [10.0, 0.0, 0.0],
        },
        ..scene_feature_object_named("Line", "1_LineFeature", "0_Root", true)
    };
    let plane = MeshlibSceneFeatureObject {
        feature_type: "PlaneObject".to_string(),
        subfeature_visibility: 0,
        xf: MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [0.0, 0.0, 5.0],
        },
        ..scene_feature_object_named("Plane", "2_PlaneFeature", "0_Root", false)
    };

    let payload = meshlib_scene_feature_object_render_payload(&MeshlibSceneFeatureRenderInput {
        feature_objects: vec![point, line, plane],
        viewport_mask: VIEWPORT_MASK_ALL,
        circle_segments: 8,
    })
    .unwrap();

    assert_eq!(payload.objects.len(), 3);
    assert_eq!(payload.objects[0].primary_points, vec![[1.0, 2.0, 3.0]]);
    assert!(payload.objects[0].label.contains("Point"));
    assert!(payload.objects[0].label.contains("1.00; 2.00; 3.00"));

    assert_eq!(payload.objects[1].primary_polylines.len(), 1);
    assert_eq!(
        payload.objects[1].primary_polylines[0].points,
        vec![[8.0, 0.0, 0.0], [12.0, 0.0, 0.0]]
    );
    assert!(!payload.objects[1].primary_polylines[0].closed);
    assert!(payload.objects[1].label.contains("dir 1.00, 0.00, 0.00"));

    assert_eq!(
        payload.objects[2].primary_mesh_vertices,
        vec![
            [1.0, 1.0, 5.0],
            [1.0, -1.0, 5.0],
            [-1.0, -1.0, 5.0],
            [-1.0, 1.0, 5.0],
        ]
    );
    assert_eq!(
        payload.objects[2].primary_mesh_faces,
        vec![[0, 2, 1], [0, 3, 2]]
    );
    assert!(payload.objects[2].subfeature_points.is_empty());
    assert!(payload.objects[2].subfeature_polylines.is_empty());
}

#[test]
fn meshlib_scene_feature_object_render_payload_matches_circle_cylinder_cone_primitives() {
    let circle = MeshlibSceneFeatureObject {
        feature_type: "CircleObject".to_string(),
        xf: MeshlibSceneXf {
            row_x: [2.0, 0.0, 0.0],
            row_y: [0.0, 2.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [1.0, 0.0, 0.0],
        },
        ..scene_feature_object_named("Circle", "3_CircleFeature", "0_Root", false)
    };
    let cylinder = MeshlibSceneFeatureObject {
        feature_type: "CylinderObject".to_string(),
        dimension_visibility: HashMap::from([("Diameter".to_string(), VIEWPORT_MASK_ALL)]),
        xf: MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 2.0],
            b: [0.0, 0.0, 10.0],
        },
        ..scene_feature_object_named("Cylinder", "4_CylinderFeature", "0_Root", false)
    };
    let cone = MeshlibSceneFeatureObject {
        feature_type: "ConeObject".to_string(),
        dimension_visibility: HashMap::from([
            ("Angle".to_string(), VIEWPORT_MASK_ALL),
            ("Diameter".to_string(), VIEWPORT_MASK_ALL),
            ("Length".to_string(), VIEWPORT_MASK_ALL),
        ]),
        ..scene_feature_object_named("Cone", "5_ConeFeature", "0_Root", false)
    };

    let payload = meshlib_scene_feature_object_render_payload(&MeshlibSceneFeatureRenderInput {
        feature_objects: vec![circle, cylinder, cone],
        viewport_mask: VIEWPORT_MASK_ALL,
        circle_segments: 8,
    })
    .unwrap();

    assert_eq!(payload.objects.len(), 3);

    let circle_polyline = &payload.objects[0].primary_polylines[0];
    assert!(circle_polyline.closed);
    assert_eq!(circle_polyline.points.len(), 128);
    assert_eq!(circle_polyline.points[0], [3.0, 0.0, 0.0]);
    assert!((circle_polyline.points[32][0] - 1.0).abs() < 1e-9);
    assert!((circle_polyline.points[32][1] - 2.0).abs() < 1e-9);

    assert_eq!(payload.objects[1].primary_mesh_vertices.len(), 256);
    assert_eq!(payload.objects[1].primary_mesh_faces.len(), 256);
    assert_eq!(payload.objects[1].primary_mesh_vertices[0], [1.0, 0.0, 9.0]);
    assert_eq!(
        payload.objects[1].primary_mesh_vertices[128],
        [1.0, 0.0, 11.0]
    );
    assert_eq!(payload.objects[1].primary_mesh_faces[0], [0, 1, 128]);
    assert_eq!(payload.objects[1].primary_mesh_faces[1], [1, 129, 128]);
    assert_eq!(
        payload.objects[1].dimensions,
        vec![MeshlibSceneFeatureRenderDimension {
            kind: "Diameter".to_string(),
            points: vec![[-1.0, 0.0, 10.0], [1.0, 0.0, 10.0]],
        }]
    );

    assert_eq!(payload.objects[2].primary_mesh_vertices.len(), 129);
    assert_eq!(payload.objects[2].primary_mesh_faces.len(), 128);
    assert_eq!(payload.objects[2].primary_mesh_vertices[0], [1.0, 0.0, 1.0]);
    assert_eq!(
        payload.objects[2].primary_mesh_vertices[128],
        [0.0, 0.0, 0.0]
    );
    assert_eq!(payload.objects[2].primary_mesh_faces[0], [1, 0, 128]);
    assert_eq!(payload.objects[2].primary_mesh_faces[127], [0, 127, 128]);
    assert_eq!(
        payload.objects[2].dimensions,
        vec![
            MeshlibSceneFeatureRenderDimension {
                kind: "Diameter".to_string(),
                points: vec![[-1.0, 0.0, 1.0], [1.0, 0.0, 1.0]],
            },
            MeshlibSceneFeatureRenderDimension {
                kind: "Angle".to_string(),
                points: vec![[0.5, 0.0, 0.5], [-0.5, 0.0, 0.5]],
            },
            MeshlibSceneFeatureRenderDimension {
                kind: "Length".to_string(),
                points: vec![[0.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            },
        ]
    );
}

#[test]
fn meshlib_scene_feature_object_render_payload_includes_meshlib_visual_subfeatures() {
    let feature = |name: &str, key: &str| MeshlibSceneFeatureObject {
        feature_type: name.to_string(),
        subfeature_visibility: VIEWPORT_MASK_ALL,
        ..scene_feature_object_named(name, key, "0_Root", false)
    };

    let payload = meshlib_scene_feature_object_render_payload(&MeshlibSceneFeatureRenderInput {
        feature_objects: vec![
            feature("PlaneObject", "2_PlaneFeature"),
            feature("CircleObject", "3_CircleFeature"),
            feature("SphereObject", "4_SphereFeature"),
            feature("CylinderObject", "5_CylinderFeature"),
            feature("ConeObject", "6_ConeFeature"),
        ],
        viewport_mask: VIEWPORT_MASK_ALL,
        circle_segments: 8,
    })
    .unwrap();

    assert_eq!(payload.objects.len(), 5);

    assert!(payload.objects[0].primary_polylines.is_empty());
    assert_eq!(payload.objects[0].subfeature_points, vec![[0.0, 0.0, 0.0]]);
    assert_eq!(payload.objects[0].subfeature_polylines.len(), 1);
    assert!(payload.objects[0].subfeature_polylines[0].closed);
    assert_eq!(
        payload.objects[0].subfeature_polylines[0].points,
        vec![
            [1.0, 1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]
    );

    assert_eq!(payload.objects[1].subfeature_points, vec![[0.0, 0.0, 0.0]]);
    assert!(payload.objects[1].subfeature_polylines.is_empty());

    assert_eq!(payload.objects[2].subfeature_points, vec![[0.0, 0.0, 0.0]]);
    assert!(payload.objects[2].subfeature_polylines.is_empty());

    assert_eq!(
        payload.objects[3].subfeature_points,
        vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.5], [0.0, 0.0, -0.5]]
    );
    assert_eq!(payload.objects[3].subfeature_polylines.len(), 3);
    assert_eq!(
        payload.objects[3].subfeature_polylines[0].points,
        vec![[0.0, 0.0, -0.5], [0.0, 0.0, 0.5]]
    );
    assert!(!payload.objects[3].subfeature_polylines[0].closed);
    assert!(payload.objects[3].subfeature_polylines[1].closed);
    assert!(payload.objects[3].subfeature_polylines[2].closed);
    assert_eq!(payload.objects[3].subfeature_polylines[1].points.len(), 128);
    assert_eq!(payload.objects[3].subfeature_polylines[2].points.len(), 128);
    assert_eq!(payload.objects[3].subfeature_polylines[1].points[0][2], 0.5);
    assert_eq!(
        payload.objects[3].subfeature_polylines[2].points[0][2],
        -0.5
    );

    assert_eq!(
        payload.objects[4].subfeature_points,
        vec![[0.0, 0.0, 0.5], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]]
    );
    assert_eq!(payload.objects[4].subfeature_polylines.len(), 2);
    assert_eq!(
        payload.objects[4].subfeature_polylines[0].points,
        vec![[0.0, 0.0, 1.0], [0.0, 0.0, 0.0]]
    );
    assert!(!payload.objects[4].subfeature_polylines[0].closed);
    assert!(payload.objects[4].subfeature_polylines[1].closed);
    assert_eq!(payload.objects[4].subfeature_polylines[1].points.len(), 128);
    assert_eq!(
        payload.objects[4].subfeature_polylines[1].points[0],
        [1.0, 0.0, 1.0]
    );
}

#[test]
fn meshlib_scene_feature_object_render_payload_includes_sphere_primary_mesh() {
    let sphere = MeshlibSceneFeatureObject {
        feature_type: "SphereObject".to_string(),
        dimension_visibility: HashMap::from([("Diameter".to_string(), VIEWPORT_MASK_ALL)]),
        xf: MeshlibSceneXf {
            row_x: [2.0, 0.0, 0.0],
            row_y: [0.0, 2.0, 0.0],
            row_z: [0.0, 0.0, 2.0],
            b: [10.0, 20.0, 30.0],
        },
        ..scene_feature_object_named("Sphere", "7_SphereFeature", "0_Root", false)
    };

    let payload = meshlib_scene_feature_object_render_payload(&MeshlibSceneFeatureRenderInput {
        feature_objects: vec![sphere],
        viewport_mask: VIEWPORT_MASK_ALL,
        circle_segments: 8,
    })
    .unwrap();

    let render = &payload.objects[0];
    assert_eq!(render.primary_mesh_vertices.len(), 2048);
    assert_eq!(render.primary_mesh_faces.len(), 4092);
    let expected_corner = [
        10.0 - 2.0 / 3.0_f64.sqrt(),
        20.0 - 2.0 / 3.0_f64.sqrt(),
        30.0 - 2.0 / 3.0_f64.sqrt(),
    ];
    for axis in 0..3 {
        assert!((render.primary_mesh_vertices[0][axis] - expected_corner[axis]).abs() < 1e-12);
    }
    for vertex in render.primary_mesh_vertices.iter().step_by(97) {
        let dx = vertex[0] - 10.0;
        let dy = vertex[1] - 20.0;
        let dz = vertex[2] - 30.0;
        assert!(((dx * dx + dy * dy + dz * dz).sqrt() - 2.0).abs() < 1e-9);
    }
    assert_eq!(
        render.dimensions,
        vec![MeshlibSceneFeatureRenderDimension {
            kind: "Diameter".to_string(),
            points: vec![[8.0, 20.0, 30.0], [12.0, 20.0, 30.0]],
        }]
    );
}

#[test]
fn meshlib_scene_tree_ribbon_actions_cover_imported_data_object_types() {
    let input = MeshlibSceneTreeRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: vec![scene_export_object_named("Mesh", "0_Mesh", "0_Root", false)],
        group_objects: Vec::new(),
        line_objects: vec![scene_line_object_named("Line", "1_Line", "0_Root", true)],
        point_objects: vec![scene_point_object_named(
            "Point", "2_Point", "0_Root", false,
        )],
        distance_map_objects: vec![scene_distance_map_object_named(
            "Distance",
            "3_Distance",
            "0_Root",
            false,
        )],
        voxel_objects: vec![scene_voxel_object_named(
            "Voxels", "4_Voxels", "0_Root", false,
        )],
        feature_objects: vec![scene_feature_object_named(
            "Feature",
            "5_Feature",
            "2_Point",
            false,
        )],
        action: MeshlibSceneRibbonAction::SelectAll,
    };

    let selected = meshlib_apply_scene_tree_ribbon_action(&input).unwrap();
    assert_eq!(
        selected.selected_object_keys,
        vec![
            "0_Mesh",
            "1_Line",
            "2_Point",
            "3_Distance",
            "4_Voxels",
            "5_Feature"
        ]
    );
    assert!(selected.objects[0].selected);
    assert!(selected.line_objects[0].selected);
    assert!(selected.point_objects[0].selected);
    assert!(selected.distance_map_objects[0].selected);
    assert!(selected.voxel_objects[0].selected);
    assert!(selected.feature_objects[0].selected);

    let shown_next = meshlib_apply_scene_tree_ribbon_action(&MeshlibSceneTreeRibbonActionInput {
        root_key: "0_Root".to_string(),
        action: MeshlibSceneRibbonAction::ShowOnlyNext,
        ..input
    })
    .unwrap();
    assert_eq!(shown_next.selected_object_keys, vec!["2_Point"]);
    assert_eq!(shown_next.visible_object_keys, vec!["2_Point", "5_Feature"]);
    assert_eq!(shown_next.line_objects[0].visibility_mask, 0);
    assert_eq!(
        shown_next.point_objects[0].visibility_mask,
        VIEWPORT_MASK_ALL
    );
    assert_eq!(
        shown_next.feature_objects[0].visibility_mask,
        VIEWPORT_MASK_ALL
    );

    let renamed = meshlib_rename_scene_tree_object(&MeshlibSceneTreeRenameInput {
        objects: shown_next.objects.clone(),
        group_objects: Vec::new(),
        line_objects: shown_next.line_objects.clone(),
        point_objects: shown_next.point_objects.clone(),
        distance_map_objects: shown_next.distance_map_objects.clone(),
        voxel_objects: shown_next.voxel_objects.clone(),
        feature_objects: shown_next.feature_objects.clone(),
        object_key: "5_Feature".to_string(),
        object_name: "Renamed Feature".to_string(),
    })
    .unwrap();
    assert_eq!(renamed.feature_objects[0].object_name, "Renamed Feature");

    let removed = meshlib_apply_scene_tree_ribbon_action(&MeshlibSceneTreeRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: renamed.objects,
        group_objects: Vec::new(),
        line_objects: renamed.line_objects,
        point_objects: renamed.point_objects,
        distance_map_objects: renamed.distance_map_objects,
        voxel_objects: renamed.voxel_objects,
        feature_objects: renamed.feature_objects,
        action: MeshlibSceneRibbonAction::RemoveSelected,
    })
    .unwrap();
    assert_eq!(removed.removed_object_keys, vec!["2_Point", "5_Feature"]);
    assert!(removed.point_objects.is_empty());
    assert!(removed.feature_objects.is_empty());
    assert_eq!(removed.objects.len(), 1);
    assert_eq!(removed.line_objects.len(), 1);
    assert_eq!(removed.distance_map_objects.len(), 1);
    assert_eq!(removed.voxel_objects.len(), 1);
}

#[test]
fn meshlib_scene_tree_sort_by_name_exports_mixed_data_children_in_meshlib_order() {
    let mesh_object = scene_export_object_named("zeta mesh", "0_ZetaMesh", "0_Root", false);
    let line_object = scene_line_object_named("alpha lines", "1_AlphaLines", "0_Root", false);
    let point_object = scene_point_object_named("Bravo points", "2_BravoPoints", "0_Root", false);
    let distance_object =
        scene_distance_map_object_named("charlie distance", "3_CharlieDistance", "0_Root", false);
    let voxel_object = scene_voxel_object_named("echo voxels", "4_EchoVoxels", "0_Root", false);
    let feature_object =
        scene_feature_object_named("delta feature", "5_DeltaFeature", "0_ZetaMesh", false);
    let nested_line =
        scene_line_object_named("beta nested lines", "6_BetaNested", "0_ZetaMesh", false);

    let sorted = meshlib_apply_scene_tree_ribbon_action(&MeshlibSceneTreeRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: vec![mesh_object.clone()],
        group_objects: Vec::new(),
        line_objects: vec![line_object.clone(), nested_line.clone()],
        point_objects: vec![point_object.clone()],
        distance_map_objects: vec![distance_object.clone()],
        voxel_objects: vec![voxel_object.clone()],
        feature_objects: vec![feature_object.clone()],
        action: MeshlibSceneRibbonAction::SortByName,
    })
    .unwrap();

    assert_eq!(
        sorted.scene_child_order,
        vec![
            MeshlibSceneChildOrder {
                parent_key: "0_ZetaMesh".to_string(),
                child_keys: vec!["6_BetaNested".to_string(), "5_DeltaFeature".to_string()],
            },
            MeshlibSceneChildOrder {
                parent_key: "0_Root".to_string(),
                child_keys: vec![
                    "1_AlphaLines".to_string(),
                    "2_BravoPoints".to_string(),
                    "3_CharlieDistance".to_string(),
                    "4_EchoVoxels".to_string(),
                    "0_ZetaMesh".to_string(),
                ],
            },
        ]
    );

    let archive = meshlib_multi_object_mru_scene_bytes_with_child_order(
        &MeshlibSceneExportInput {
            root_name: "Root".to_string(),
            root_key: "0_Root".to_string(),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            faces: vec![[0, 1, 2]],
            objects: sorted.objects,
            group_objects: Vec::new(),
            line_objects: sorted.line_objects,
            point_objects: sorted.point_objects,
            distance_map_objects: sorted.distance_map_objects,
            voxel_objects: sorted.voxel_objects,
            feature_objects: sorted.feature_objects,
        },
        &sorted.scene_child_order,
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
    let root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();

    assert_eq!(root["Children"]["0"]["Name"], "alpha lines");
    assert_eq!(root["Children"]["1"]["Name"], "Bravo points");
    assert_eq!(root["Children"]["2"]["Name"], "charlie distance");
    assert_eq!(root["Children"]["3"]["Name"], "echo voxels");
    assert_eq!(root["Children"]["4"]["Name"], "zeta mesh");
    assert_eq!(
        root["Children"]["4"]["Children"]["0"]["Name"],
        "beta nested lines"
    );
    assert_eq!(
        root["Children"]["4"]["Children"]["1"]["Name"],
        "delta feature"
    );
}

#[test]
fn meshlib_scene_ribbon_bulk_actions_match_official_scene_buttons() {
    let mut objects = vec![
        scene_export_object_named("Zeta", "0_Zeta", "0_Root", true),
        scene_export_object_named("Alpha", "1_Alpha", "0_Root", false),
        scene_export_object_named("beta", "2_beta", "0_Root", true),
    ];
    objects[0].visibility_mask = 0;
    objects[1].visibility_mask = 27;

    let select_all = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: objects.clone(),
        action: MeshlibSceneRibbonAction::SelectAll,
    })
    .unwrap();
    assert_eq!(
        select_all.selected_object_keys,
        vec!["0_Zeta", "1_Alpha", "2_beta"]
    );
    assert!(select_all
        .objects
        .iter()
        .all(|object| object.selected && object.visibility_mask == VIEWPORT_MASK_ALL));

    let unselect_all = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: select_all.objects.clone(),
        action: MeshlibSceneRibbonAction::UnselectAll,
    })
    .unwrap();
    assert!(unselect_all
        .objects
        .iter()
        .all(|object| !object.selected && object.visibility_mask == VIEWPORT_MASK_ALL));

    let hide_all = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: unselect_all.objects.clone(),
        action: MeshlibSceneRibbonAction::HideAll,
    })
    .unwrap();
    assert_eq!(hide_all.visible_object_keys, Vec::<String>::new());
    assert!(hide_all
        .objects
        .iter()
        .all(|object| object.visibility_mask == 0));

    let show_all = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: hide_all.objects,
        action: MeshlibSceneRibbonAction::ShowAll,
    })
    .unwrap();
    assert_eq!(
        show_all.visible_object_keys,
        vec!["0_Zeta", "1_Alpha", "2_beta"]
    );
    assert!(show_all
        .objects
        .iter()
        .all(|object| object.visibility_mask == VIEWPORT_MASK_ALL));
}

#[test]
fn meshlib_scene_ribbon_show_only_next_and_previous_match_scene_list_navigation() {
    let objects = vec![
        scene_export_object_named("Zeta", "0_Zeta", "0_Root", true),
        scene_export_object_named("Alpha", "1_Alpha", "0_Root", false),
        scene_export_object_named("beta", "2_beta", "0_Root", false),
        scene_export_object_named("Nested", "3_Nested", "1_Alpha", false),
    ];

    let next = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects,
        action: MeshlibSceneRibbonAction::ShowOnlyNext,
    })
    .unwrap();
    assert_eq!(next.selected_object_keys, vec!["1_Alpha"]);
    assert_eq!(next.visible_object_keys, vec!["1_Alpha", "3_Nested"]);
    assert_eq!(
        next.objects
            .iter()
            .map(|object| (
                object.object_key.as_str(),
                object.selected,
                object.visibility_mask
            ))
            .collect::<Vec<_>>(),
        vec![
            ("0_Zeta", false, 0),
            ("1_Alpha", true, VIEWPORT_MASK_ALL),
            ("2_beta", false, 0),
            ("3_Nested", false, VIEWPORT_MASK_ALL),
        ]
    );

    let previous = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: next.objects,
        action: MeshlibSceneRibbonAction::ShowOnlyPrevious,
    })
    .unwrap();
    assert_eq!(previous.selected_object_keys, vec!["0_Zeta"]);
    assert_eq!(
        previous
            .objects
            .iter()
            .map(|object| (
                object.object_key.as_str(),
                object.selected,
                object.visibility_mask
            ))
            .collect::<Vec<_>>(),
        vec![
            ("0_Zeta", true, VIEWPORT_MASK_ALL),
            ("1_Alpha", false, 0),
            ("2_beta", false, 0),
            ("3_Nested", false, VIEWPORT_MASK_ALL),
        ]
    );
}

#[test]
fn meshlib_scene_ribbon_sort_rename_and_remove_match_official_scene_buttons() {
    let objects = vec![
        scene_export_object_named("Zeta", "0_Zeta", "0_Root", false),
        scene_export_object_named("Alpha", "1_Alpha", "0_Root", false),
        scene_export_object_named("beta", "2_beta", "0_Root", false),
        scene_export_object_named("delta", "3_delta", "0_Zeta", true),
        scene_export_object_named("Charlie", "4_Charlie", "0_Zeta", false),
    ];

    let sorted = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: objects.clone(),
        action: MeshlibSceneRibbonAction::SortByName,
    })
    .unwrap();
    assert_eq!(
        sorted
            .objects
            .iter()
            .map(|object| object.object_key.as_str())
            .collect::<Vec<_>>(),
        vec!["1_Alpha", "2_beta", "0_Zeta", "4_Charlie", "3_delta"]
    );

    let renamed = meshlib_rename_scene_object(&MeshlibSceneRenameInput {
        objects: sorted.objects,
        object_key: "4_Charlie".to_string(),
        object_name: "Echo".to_string(),
    })
    .unwrap();
    assert_eq!(
        renamed
            .objects
            .iter()
            .find(|object| object.object_key == "4_Charlie")
            .unwrap()
            .object_name,
        "Echo"
    );

    let removed = meshlib_apply_scene_ribbon_action(&MeshlibSceneRibbonActionInput {
        root_key: "0_Root".to_string(),
        objects: renamed.objects,
        action: MeshlibSceneRibbonAction::RemoveSelected,
    })
    .unwrap();
    assert_eq!(removed.removed_object_keys, vec!["3_delta"]);
    assert_eq!(
        removed
            .objects
            .iter()
            .map(|object| object.object_key.as_str())
            .collect::<Vec<_>>(),
        vec!["1_Alpha", "2_beta", "0_Zeta", "4_Charlie"]
    );
}

#[test]
fn meshlib_transform_scene_object_updates_world_vertices_from_object_xf() {
    let input = MeshlibSceneTransformInput {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.5, 0.0, 0.0],
            [4.0, 0.5, 0.0],
        ],
        objects: vec![
            MeshlibSceneExportObject {
                object_name: "Base A".to_string(),
                object_key: "0_Base_A".to_string(),
                parent_key: "0_Root".to_string(),
                hierarchy_path: vec!["0_Root".to_string(), "0_Base_A".to_string()],
                model_file: "0_Root/0_Base_A.ply".to_string(),
                model_extension: ".ply".to_string(),
                link: None,
                shared_model_source_index: None,
                vertex_range: [0, 3],
                face_range: [0, 1],
                xf: MeshlibSceneXf {
                    row_x: [1.0, 0.0, 0.0],
                    row_y: [0.0, 1.0, 0.0],
                    row_z: [0.0, 0.0, 1.0],
                    b: [0.0, 0.0, 0.0],
                },
                visibility_mask: VIEWPORT_MASK_ALL,
                selected: false,
                locked: false,
                parent_locked: false,
            },
            MeshlibSceneExportObject {
                object_name: "Translated B".to_string(),
                object_key: "1_Translated".to_string(),
                parent_key: "0_Root".to_string(),
                hierarchy_path: vec!["0_Root".to_string(), "1_Translated".to_string()],
                model_file: "0_Root/1_Translated.ply".to_string(),
                model_extension: ".ply".to_string(),
                link: None,
                shared_model_source_index: None,
                vertex_range: [3, 6],
                face_range: [1, 2],
                xf: MeshlibSceneXf {
                    row_x: [1.0, 0.0, 0.0],
                    row_y: [0.0, 1.0, 0.0],
                    row_z: [0.0, 0.0, 1.0],
                    b: [4.0, 0.0, 0.0],
                },
                visibility_mask: VIEWPORT_MASK_ALL,
                selected: false,
                locked: false,
                parent_locked: false,
            },
        ],
        feature_objects: Vec::new(),
        object_key: "1_Translated".to_string(),
        xf: MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [8.0, 2.0, 0.0],
        },
    };

    let result = meshlib_transform_scene_object(&input).unwrap();

    assert_eq!(
        result.vertices,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [8.0, 2.0, 0.0],
            [8.5, 2.0, 0.0],
            [8.0, 2.5, 0.0],
        ]
    );
    assert_eq!(result.objects[0].xf, input.objects[0].xf);
    assert_eq!(result.objects[1].xf, input.xf);
}

#[test]
fn meshlib_transform_scene_feature_object_updates_feature_xf_without_touching_mesh_vertices() {
    let feature = MeshlibSceneFeatureObject {
        feature_type: "PlaneObject".to_string(),
        subfeature_visibility: 7,
        details_on_name_tag: 3,
        decorations_color_unselected: [0.25, 0.5, 0.75, 1.0],
        decorations_color_selected: [1.0, 0.7, 0.2, 1.0],
        point_size: 4.0,
        line_width: 2.0,
        sub_point_size: 2.5,
        sub_line_width: 1.5,
        main_alpha: 0.8,
        sub_alpha_points: 0.6,
        sub_alpha_lines: 0.5,
        sub_alpha_mesh: 0.4,
        dimension_visibility: HashMap::from([("Radius".to_string(), 1)]),
        xf: MeshlibSceneXf {
            row_x: [1.0, 0.0, 0.0],
            row_y: [0.0, 1.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            b: [1.0, 2.0, 3.0],
        },
        ..scene_feature_object_named("Plane", "2_PlaneFeature", "0_Root", true)
    };
    let mesh_object = scene_export_object_named("Mesh", "0_Mesh", "0_Root", false);
    let target_xf = MeshlibSceneXf {
        row_x: [0.0, -1.0, 0.0],
        row_y: [1.0, 0.0, 0.0],
        row_z: [0.0, 0.0, 1.0],
        b: [5.0, 6.0, 7.0],
    };
    let input = MeshlibSceneTransformInput {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        objects: vec![mesh_object.clone()],
        feature_objects: vec![feature.clone()],
        object_key: "2_PlaneFeature".to_string(),
        xf: target_xf,
    };

    let result = meshlib_transform_scene_object(&input).unwrap();

    assert_eq!(result.vertices, input.vertices);
    assert_eq!(result.objects, vec![mesh_object]);
    assert_eq!(result.feature_objects.len(), 1);
    let transformed_feature = &result.feature_objects[0];
    assert_eq!(transformed_feature.xf, target_xf);
    assert_eq!(transformed_feature.feature_type, feature.feature_type);
    assert_eq!(
        transformed_feature.subfeature_visibility,
        feature.subfeature_visibility
    );
    assert_eq!(
        transformed_feature.decorations_color_selected,
        feature.decorations_color_selected
    );
    assert_eq!(
        transformed_feature.dimension_visibility,
        feature.dimension_visibility
    );
    assert!(transformed_feature.selected);
}

#[test]
fn meshlib_multi_object_mru_scene_preserves_nested_object_children() {
    let input = MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: "0_Root".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.5, 0.0, 0.0],
            [3.0, 0.5, 0.0],
        ],
        faces: vec![[0, 1, 2], [3, 4, 5]],
        objects: vec![
            MeshlibSceneExportObject {
                object_name: "Base A".to_string(),
                object_key: "0_Base_A".to_string(),
                parent_key: "0_Root".to_string(),
                hierarchy_path: vec!["0_Root".to_string(), "0_Base_A".to_string()],
                model_file: "0_Root/0_Base_A.ply".to_string(),
                model_extension: ".ply".to_string(),
                link: None,
                shared_model_source_index: None,
                vertex_range: [0, 3],
                face_range: [0, 1],
                xf: MeshlibSceneXf {
                    row_x: [1.0, 0.0, 0.0],
                    row_y: [0.0, 1.0, 0.0],
                    row_z: [0.0, 0.0, 1.0],
                    b: [0.0, 0.0, 0.0],
                },
                visibility_mask: VIEWPORT_MASK_ALL,
                selected: false,
                locked: false,
                parent_locked: false,
            },
            MeshlibSceneExportObject {
                object_name: "Child B".to_string(),
                object_key: "0_Child_B".to_string(),
                parent_key: "0_Base_A".to_string(),
                hierarchy_path: vec![
                    "0_Root".to_string(),
                    "0_Base_A".to_string(),
                    "0_Child_B".to_string(),
                ],
                model_file: "0_Root/0_Base_A/0_Child_B.ply".to_string(),
                model_extension: ".ply".to_string(),
                link: None,
                shared_model_source_index: None,
                vertex_range: [3, 6],
                face_range: [1, 2],
                xf: MeshlibSceneXf {
                    row_x: [1.0, 0.0, 0.0],
                    row_y: [0.0, 1.0, 0.0],
                    row_z: [0.0, 0.0, 1.0],
                    b: [3.0, 0.0, 0.0],
                },
                visibility_mask: VIEWPORT_MASK_ALL,
                selected: false,
                locked: false,
                parent_locked: false,
            },
        ],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    };

    let bytes = meshlib_multi_object_mru_scene_bytes(&input).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert!(archive.by_name("0_Root/0_Base_A.ply").is_ok());
    assert!(archive.by_name("0_Root/0_Base_A/0_Child_B.ply").is_ok());
    let root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();

    assert_eq!(root["Children"]["0"]["Name"], "Base A");
    assert_eq!(root["Children"]["0"]["Children"]["0"]["Name"], "Child B");
    assert!(root["Children"].get("1").is_none());
}

#[test]
fn meshlib_mru_scene_round_trips_object_lines_nodes() {
    use std::io::Write as _;

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Profile Lines",
                "Key": "1_Profile_Lines",
                "Visibility": 0,
                "Selected": true,
                "Locked": true,
                "ParentLocked": false,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0}
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0}
                },
                "Type": ["Object", "VisualObject", "LinesHolder", "ObjectLines"],
                "ShowPoints": 4294967295_u32,
                "SmoothConnections": 0,
                "ColoringType": "PerLine",
                "LineColors": [],
                "VertColors": [],
                "LineWidth": 2.5,
                "Polyline": {
                    "Points": [
                        {"x": 0.0, "y": 0.0, "z": 0.0},
                        {"x": 1.0, "y": 0.0, "z": 0.0},
                        {"x": 1.0, "y": 1.0, "z": 0.0}
                    ],
                    "Lines": [0, 1, 1, 2]
                }
            }
        }
    });
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_line_objects.len(), 1);
    let line_object = &document.scene_line_objects[0];
    assert_eq!(line_object.object_name, "Profile Lines");
    assert_eq!(line_object.object_key, "1_Profile_Lines");
    assert_eq!(line_object.parent_key, "0_Root");
    assert_eq!(
        line_object.hierarchy_path,
        vec!["0_Root", "1_Profile_Lines"]
    );
    assert_eq!(
        line_object.points,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    );
    assert_eq!(line_object.lines, vec![[0, 1], [1, 2]]);
    assert_eq!(line_object.show_points, VIEWPORT_MASK_ALL);
    assert_eq!(line_object.smooth_connections, 0);
    assert_eq!(line_object.coloring_type, "PerLine");
    assert!((line_object.line_width - 2.5).abs() < f32::EPSILON);
    assert_eq!(line_object.visibility_mask, 0);
    assert!(line_object.selected);
    assert!(line_object.locked);
    assert!(!line_object.parent_locked);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: document.scene_line_objects.clone(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported)).unwrap();
    assert!(archive.by_name("0_Root/0_Base_A.ply").is_ok());
    let exported_root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let exported_lines = &exported_root["Children"]["1"];
    assert_eq!(exported_lines["Name"], "Profile Lines");
    assert_eq!(
        exported_lines["Type"],
        serde_json::json!(["Object", "VisualObject", "LinesHolder", "ObjectLines"])
    );
    assert_eq!(
        exported_lines["Polyline"]["Lines"],
        serde_json::json!([0, 1, 1, 2])
    );
    assert_eq!(exported_lines["Polyline"]["Points"][2]["y"], 1.0);
    assert_eq!(exported_lines["ShowPoints"], VIEWPORT_MASK_ALL);
    assert_eq!(exported_lines["LineWidth"], 2.5);
    assert_eq!(exported_lines["ColoringType"], "PerLine");
}

#[test]
fn meshlib_mru_scene_round_trips_object_points_nodes() {
    use std::io::Write as _;

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Probe Points",
                "Key": "1_Probe_Points",
                "Visibility": 0,
                "Selected": true,
                "Locked": true,
                "ParentLocked": false,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0}
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0}
                },
                "Type": ["Object", "VisualObject", "PointsHolder", "ObjectPoints"],
                "Colors": {
                    "Selection": {
                        "Points": {"x": 1.0, "y": 0.7, "z": 0.0, "w": 1.0}
                    }
                },
                "SelectionVertBitSet": {},
                "ValidVertBitSet": {},
                "PointSize": 7.0,
                "MaxRenderingPoints": 123
            }
        }
    });
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Probe_Points.ply", options)
            .unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\ncomment MeshInspector.com\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nproperty float nx\nproperty float ny\nproperty float nz\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n0 0 0 0 0 1 255 0 0\n1 0 0 0 1 0 0 255 0\n1 1 0 1 0 0 0 0 255\n",
            )
            .unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_point_objects.len(), 1);
    let point_object = &document.scene_point_objects[0];
    assert_eq!(point_object.object_name, "Probe Points");
    assert_eq!(point_object.object_key, "1_Probe_Points");
    assert_eq!(point_object.parent_key, "0_Root");
    assert_eq!(
        point_object.hierarchy_path,
        vec!["0_Root", "1_Probe_Points"]
    );
    assert_eq!(point_object.model_file, "0_Root/1_Probe_Points.ply");
    assert_eq!(point_object.model_extension, ".ply");
    assert_eq!(
        point_object.points,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    );
    assert_eq!(
        point_object.normals,
        vec![[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]]
    );
    assert_eq!(
        point_object.vert_colors,
        vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
    );
    assert!((point_object.point_size - 7.0).abs() < f32::EPSILON);
    assert_eq!(point_object.max_rendering_points, 123);
    assert_eq!(point_object.visibility_mask, 0);
    assert!(point_object.selected);
    assert!(point_object.locked);
    assert!(!point_object.parent_locked);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: document.scene_point_objects.clone(),
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported.clone())).unwrap();
    assert!(archive.by_name("0_Root/0_Base_A.ply").is_ok());
    assert!(archive.by_name("0_Root/1_Probe_Points.ply").is_ok());
    let exported_root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let exported_points = &exported_root["Children"]["1"];
    assert_eq!(exported_points["Name"], "Probe Points");
    assert_eq!(
        exported_points["Type"],
        serde_json::json!(["Object", "VisualObject", "PointsHolder", "ObjectPoints"])
    );
    assert_eq!(exported_points["PointSize"], 7.0);
    assert_eq!(exported_points["MaxRenderingPoints"], 123);
    assert_eq!(exported_points["Selected"], true);
    assert_eq!(exported_points["Locked"], true);

    let reloaded = meshlib_object_mesh_document_from_mru_scene_bytes(&exported).unwrap();
    assert_eq!(reloaded.scene_point_objects.len(), 1);
    assert_eq!(reloaded.scene_point_objects[0].points.len(), 3);
    assert_eq!(
        reloaded.scene_point_objects[0].vert_colors,
        vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
    );
}

#[test]
fn meshlib_mru_scene_round_trips_object_distance_map_nodes() {
    use std::io::Write as _;

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Depth Map",
                "Key": "1_Depth_Map",
                "Visibility": 0,
                "Selected": true,
                "Locked": true,
                "ParentLocked": false,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0}
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0}
                },
                "Type": ["Object", "VisualObject", "ObjectDistanceMap"],
                "PixelXVec": {"x": 0.5, "y": 0.0, "z": 0.0},
                "PixelYVec": {"x": 0.0, "y": 0.25, "z": 0.0},
                "DepthVec": {"x": 0.0, "y": 0.0, "z": 1.5},
                "OriginWorld": {"x": 1.0, "y": 2.0, "z": 3.0}
            }
        }
    });
    let mut raw_distance_map = Vec::new();
    raw_distance_map.extend_from_slice(&(2_u64).to_le_bytes());
    raw_distance_map.extend_from_slice(&(2_u64).to_le_bytes());
    for value in [0.0_f32, 1.0, DISTANCE_MAP_NOT_VALID_VALUE, 2.5] {
        raw_distance_map.extend_from_slice(&value.to_le_bytes());
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Depth_Map.raw", options)
            .unwrap();
        archive.write_all(&raw_distance_map).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_distance_map_objects.len(), 1);
    let distance_map_object = &document.scene_distance_map_objects[0];
    assert_eq!(distance_map_object.object_name, "Depth Map");
    assert_eq!(distance_map_object.object_key, "1_Depth_Map");
    assert_eq!(distance_map_object.parent_key, "0_Root");
    assert_eq!(
        distance_map_object.hierarchy_path,
        vec!["0_Root", "1_Depth_Map"]
    );
    assert_eq!(distance_map_object.model_file, "0_Root/1_Depth_Map.raw");
    assert_eq!(distance_map_object.model_extension, ".raw");
    assert_eq!(distance_map_object.width, 2);
    assert_eq!(distance_map_object.height, 2);
    assert_eq!(
        distance_map_object.values,
        vec![0.0, 1.0, DISTANCE_MAP_NOT_VALID_VALUE, 2.5]
    );
    assert_eq!(distance_map_object.valid_count, 3);
    assert!((distance_map_object.min_value - 0.0).abs() < f32::EPSILON);
    assert!((distance_map_object.max_value - 2.5).abs() < f32::EPSILON);
    assert_eq!(distance_map_object.pixel_x_vec, [0.5, 0.0, 0.0]);
    assert_eq!(distance_map_object.pixel_y_vec, [0.0, 0.25, 0.0]);
    assert_eq!(distance_map_object.depth_vec, [0.0, 0.0, 1.5]);
    assert_eq!(distance_map_object.origin_world, [1.0, 2.0, 3.0]);
    assert_eq!(distance_map_object.visibility_mask, 0);
    assert!(distance_map_object.selected);
    assert!(distance_map_object.locked);
    assert!(!distance_map_object.parent_locked);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: document.scene_distance_map_objects.clone(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported.clone())).unwrap();
    assert!(archive.by_name("0_Root/0_Base_A.ply").is_ok());
    assert!(archive.by_name("0_Root/1_Depth_Map.raw").is_ok());
    let exported_root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let exported_distance_map = &exported_root["Children"]["1"];
    assert_eq!(exported_distance_map["Name"], "Depth Map");
    assert_eq!(
        exported_distance_map["Type"],
        serde_json::json!(["Object", "VisualObject", "ObjectDistanceMap"])
    );
    assert_eq!(exported_distance_map["PixelXVec"]["x"], 0.5);
    assert_eq!(exported_distance_map["PixelYVec"]["y"], 0.25);
    assert_eq!(exported_distance_map["DepthVec"]["z"], 1.5);
    assert_eq!(exported_distance_map["OriginWorld"]["z"], 3.0);
    assert_eq!(exported_distance_map["Selected"], true);
    assert_eq!(exported_distance_map["Locked"], true);

    let reloaded = meshlib_object_mesh_document_from_mru_scene_bytes(&exported).unwrap();
    assert_eq!(reloaded.scene_distance_map_objects.len(), 1);
    assert_eq!(reloaded.scene_distance_map_objects[0].values[3], 2.5);
}

#[test]
fn meshlib_mru_scene_round_trips_object_voxels_nodes() {
    use std::io::Write as _;

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Visibility": 0,
                "Selected": true,
                "Locked": true,
                "ParentLocked": false,
                "XF": {
                    "A": {
                        "rowX": {"x": 1.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 1.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.0}
                    },
                    "b": {"x": 0.0, "y": 0.0, "z": 0.0}
                },
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 0.5, "y": 0.25, "z": 1.0},
                "Dimensions": {"x": 2, "y": 2, "z": 1},
                "MinCorner": {"x": 0, "y": 0, "z": 0},
                "MaxCorner": {"x": 2, "y": 2, "z": 1},
                "SelectionVoxels": {"size": 4, "bits": "CgAAAAAAAAA="},
                "IsoValue": 0.75,
                "DualMarchingCubes": false
            }
        }
    });
    let mut raw_voxels = Vec::new();
    for value in [0.0_f32, 0.5, 1.0, 1.5] {
        raw_voxels.extend_from_slice(&value.to_le_bytes());
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file(
                "0_Root/W2_H2_S1_V500_250_1000_G1_F 1_Scan_Voxels.raw",
                options,
            )
            .unwrap();
        archive.write_all(&raw_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.object_name, "Scan Voxels");
    assert_eq!(voxel_object.object_key, "1_Scan_Voxels");
    assert_eq!(voxel_object.parent_key, "0_Root");
    assert_eq!(voxel_object.hierarchy_path, vec!["0_Root", "1_Scan_Voxels"]);
    assert_eq!(
        voxel_object.model_file,
        "0_Root/W2_H2_S1_V500_250_1000_G1_F 1_Scan_Voxels.raw"
    );
    assert_eq!(voxel_object.model_extension, ".raw");
    assert_eq!(voxel_object.dimensions, [2, 2, 1]);
    assert_eq!(voxel_object.voxel_size, [0.5, 0.25, 1.0]);
    assert!(voxel_object.grid_level_set);
    assert_eq!(voxel_object.values, vec![0.0, 0.5, 1.0, 1.5]);
    assert!((voxel_object.min_value - 0.0).abs() < f32::EPSILON);
    assert!((voxel_object.max_value - 1.5).abs() < f32::EPSILON);
    assert_eq!(voxel_object.min_corner, [0, 0, 0]);
    assert_eq!(voxel_object.max_corner, [2, 2, 1]);
    assert!((voxel_object.iso_value - 0.75).abs() < f32::EPSILON);
    assert!(!voxel_object.dual_marching_cubes);
    assert_eq!(voxel_object.selected_voxels, vec![1, 3]);
    assert_eq!(voxel_object.visibility_mask, 0);
    assert!(voxel_object.selected);
    assert!(voxel_object.locked);
    assert!(!voxel_object.parent_locked);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: document.scene_voxel_objects.clone(),
        feature_objects: Vec::new(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported.clone())).unwrap();
    assert!(archive.by_name("0_Root/0_Base_A.ply").is_ok());
    assert!(archive
        .by_name("0_Root/W2_H2_S1_V500_250_1000_G1_F 1_Scan_Voxels.raw")
        .is_ok());
    let exported_root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let exported_voxels = &exported_root["Children"]["1"];
    assert_eq!(exported_voxels["Name"], "Scan Voxels");
    assert_eq!(
        exported_voxels["Type"],
        serde_json::json!(["Object", "VisualObject", "ObjectVoxels"])
    );
    assert_eq!(exported_voxels["VoxelSize"]["x"], 0.5);
    assert_eq!(exported_voxels["Dimensions"]["y"], 2);
    assert_eq!(exported_voxels["IsoValue"], 0.75);
    assert_eq!(exported_voxels["DualMarchingCubes"], false);
    assert_eq!(
        exported_voxels["SelectionVoxels"],
        serde_json::json!({"size": 4, "bits": "CgAAAAAAAAA="})
    );
    assert_eq!(exported_voxels["Selected"], true);
    assert_eq!(exported_voxels["Locked"], true);

    let reloaded = meshlib_object_mesh_document_from_mru_scene_bytes(&exported).unwrap();
    assert_eq!(reloaded.scene_voxel_objects.len(), 1);
    assert_eq!(reloaded.scene_voxel_objects[0].values[3], 1.5);
    assert_eq!(reloaded.scene_voxel_objects[0].selected_voxels, vec![1, 3]);
}

#[test]
fn meshlib_mru_scene_round_trips_object_voxels_gav_nodes() {
    use std::io::{Read as _, Write as _};

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 1.0, "y": 1.5, "z": 2.0},
                "Dimensions": {"x": 2, "y": 2, "z": 1},
                "MinCorner": {"x": 0, "y": 0, "z": 0},
                "MaxCorner": {"x": 2, "y": 2, "z": 1},
                "IsoValue": 0.5,
                "DualMarchingCubes": true
            }
        }
    });
    let header = serde_json::json!({
        "ValueType": "Float",
        "Dimensions": {"X": 2, "Y": 2, "Z": 1},
        "VoxelSize": {"X": 1.0, "Y": 1.5, "Z": 2.0},
        "Range": {"Min": -1.0, "Max": 2.0}
    })
    .to_string();
    let mut gav_voxels = Vec::new();
    gav_voxels.extend_from_slice(&(header.len() as u32).to_le_bytes());
    gav_voxels.extend_from_slice(header.as_bytes());
    for value in [-1.0_f32, 0.0, 1.0, 2.0] {
        gav_voxels.extend_from_slice(&value.to_le_bytes());
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Scan_Voxels.gav", options)
            .unwrap();
        archive.write_all(&gav_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.model_file, "0_Root/1_Scan_Voxels.gav");
    assert_eq!(voxel_object.model_extension, ".gav");
    assert_eq!(voxel_object.dimensions, [2, 2, 1]);
    assert_eq!(voxel_object.voxel_size, [1.0, 1.5, 2.0]);
    assert_eq!(voxel_object.values, vec![-1.0, 0.0, 1.0, 2.0]);
    assert!((voxel_object.min_value - -1.0).abs() < f32::EPSILON);
    assert!((voxel_object.max_value - 2.0).abs() < f32::EPSILON);
    assert!(voxel_object.dual_marching_cubes);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: document.scene_voxel_objects.clone(),
        feature_objects: Vec::new(),
    })
    .unwrap();

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported.clone())).unwrap();
    let mut exported_gav = Vec::new();
    archive
        .by_name("0_Root/1_Scan_Voxels.gav")
        .unwrap()
        .read_to_end(&mut exported_gav)
        .unwrap();
    let header_len = u32::from_le_bytes(exported_gav[0..4].try_into().unwrap()) as usize;
    let header_json: serde_json::Value =
        serde_json::from_slice(&exported_gav[4..4 + header_len]).unwrap();
    assert_eq!(header_json["ValueType"], "Float");
    assert_eq!(header_json["Dimensions"]["X"], 2);
    assert_eq!(header_json["VoxelSize"]["Y"], 1.5);
    assert_eq!(header_json["Range"]["Min"], -1.0);
    assert_eq!(header_json["Range"]["Max"], 2.0);

    let reloaded = meshlib_object_mesh_document_from_mru_scene_bytes(&exported).unwrap();
    assert_eq!(reloaded.scene_voxel_objects.len(), 1);
    assert_eq!(
        reloaded.scene_voxel_objects[0].values,
        vec![-1.0, 0.0, 1.0, 2.0]
    );
    assert_eq!(reloaded.scene_voxel_objects[0].model_extension, ".gav");
}

#[test]
fn meshlib_mru_scene_round_trips_object_voxels_vdb_payloads() {
    use std::io::{Read as _, Write as _};

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Visibility": 4294967295_u32,
                "Selected": true,
                "Locked": false,
                "ParentLocked": false,
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 0.25, "y": 0.5, "z": 1.0},
                "Dimensions": {"x": 2, "y": 2, "z": 1},
                "MinCorner": {"x": 0, "y": 0, "z": 0},
                "MaxCorner": {"x": 2, "y": 2, "z": 1},
                "SelectionVoxels": {"size": 4, "bits": "CgAAAAAAAAA="},
                "IsoValue": 0.125,
                "DualMarchingCubes": true
            }
        }
    });
    let vdb_voxels = b"OPENVDB_OPAQUE_PAYLOAD_FOR_MRU_SCENE_ROUNDTRIP".to_vec();
    let voxel_file = "0_Root/1_Scan_Voxels.vdb";

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive.start_file(voxel_file, options).unwrap();
        archive.write_all(&vdb_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.model_file, voxel_file);
    assert_eq!(voxel_object.model_extension, ".vdb");
    assert_eq!(voxel_object.dimensions, [2, 2, 1]);
    assert_eq!(voxel_object.voxel_size, [0.25, 0.5, 1.0]);
    assert!(voxel_object.values.is_empty());
    assert_eq!(voxel_object.model_bytes, vdb_voxels);
    assert_eq!(voxel_object.selected_voxels, vec![1, 3]);
    assert!(voxel_object.dual_marching_cubes);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: document.scene_voxel_objects.clone(),
        feature_objects: Vec::new(),
    })
    .unwrap();

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported.clone())).unwrap();
    let mut exported_vdb = Vec::new();
    archive
        .by_name(voxel_file)
        .unwrap()
        .read_to_end(&mut exported_vdb)
        .unwrap();
    assert_eq!(exported_vdb, vdb_voxels);
    let exported_root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let exported_voxels = &exported_root["Children"]["1"];
    assert_eq!(exported_voxels["VoxelSize"]["x"], 0.25);
    assert_eq!(exported_voxels["Dimensions"]["z"], 1);
    assert_eq!(exported_voxels["IsoValue"], 0.125);
    assert_eq!(exported_voxels["DualMarchingCubes"], true);
    assert_eq!(
        exported_voxels["SelectionVoxels"],
        serde_json::json!({"size": 4, "bits": "CgAAAAAAAAA="})
    );

    let reloaded = meshlib_object_mesh_document_from_mru_scene_bytes(&exported).unwrap();
    assert_eq!(reloaded.scene_voxel_objects[0].model_extension, ".vdb");
    assert_eq!(reloaded.scene_voxel_objects[0].model_bytes, vdb_voxels);
    assert_eq!(reloaded.scene_voxel_objects[0].selected_voxels, vec![1, 3]);
}

#[test]
fn meshlib_mru_scene_imports_object_voxels_vdb_metadata_from_openvdb_header() {
    use std::io::Write as _;

    fn push_u8(bytes: &mut Vec<u8>, value: u8) {
        bytes.push(value);
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f64(bytes: &mut Vec<u8>, value: f64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
        push_string(bytes, name);
        push_string(bytes, "string");
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_i64(bytes: &mut Vec<u8>, name: &str, value: i64) {
        push_string(bytes, name);
        push_string(bytes, "int64");
        push_u32(bytes, 8);
        push_i64(bytes, value);
    }

    fn push_metadata_vec3i(bytes: &mut Vec<u8>, name: &str, value: [i32; 3]) {
        push_string(bytes, name);
        push_string(bytes, "vec3i");
        push_u32(bytes, 12);
        for component in value {
            push_i32(bytes, component);
        }
    }

    fn push_dvec3(bytes: &mut Vec<u8>, value: [f64; 3]) {
        for component in value {
            push_f64(bytes, component);
        }
    }

    fn synthetic_openvdb_level_set_header() -> Vec<u8> {
        let mut grid = Vec::new();
        push_u32(&mut grid, 0); // per-grid compression
        push_u32(&mut grid, 5);
        push_metadata_vec3i(&mut grid, "file_bbox_min", [-2, -3, 4]);
        push_metadata_vec3i(&mut grid, "file_bbox_max", [3, 1, 6]);
        push_metadata_i64(&mut grid, "file_voxel_count", 90);
        push_metadata_string(&mut grid, "value_type", "float");
        push_metadata_string(&mut grid, "class", "level set");
        push_string(&mut grid, "UniformScaleMap");
        push_dvec3(&mut grid, [0.125, 0.25, 0.5]);
        push_dvec3(&mut grid, [0.125, 0.25, 0.5]);
        push_dvec3(&mut grid, [8.0, 4.0, 2.0]);
        push_dvec3(&mut grid, [64.0, 16.0, 4.0]);
        push_dvec3(&mut grid, [4.0, 2.0, 1.0]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00]);
        push_u32(&mut bytes, 223);
        push_u32(&mut bytes, 12);
        push_u32(&mut bytes, 0);
        push_u8(&mut bytes, 1);
        bytes.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
        push_u32(&mut bytes, 0); // archive metadata count
        push_u32(&mut bytes, 1); // grid count
        push_string(&mut bytes, "ls_scan");
        push_string(&mut bytes, "Tree_float_5_4_3");
        push_string(&mut bytes, "");
        let grid_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let block_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let end_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);

        let grid_pos = bytes.len() as u64;
        bytes.extend_from_slice(&grid);
        let end_pos = bytes.len() as u64;
        bytes[grid_pos_offset..grid_pos_offset + 8].copy_from_slice(&grid_pos.to_le_bytes());
        bytes[block_pos_offset..block_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes[end_pos_offset..end_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes
    }

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1}
            }
        }
    });
    let vdb_voxels = synthetic_openvdb_level_set_header();

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Scan_Voxels.vdb", options)
            .unwrap();
        archive.write_all(&vdb_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.model_extension, ".vdb");
    assert_eq!(voxel_object.dimensions, [6, 5, 3]);
    assert_eq!(voxel_object.voxel_size, [0.125, 0.25, 0.5]);
    assert!(voxel_object.grid_level_set);
    assert!(voxel_object.values.is_empty());
    assert_eq!(voxel_object.model_bytes, vdb_voxels);
}

#[test]
fn meshlib_mru_scene_imports_object_voxels_vdb_dense_float_leaf_values() {
    use std::io::Write as _;

    fn push_u8(bytes: &mut Vec<u8>, value: u8) {
        bytes.push(value);
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f64(bytes: &mut Vec<u8>, value: f64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
        push_string(bytes, name);
        push_string(bytes, "string");
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_i64(bytes: &mut Vec<u8>, name: &str, value: i64) {
        push_string(bytes, name);
        push_string(bytes, "int64");
        push_u32(bytes, 8);
        push_i64(bytes, value);
    }

    fn push_metadata_vec3i(bytes: &mut Vec<u8>, name: &str, value: [i32; 3]) {
        push_string(bytes, name);
        push_string(bytes, "vec3i");
        push_u32(bytes, 12);
        for component in value {
            push_i32(bytes, component);
        }
    }

    fn push_dvec3(bytes: &mut Vec<u8>, value: [f64; 3]) {
        for component in value {
            push_f64(bytes, component);
        }
    }

    fn push_node_mask(bytes: &mut Vec<u8>, log2_dim: usize, enabled_offsets: &[usize]) {
        let bit_count = 1usize << (3 * log2_dim);
        let byte_count = bit_count / 8;
        let mut mask = vec![0_u8; byte_count];
        for offset in enabled_offsets {
            mask[*offset / 8] |= 1_u8 << (*offset % 8);
        }
        bytes.extend_from_slice(&mask);
    }

    fn push_uncompressed_float_values(bytes: &mut Vec<u8>, count: usize, value: f32) {
        push_u8(bytes, 6); // OpenVDB NO_MASK_AND_ALL_VALS
        for _ in 0..count {
            push_f32(bytes, value);
        }
    }

    fn synthetic_openvdb_single_dense_leaf(values: &[f32]) -> Vec<u8> {
        assert_eq!(values.len(), 512);

        let mut grid = Vec::new();
        push_u32(&mut grid, 0); // per-grid compression: COMPRESS_NONE
        push_u32(&mut grid, 5);
        push_metadata_vec3i(&mut grid, "file_bbox_min", [0, 0, 0]);
        push_metadata_vec3i(&mut grid, "file_bbox_max", [7, 7, 7]);
        push_metadata_i64(&mut grid, "file_voxel_count", 512);
        push_metadata_string(&mut grid, "value_type", "float");
        push_metadata_string(&mut grid, "class", "level set");
        push_string(&mut grid, "UniformScaleMap");
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [2.0, 2.0, 2.0]);
        push_dvec3(&mut grid, [4.0, 4.0, 4.0]);
        push_dvec3(&mut grid, [1.0, 1.0, 1.0]);

        push_f32(&mut grid, 1000.0); // root background
        push_u32(&mut grid, 0); // root tile count
        push_u32(&mut grid, 1); // root child count
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);

        push_node_mask(&mut grid, 5, &[0]); // root child internal node has one child
        push_node_mask(&mut grid, 5, &[]); // no active internal tiles
        push_uncompressed_float_values(&mut grid, 1 << 15, 1000.0);
        push_node_mask(&mut grid, 4, &[0]); // second-level internal node has one leaf
        push_node_mask(&mut grid, 4, &[]); // no active internal tiles
        push_uncompressed_float_values(&mut grid, 1 << 12, 1000.0);
        push_node_mask(&mut grid, 3, &(0..512).collect::<Vec<_>>()); // leaf topology

        push_node_mask(&mut grid, 3, &(0..512).collect::<Vec<_>>()); // leaf buffer mask
        push_u8(&mut grid, 6); // OpenVDB NO_MASK_AND_ALL_VALS
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    let dense_index = x + y * 8 + z * 64;
                    push_f32(&mut grid, values[dense_index]);
                }
            }
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00]);
        push_u32(&mut bytes, 223);
        push_u32(&mut bytes, 12);
        push_u32(&mut bytes, 0);
        push_u8(&mut bytes, 1);
        bytes.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
        push_u32(&mut bytes, 0); // archive metadata count
        push_u32(&mut bytes, 1); // grid count
        push_string(&mut bytes, "dense_leaf");
        push_string(&mut bytes, "Tree_float_5_4_3");
        push_string(&mut bytes, "");
        let grid_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let block_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let end_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);

        let grid_pos = bytes.len() as u64;
        bytes.extend_from_slice(&grid);
        let end_pos = bytes.len() as u64;
        bytes[grid_pos_offset..grid_pos_offset + 8].copy_from_slice(&grid_pos.to_le_bytes());
        bytes[block_pos_offset..block_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes[end_pos_offset..end_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes
    }

    let expected_values = (0..512)
        .map(|index| {
            let z = index / 64;
            let y = (index % 64) / 8;
            let x = index % 8;
            x as f32 + y as f32 * 10.0 + z as f32 * 100.0
        })
        .collect::<Vec<_>>();
    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1}
            }
        }
    });
    let vdb_voxels = synthetic_openvdb_single_dense_leaf(&expected_values);

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Scan_Voxels.vdb", options)
            .unwrap();
        archive.write_all(&vdb_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.dimensions, [8, 8, 8]);
    assert_eq!(voxel_object.voxel_size, [0.5, 0.5, 0.5]);
    assert!(voxel_object.grid_level_set);
    assert_eq!(voxel_object.values, expected_values);
    assert_eq!(voxel_object.min_value, 0.0);
    assert_eq!(voxel_object.max_value, 777.0);
}

#[test]
fn meshlib_mru_scene_imports_object_voxels_vdb_half_float_active_mask_leaf_values() {
    use std::io::Write as _;

    fn push_u8(bytes: &mut Vec<u8>, value: u8) {
        bytes.push(value);
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f64(bytes: &mut Vec<u8>, value: f64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
        push_string(bytes, name);
        push_string(bytes, "string");
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_i64(bytes: &mut Vec<u8>, name: &str, value: i64) {
        push_string(bytes, name);
        push_string(bytes, "int64");
        push_u32(bytes, 8);
        push_i64(bytes, value);
    }

    fn push_metadata_vec3i(bytes: &mut Vec<u8>, name: &str, value: [i32; 3]) {
        push_string(bytes, name);
        push_string(bytes, "vec3i");
        push_u32(bytes, 12);
        for component in value {
            push_i32(bytes, component);
        }
    }

    fn push_dvec3(bytes: &mut Vec<u8>, value: [f64; 3]) {
        for component in value {
            push_f64(bytes, component);
        }
    }

    fn push_node_mask(bytes: &mut Vec<u8>, log2_dim: usize, enabled_offsets: &[usize]) {
        let bit_count = 1usize << (3 * log2_dim);
        let byte_count = bit_count / 8;
        let mut mask = vec![0_u8; byte_count];
        for offset in enabled_offsets {
            mask[*offset / 8] |= 1_u8 << (*offset % 8);
        }
        bytes.extend_from_slice(&mask);
    }

    fn push_active_mask_values_header(bytes: &mut Vec<u8>) {
        push_u8(bytes, 0); // OpenVDB NO_MASK_OR_INACTIVE_VALS
    }

    fn synthetic_openvdb_single_half_active_leaf() -> Vec<u8> {
        let active_offsets = [0usize, 83usize, 511usize];

        let mut grid = Vec::new();
        push_u32(&mut grid, 2); // per-grid compression: COMPRESS_ACTIVE_MASK
        push_u32(&mut grid, 5);
        push_metadata_vec3i(&mut grid, "file_bbox_min", [0, 0, 0]);
        push_metadata_vec3i(&mut grid, "file_bbox_max", [7, 7, 7]);
        push_metadata_i64(&mut grid, "file_voxel_count", 3);
        push_metadata_string(&mut grid, "value_type", "float");
        push_metadata_string(&mut grid, "class", "level set");
        push_string(&mut grid, "UniformScaleMap");
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [2.0, 2.0, 2.0]);
        push_dvec3(&mut grid, [4.0, 4.0, 4.0]);
        push_dvec3(&mut grid, [1.0, 1.0, 1.0]);

        push_f32(&mut grid, 9.0); // root background
        push_u32(&mut grid, 0); // root tile count
        push_u32(&mut grid, 1); // root child count
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);

        push_node_mask(&mut grid, 5, &[0]); // root internal child
        push_node_mask(&mut grid, 5, &[]); // no active root internal tiles
        push_active_mask_values_header(&mut grid);
        push_node_mask(&mut grid, 4, &[0]); // second internal child
        push_node_mask(&mut grid, 4, &[]); // no active second internal tiles
        push_active_mask_values_header(&mut grid);
        push_node_mask(&mut grid, 3, &active_offsets); // leaf topology value mask

        push_node_mask(&mut grid, 3, &active_offsets); // leaf buffer value mask
        push_active_mask_values_header(&mut grid);
        push_u16(&mut grid, 0x3c00); // 1.0 half
        push_u16(&mut grid, 0x4100); // 2.5 half
        push_u16(&mut grid, 0xc200); // -3.0 half

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00]);
        push_u32(&mut bytes, 223);
        push_u32(&mut bytes, 12);
        push_u32(&mut bytes, 0);
        push_u8(&mut bytes, 1);
        bytes.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
        push_u32(&mut bytes, 0); // archive metadata count
        push_u32(&mut bytes, 1); // grid count
        push_string(&mut bytes, "half_active_leaf");
        push_string(&mut bytes, "Tree_float_5_4_3_HalfFloat");
        push_string(&mut bytes, "");
        let grid_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let block_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let end_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);

        let grid_pos = bytes.len() as u64;
        bytes.extend_from_slice(&grid);
        let end_pos = bytes.len() as u64;
        bytes[grid_pos_offset..grid_pos_offset + 8].copy_from_slice(&grid_pos.to_le_bytes());
        bytes[block_pos_offset..block_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes[end_pos_offset..end_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes
    }

    let mut expected_values = vec![9.0; 512];
    expected_values[0] = 1.0;
    expected_values[1 + 2 * 8 + 3 * 64] = 2.5;
    expected_values[7 + 7 * 8 + 7 * 64] = -3.0;
    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1}
            }
        }
    });
    let vdb_voxels = synthetic_openvdb_single_half_active_leaf();

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Scan_Voxels.vdb", options)
            .unwrap();
        archive.write_all(&vdb_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.dimensions, [8, 8, 8]);
    assert_eq!(voxel_object.voxel_size, [0.5, 0.5, 0.5]);
    assert!(voxel_object.grid_level_set);
    assert_eq!(voxel_object.values, expected_values);
    assert_eq!(voxel_object.min_value, -3.0);
    assert_eq!(voxel_object.max_value, 9.0);
}

#[test]
fn meshlib_mru_scene_imports_object_voxels_vdb_zip_compressed_leaf_values() {
    use std::io::Write as _;

    fn push_u8(bytes: &mut Vec<u8>, value: u8) {
        bytes.push(value);
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f64(bytes: &mut Vec<u8>, value: f64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
        push_string(bytes, name);
        push_string(bytes, "string");
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_i64(bytes: &mut Vec<u8>, name: &str, value: i64) {
        push_string(bytes, name);
        push_string(bytes, "int64");
        push_u32(bytes, 8);
        push_i64(bytes, value);
    }

    fn push_metadata_vec3i(bytes: &mut Vec<u8>, name: &str, value: [i32; 3]) {
        push_string(bytes, name);
        push_string(bytes, "vec3i");
        push_u32(bytes, 12);
        for component in value {
            push_i32(bytes, component);
        }
    }

    fn push_dvec3(bytes: &mut Vec<u8>, value: [f64; 3]) {
        for component in value {
            push_f64(bytes, component);
        }
    }

    fn push_node_mask(bytes: &mut Vec<u8>, log2_dim: usize, enabled_offsets: &[usize]) {
        let bit_count = 1usize << (3 * log2_dim);
        let byte_count = bit_count / 8;
        let mut mask = vec![0_u8; byte_count];
        for offset in enabled_offsets {
            mask[*offset / 8] |= 1_u8 << (*offset % 8);
        }
        bytes.extend_from_slice(&mask);
    }

    fn push_zip_uncompressed_float_values(bytes: &mut Vec<u8>, count: usize, value: f32) {
        let byte_count = count.checked_mul(4).unwrap();
        push_u8(bytes, 6); // OpenVDB NO_MASK_AND_ALL_VALS
        push_i64(bytes, -(byte_count as i64));
        for _ in 0..count {
            push_f32(bytes, value);
        }
    }

    fn push_zip_compressed_leaf_values(bytes: &mut Vec<u8>) {
        const COMPRESSED_LEAF_VALUES: &[u8] = &[
            120, 156, 99, 96, 104, 176, 103, 24, 5, 84, 2, 10, 14, 3, 237, 130, 81, 48, 10, 70,
            193, 40, 24, 5, 163, 128, 48, 112, 56, 0, 0, 130, 123, 2, 32,
        ];
        push_u8(bytes, 6); // OpenVDB NO_MASK_AND_ALL_VALS
        push_i64(bytes, COMPRESSED_LEAF_VALUES.len() as i64);
        bytes.extend_from_slice(COMPRESSED_LEAF_VALUES);
    }

    fn synthetic_openvdb_single_zip_dense_leaf() -> Vec<u8> {
        let mut grid = Vec::new();
        push_u32(&mut grid, 1); // per-grid compression: COMPRESS_ZIP
        push_u32(&mut grid, 5);
        push_metadata_vec3i(&mut grid, "file_bbox_min", [0, 0, 0]);
        push_metadata_vec3i(&mut grid, "file_bbox_max", [7, 7, 7]);
        push_metadata_i64(&mut grid, "file_voxel_count", 512);
        push_metadata_string(&mut grid, "value_type", "float");
        push_metadata_string(&mut grid, "class", "level set");
        push_string(&mut grid, "UniformScaleMap");
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [2.0, 2.0, 2.0]);
        push_dvec3(&mut grid, [4.0, 4.0, 4.0]);
        push_dvec3(&mut grid, [1.0, 1.0, 1.0]);

        push_f32(&mut grid, 1000.0); // root background
        push_u32(&mut grid, 0); // root tile count
        push_u32(&mut grid, 1); // root child count
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);

        push_node_mask(&mut grid, 5, &[0]); // root child internal node has one child
        push_node_mask(&mut grid, 5, &[]); // no active internal tiles
        push_zip_uncompressed_float_values(&mut grid, 1 << 15, 1000.0);
        push_node_mask(&mut grid, 4, &[0]); // second-level internal node has one leaf
        push_node_mask(&mut grid, 4, &[]); // no active internal tiles
        push_zip_uncompressed_float_values(&mut grid, 1 << 12, 1000.0);
        push_node_mask(&mut grid, 3, &(0..512).collect::<Vec<_>>()); // leaf topology

        push_node_mask(&mut grid, 3, &(0..512).collect::<Vec<_>>()); // leaf buffer mask
        push_zip_compressed_leaf_values(&mut grid);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00]);
        push_u32(&mut bytes, 223);
        push_u32(&mut bytes, 12);
        push_u32(&mut bytes, 0);
        push_u8(&mut bytes, 1);
        bytes.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
        push_u32(&mut bytes, 0); // archive metadata count
        push_u32(&mut bytes, 1); // grid count
        push_string(&mut bytes, "zip_dense_leaf");
        push_string(&mut bytes, "Tree_float_5_4_3");
        push_string(&mut bytes, "");
        let grid_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let block_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let end_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);

        let grid_pos = bytes.len() as u64;
        bytes.extend_from_slice(&grid);
        let end_pos = bytes.len() as u64;
        bytes[grid_pos_offset..grid_pos_offset + 8].copy_from_slice(&grid_pos.to_le_bytes());
        bytes[block_pos_offset..block_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes[end_pos_offset..end_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes
    }

    let mut expected_values = vec![0.0; 512];
    expected_values[0] = 1.0;
    expected_values[1 + 2 * 8 + 3 * 64] = 2.5;
    expected_values[7 + 7 * 8 + 7 * 64] = -3.0;
    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1}
            }
        }
    });
    let vdb_voxels = synthetic_openvdb_single_zip_dense_leaf();

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Scan_Voxels.vdb", options)
            .unwrap();
        archive.write_all(&vdb_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.dimensions, [8, 8, 8]);
    assert_eq!(voxel_object.voxel_size, [0.5, 0.5, 0.5]);
    assert!(voxel_object.grid_level_set);
    assert_eq!(voxel_object.values, expected_values);
    assert_eq!(voxel_object.min_value, -3.0);
    assert_eq!(voxel_object.max_value, 2.5);
}

#[test]
fn meshlib_mru_scene_imports_object_voxels_vdb_blosc_compressed_leaf_values() {
    use std::io::Write as _;

    fn push_u8(bytes: &mut Vec<u8>, value: u8) {
        bytes.push(value);
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f64(bytes: &mut Vec<u8>, value: f64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
        push_string(bytes, name);
        push_string(bytes, "string");
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_i64(bytes: &mut Vec<u8>, name: &str, value: i64) {
        push_string(bytes, name);
        push_string(bytes, "int64");
        push_u32(bytes, 8);
        push_i64(bytes, value);
    }

    fn push_metadata_vec3i(bytes: &mut Vec<u8>, name: &str, value: [i32; 3]) {
        push_string(bytes, name);
        push_string(bytes, "vec3i");
        push_u32(bytes, 12);
        for component in value {
            push_i32(bytes, component);
        }
    }

    fn push_dvec3(bytes: &mut Vec<u8>, value: [f64; 3]) {
        for component in value {
            push_f64(bytes, component);
        }
    }

    fn push_node_mask(bytes: &mut Vec<u8>, log2_dim: usize, enabled_offsets: &[usize]) {
        let bit_count = 1usize << (3 * log2_dim);
        let byte_count = bit_count / 8;
        let mut mask = vec![0_u8; byte_count];
        for offset in enabled_offsets {
            mask[*offset / 8] |= 1_u8 << (*offset % 8);
        }
        bytes.extend_from_slice(&mask);
    }

    fn push_blosc_uncompressed_float_values(bytes: &mut Vec<u8>, count: usize, value: f32) {
        let byte_count = count.checked_mul(4).unwrap();
        push_u8(bytes, 6); // OpenVDB NO_MASK_AND_ALL_VALS
        push_i64(bytes, -(byte_count as i64));
        for _ in 0..count {
            push_f32(bytes, value);
        }
    }

    fn push_blosc_compressed_leaf_values(bytes: &mut Vec<u8>) {
        static BLOSC_TEST_INIT: std::sync::Once = std::sync::Once::new();
        BLOSC_TEST_INIT.call_once(|| unsafe {
            blosc_src::blosc_init();
        });

        let mut raw_values = Vec::with_capacity(512 * 4);
        for offset in 0..512 {
            let value = match offset {
                0 => 1.0,
                83 => 2.5,
                511 => -3.0,
                _ => 0.0,
            };
            push_f32(&mut raw_values, value);
        }

        let mut compressed = vec![0_u8; raw_values.len() + blosc_src::BLOSC_MAX_OVERHEAD as usize];
        let compressed_len = unsafe {
            blosc_src::blosc_compress_ctx(
                9,
                blosc_src::BLOSC_SHUFFLE as i32,
                4,
                raw_values.len(),
                raw_values.as_ptr().cast(),
                compressed.as_mut_ptr().cast(),
                compressed.len(),
                blosc_src::BLOSC_LZ4_COMPNAME.as_ptr().cast(),
                raw_values.len(),
                1,
            )
        };
        assert!(
            compressed_len > 0,
            "OpenVDB Blosc/LZ4 test fixture compression failed: {compressed_len}"
        );
        compressed.truncate(compressed_len as usize);

        push_u8(bytes, 6); // OpenVDB NO_MASK_AND_ALL_VALS
        push_i64(bytes, compressed.len() as i64);
        bytes.extend_from_slice(&compressed);
    }

    fn synthetic_openvdb_single_blosc_dense_leaf() -> Vec<u8> {
        let mut grid = Vec::new();
        push_u32(&mut grid, 4); // per-grid compression: COMPRESS_BLOSC
        push_u32(&mut grid, 5);
        push_metadata_vec3i(&mut grid, "file_bbox_min", [0, 0, 0]);
        push_metadata_vec3i(&mut grid, "file_bbox_max", [7, 7, 7]);
        push_metadata_i64(&mut grid, "file_voxel_count", 512);
        push_metadata_string(&mut grid, "value_type", "float");
        push_metadata_string(&mut grid, "class", "level set");
        push_string(&mut grid, "UniformScaleMap");
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [2.0, 2.0, 2.0]);
        push_dvec3(&mut grid, [4.0, 4.0, 4.0]);
        push_dvec3(&mut grid, [1.0, 1.0, 1.0]);

        push_f32(&mut grid, 1000.0); // root background
        push_u32(&mut grid, 0); // root tile count
        push_u32(&mut grid, 1); // root child count
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);

        push_node_mask(&mut grid, 5, &[0]); // root child internal node has one child
        push_node_mask(&mut grid, 5, &[]); // no active internal tiles
        push_blosc_uncompressed_float_values(&mut grid, 1 << 15, 1000.0);
        push_node_mask(&mut grid, 4, &[0]); // second-level internal node has one leaf
        push_node_mask(&mut grid, 4, &[]); // no active internal tiles
        push_blosc_uncompressed_float_values(&mut grid, 1 << 12, 1000.0);
        push_node_mask(&mut grid, 3, &(0..512).collect::<Vec<_>>()); // leaf topology

        push_node_mask(&mut grid, 3, &(0..512).collect::<Vec<_>>()); // leaf buffer mask
        push_blosc_compressed_leaf_values(&mut grid);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00]);
        push_u32(&mut bytes, 223);
        push_u32(&mut bytes, 12);
        push_u32(&mut bytes, 0);
        push_u8(&mut bytes, 1);
        bytes.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
        push_u32(&mut bytes, 0); // archive metadata count
        push_u32(&mut bytes, 1); // grid count
        push_string(&mut bytes, "blosc_dense_leaf");
        push_string(&mut bytes, "Tree_float_5_4_3");
        push_string(&mut bytes, "");
        let grid_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let block_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let end_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);

        let grid_pos = bytes.len() as u64;
        bytes.extend_from_slice(&grid);
        let end_pos = bytes.len() as u64;
        bytes[grid_pos_offset..grid_pos_offset + 8].copy_from_slice(&grid_pos.to_le_bytes());
        bytes[block_pos_offset..block_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes[end_pos_offset..end_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes
    }

    let mut expected_values = vec![0.0; 512];
    expected_values[0] = 1.0;
    expected_values[1 + 2 * 8 + 3 * 64] = 2.5;
    expected_values[7 + 7 * 8 + 7 * 64] = -3.0;
    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Scan Voxels",
                "Key": "1_Scan_Voxels",
                "Type": ["Object", "VisualObject", "ObjectVoxels"],
                "VoxelSize": {"x": 9.0, "y": 9.0, "z": 9.0},
                "Dimensions": {"x": 1, "y": 1, "z": 1}
            }
        }
    });
    let vdb_voxels = synthetic_openvdb_single_blosc_dense_leaf();

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive
            .start_file("0_Root/1_Scan_Voxels.vdb", options)
            .unwrap();
        archive.write_all(&vdb_voxels).unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_voxel_objects.len(), 1);
    let voxel_object = &document.scene_voxel_objects[0];
    assert_eq!(voxel_object.dimensions, [8, 8, 8]);
    assert_eq!(voxel_object.voxel_size, [0.5, 0.5, 0.5]);
    assert!(voxel_object.grid_level_set);
    assert_eq!(voxel_object.values, expected_values);
    assert_eq!(voxel_object.min_value, -3.0);
    assert_eq!(voxel_object.max_value, 2.5);
}

#[test]
fn meshlib_mru_scene_round_trips_feature_object_nodes() {
    use std::io::Write as _;

    let root = serde_json::json!({
        "FormatVersion": 1.0,
        "Name": "Root",
        "Key": "0_Root",
        "Type": ["Object", "RootObject"],
        "Children": {
            "0": {
                "Name": "Base A",
                "Key": "0_Base_A",
                "Type": ["Object", "VisualObject", "MeshHolder", "ObjectMesh"]
            },
            "1": {
                "Name": "Plane Feature",
                "Key": "1_Plane_Feature",
                "Visibility": 0,
                "Selected": true,
                "Locked": true,
                "ParentLocked": false,
                "XF": {
                    "A": {
                        "rowX": {"x": 2.0, "y": 0.0, "z": 0.0},
                        "rowY": {"x": 0.0, "y": 3.0, "z": 0.0},
                        "rowZ": {"x": 0.0, "y": 0.0, "z": 1.5}
                    },
                    "b": {"x": 1.0, "y": 2.0, "z": 3.0}
                },
                "Type": ["Object", "VisualObject", "FeatureObject", "PlaneObject"],
                "SubfeatureVisibility": 15,
                "DetailsOnNameTag": 7,
                "DecorationsColorUnselected": {"x": 0.1, "y": 0.2, "z": 0.3, "w": 0.4},
                "DecorationsColorSelected": {"x": 0.5, "y": 0.6, "z": 0.7, "w": 0.8},
                "PointSize": 11.0,
                "LineWidth": 2.5,
                "SubPointSize": 6.5,
                "SubLineWidth": 1.5,
                "MainAlpha": 0.9,
                "SubAlphaPoints": 0.8,
                "SubAlphaLines": 0.7,
                "SubAlphaMesh": 0.6,
                "DimensionVisibility": {
                    "Length": 3,
                    "Angle": 5
                }
            }
        }
    });

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default();
        archive.start_file("Root.json", options).unwrap();
        archive.write_all(root.to_string().as_bytes()).unwrap();
        archive.start_file("0_Root/0_Base_A.ply", options).unwrap();
        archive
            .write_all(
                b"ply\nformat ascii 1.0\nelement vertex 3\nproperty double x\nproperty double y\nproperty double z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n",
            )
            .unwrap();
        archive.finish().unwrap();
    }

    let document = meshlib_object_mesh_document_from_mru_scene_bytes(&cursor.into_inner()).unwrap();

    assert_eq!(document.scene_objects.len(), 1);
    assert_eq!(document.scene_feature_objects.len(), 1);
    let feature_object = &document.scene_feature_objects[0];
    assert_eq!(feature_object.object_name, "Plane Feature");
    assert_eq!(feature_object.object_key, "1_Plane_Feature");
    assert_eq!(feature_object.parent_key, "0_Root");
    assert_eq!(
        feature_object.hierarchy_path,
        vec!["0_Root", "1_Plane_Feature"]
    );
    assert_eq!(feature_object.feature_type, "PlaneObject");
    assert_eq!(feature_object.xf.row_x, [2.0, 0.0, 0.0]);
    assert_eq!(feature_object.xf.row_y, [0.0, 3.0, 0.0]);
    assert_eq!(feature_object.xf.row_z, [0.0, 0.0, 1.5]);
    assert_eq!(feature_object.xf.b, [1.0, 2.0, 3.0]);
    assert_eq!(feature_object.subfeature_visibility, 15);
    assert_eq!(feature_object.details_on_name_tag, 7);
    assert_eq!(
        feature_object.decorations_color_unselected,
        [0.1, 0.2, 0.3, 0.4]
    );
    assert_eq!(
        feature_object.decorations_color_selected,
        [0.5, 0.6, 0.7, 0.8]
    );
    assert!((feature_object.point_size - 11.0).abs() < f32::EPSILON);
    assert!((feature_object.line_width - 2.5).abs() < f32::EPSILON);
    assert!((feature_object.sub_point_size - 6.5).abs() < f32::EPSILON);
    assert!((feature_object.sub_line_width - 1.5).abs() < f32::EPSILON);
    assert!((feature_object.main_alpha - 0.9).abs() < f32::EPSILON);
    assert!((feature_object.sub_alpha_points - 0.8).abs() < f32::EPSILON);
    assert!((feature_object.sub_alpha_lines - 0.7).abs() < f32::EPSILON);
    assert!((feature_object.sub_alpha_mesh - 0.6).abs() < f32::EPSILON);
    assert_eq!(feature_object.dimension_visibility.get("Length"), Some(&3));
    assert_eq!(feature_object.dimension_visibility.get("Angle"), Some(&5));
    assert_eq!(feature_object.visibility_mask, 0);
    assert!(feature_object.selected);
    assert!(feature_object.locked);
    assert!(!feature_object.parent_locked);

    let mesh_object = &document.scene_objects[0];
    let exported = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: document.root_key.clone(),
        vertices: document.vertices.clone(),
        faces: document.faces.clone(),
        objects: vec![MeshlibSceneExportObject {
            object_name: mesh_object.object_name.clone(),
            object_key: mesh_object.object_key.clone(),
            parent_key: mesh_object.parent_key.clone(),
            hierarchy_path: mesh_object.hierarchy_path.clone(),
            model_file: mesh_object.model_file.clone(),
            model_extension: mesh_object.model_extension.clone(),
            link: mesh_object.link.clone(),
            shared_model_source_index: mesh_object.shared_model_source_index,
            vertex_range: mesh_object.vertex_range,
            face_range: mesh_object.face_range,
            xf: mesh_object.xf,
            visibility_mask: mesh_object.visibility_mask,
            selected: mesh_object.selected,
            locked: mesh_object.locked,
            parent_locked: mesh_object.parent_locked,
        }],
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: document.scene_feature_objects.clone(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(exported.clone())).unwrap();
    assert!(archive.by_name("0_Root/0_Base_A.ply").is_ok());
    assert!(archive.by_name("0_Root/1_Plane_Feature.ply").is_err());
    let exported_root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let exported_feature = &exported_root["Children"]["1"];
    assert_eq!(exported_feature["Name"], "Plane Feature");
    assert_eq!(
        exported_feature["Type"],
        serde_json::json!(["Object", "VisualObject", "FeatureObject", "PlaneObject"])
    );
    assert_eq!(exported_feature["SubfeatureVisibility"], 15);
    assert_eq!(exported_feature["DetailsOnNameTag"], 7);
    assert_eq!(exported_feature["DecorationsColorSelected"]["w"], 0.8);
    assert_eq!(exported_feature["PointSize"], 11.0);
    assert!((exported_feature["SubAlphaMesh"].as_f64().unwrap() - 0.6).abs() < 1.0e-6);
    assert_eq!(exported_feature["DimensionVisibility"]["Length"], 3);

    let reloaded = meshlib_object_mesh_document_from_mru_scene_bytes(&exported).unwrap();
    assert_eq!(reloaded.scene_feature_objects.len(), 1);
    assert_eq!(
        reloaded.scene_feature_objects[0].feature_type,
        "PlaneObject"
    );
    assert_eq!(reloaded.scene_feature_objects[0].xf.b, [1.0, 2.0, 3.0]);
}

#[test]
fn meshlib_reparent_scene_object_updates_hierarchy_paths_like_add_child() {
    let input = MeshlibSceneReparentInput {
        root_key: "0_Root".to_string(),
        objects: vec![
            MeshlibSceneExportObject {
                object_name: "Base A".to_string(),
                object_key: "0_Base_A".to_string(),
                parent_key: "0_Root".to_string(),
                hierarchy_path: vec!["0_Root".to_string(), "0_Base_A".to_string()],
                model_file: "0_Root/0_Base_A.ply".to_string(),
                model_extension: ".ply".to_string(),
                link: None,
                shared_model_source_index: None,
                vertex_range: [0, 3],
                face_range: [0, 1],
                xf: MeshlibSceneXf {
                    row_x: [1.0, 0.0, 0.0],
                    row_y: [0.0, 1.0, 0.0],
                    row_z: [0.0, 0.0, 1.0],
                    b: [0.0, 0.0, 0.0],
                },
                visibility_mask: VIEWPORT_MASK_ALL,
                selected: false,
                locked: false,
                parent_locked: false,
            },
            MeshlibSceneExportObject {
                object_name: "Child B".to_string(),
                object_key: "1_Child_B".to_string(),
                parent_key: "0_Root".to_string(),
                hierarchy_path: vec!["0_Root".to_string(), "1_Child_B".to_string()],
                model_file: "0_Root/1_Child_B.ply".to_string(),
                model_extension: ".ply".to_string(),
                link: None,
                shared_model_source_index: None,
                vertex_range: [3, 6],
                face_range: [1, 2],
                xf: MeshlibSceneXf {
                    row_x: [1.0, 0.0, 0.0],
                    row_y: [0.0, 1.0, 0.0],
                    row_z: [0.0, 0.0, 1.0],
                    b: [0.0, 0.0, 0.0],
                },
                visibility_mask: VIEWPORT_MASK_ALL,
                selected: false,
                locked: false,
                parent_locked: false,
            },
        ],
        object_key: "1_Child_B".to_string(),
        new_parent_key: "0_Base_A".to_string(),
    };

    let result = meshlib_reparent_scene_object(&input).unwrap();

    assert_eq!(result.objects[1].parent_key, "0_Base_A");
    assert_eq!(
        result.objects[1].hierarchy_path,
        vec!["0_Root", "0_Base_A", "1_Child_B"]
    );
    assert_eq!(
        result.objects[1].model_file,
        "0_Root/0_Base_A/1_Child_B.ply"
    );
}

#[test]
fn meshlib_set_scene_object_state_serializes_visibility_and_lock_flags() {
    let input = MeshlibSceneObjectStateInput {
        objects: vec![MeshlibSceneExportObject {
            object_name: "Base A".to_string(),
            object_key: "0_Base_A".to_string(),
            parent_key: "0_Root".to_string(),
            hierarchy_path: vec!["0_Root".to_string(), "0_Base_A".to_string()],
            model_file: "0_Root/0_Base_A.ply".to_string(),
            model_extension: ".ply".to_string(),
            link: None,
            shared_model_source_index: None,
            vertex_range: [0, 3],
            face_range: [0, 1],
            xf: MeshlibSceneXf {
                row_x: [1.0, 0.0, 0.0],
                row_y: [0.0, 1.0, 0.0],
                row_z: [0.0, 0.0, 1.0],
                b: [0.0, 0.0, 0.0],
            },
            visibility_mask: VIEWPORT_MASK_ALL,
            selected: false,
            locked: false,
            parent_locked: false,
        }],
        feature_objects: Vec::new(),
        object_key: "0_Base_A".to_string(),
        visibility_mask: Some(0),
        selected: Some(true),
        locked: Some(true),
        parent_locked: Some(true),
    };

    let result = meshlib_set_scene_object_state(&input).unwrap();

    assert_eq!(result.objects[0].visibility_mask, 0);
    assert!(result.objects[0].selected);
    assert!(result.objects[0].locked);
    assert!(result.objects[0].parent_locked);

    let archive = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: "0_Root".to_string(),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![[0, 1, 2]],
        objects: result.objects,
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
    let root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    let object = &root["Children"]["0"];
    assert_eq!(object["Visibility"], 0);
    assert_eq!(object["Selected"], true);
    assert_eq!(object["Locked"], true);
    assert_eq!(object["ParentLocked"], true);
}

#[test]
fn meshlib_reorder_scene_children_matches_change_scene_objects_order() {
    let base_object =
        |name: &str, key: &str, vertex_start: usize, face_start: usize| MeshlibSceneExportObject {
            object_name: name.to_string(),
            object_key: key.to_string(),
            parent_key: "0_Root".to_string(),
            hierarchy_path: vec!["0_Root".to_string(), key.to_string()],
            model_file: format!("0_Root/{key}.ply"),
            model_extension: ".ply".to_string(),
            link: None,
            shared_model_source_index: None,
            vertex_range: [vertex_start, vertex_start + 3],
            face_range: [face_start, face_start + 1],
            xf: MeshlibSceneXf {
                row_x: [1.0, 0.0, 0.0],
                row_y: [0.0, 1.0, 0.0],
                row_z: [0.0, 0.0, 1.0],
                b: [0.0, 0.0, 0.0],
            },
            visibility_mask: VIEWPORT_MASK_ALL,
            selected: false,
            locked: false,
            parent_locked: false,
        };
    let input = MeshlibSceneReorderInput {
        root_key: "0_Root".to_string(),
        parent_key: "0_Root".to_string(),
        objects: vec![
            base_object("Base A", "0_Base_A", 0, 0),
            base_object("Child B", "1_Child_B", 3, 1),
            base_object("Child C", "2_Child_C", 6, 2),
        ],
        ordered_child_keys: vec![
            "2_Child_C".to_string(),
            "0_Base_A".to_string(),
            "1_Child_B".to_string(),
        ],
    };

    let result = meshlib_reorder_scene_children(&input).unwrap();

    assert_eq!(
        result
            .objects
            .iter()
            .map(|object| object.object_key.as_str())
            .collect::<Vec<_>>(),
        vec!["2_Child_C", "0_Base_A", "1_Child_B"]
    );

    let archive = meshlib_multi_object_mru_scene_bytes(&MeshlibSceneExportInput {
        root_name: "Root".to_string(),
        root_key: "0_Root".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [4.0, 0.0, 0.0],
            [5.0, 0.0, 0.0],
            [4.0, 1.0, 0.0],
        ],
        faces: vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]],
        objects: result.objects,
        group_objects: Vec::new(),
        line_objects: Vec::new(),
        point_objects: Vec::new(),
        distance_map_objects: Vec::new(),
        voxel_objects: Vec::new(),
        feature_objects: Vec::new(),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
    let root: serde_json::Value =
        serde_json::from_reader(archive.by_name("Root.json").unwrap()).unwrap();
    assert_eq!(root["Children"]["0"]["Name"], "Child C");
    assert_eq!(root["Children"]["1"]["Name"], "Base A");
    assert_eq!(root["Children"]["2"]["Name"], "Child B");
}

#[test]
fn mesh_geodesic_path_matches_meshlib_edge_shortest_path_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_geodesic_path(&vertices, &faces, 0, 3, f64::INFINITY).unwrap();

    assert_eq!(path.vertex_indices.first(), Some(&0));
    assert_eq!(path.vertex_indices.last(), Some(&3));
    assert_eq!(path.vertex_indices.len(), 3);
    assert_eq!(path.edge_lengths.len(), 2);
    assert!((path.length_mm - 2.0).abs() < 1e-9);
}

#[test]
fn mesh_geodesic_polyline_path_stitches_mesh_cut_control_vertices() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_geodesic_polyline_path(&vertices, &faces, &[0, 1, 3], f64::INFINITY).unwrap();

    assert_eq!(path.control_vertex_indices, vec![0, 1, 3]);
    assert_eq!(path.control_vertex_offsets, vec![0, 1, 2]);
    assert_eq!(path.leg_vertex_offsets, vec![0, 1]);
    assert_eq!(path.vertex_indices, vec![0, 1, 3]);
    assert_eq!(path.points, vec![vertices[0], vertices[1], vertices[3]]);
    assert_eq!(path.leg_lengths, vec![1.0, 1.0]);
    assert_eq!(path.edge_lengths, vec![1.0, 1.0]);
    assert!((path.length_mm - 2.0).abs() < 1e-9);
    assert_eq!(path.line_segments, 2);
    assert!(!path.closed_path);
}

#[test]
fn mesh_geodesic_polyline_path_closes_mesh_cut_control_path() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path =
        mesh_geodesic_polyline_path_with_close(&vertices, &faces, &[0, 1, 3], true, f64::INFINITY)
            .unwrap();

    assert_eq!(path.control_vertex_indices, vec![0, 1, 3, 0]);
    assert_eq!(path.control_vertex_offsets.first(), Some(&0));
    assert_eq!(
        path.control_vertex_offsets.last(),
        Some(&(path.vertex_indices.len() - 1))
    );
    assert_eq!(path.vertex_indices.first(), Some(&0));
    assert_eq!(path.vertex_indices.last(), Some(&0));
    assert_eq!(path.leg_lengths.len(), 3);
    assert!((path.length_mm - 4.0).abs() < 1e-9);
    assert_eq!(path.line_segments, 4);
    assert!(path.closed_path);
}

#[test]
fn mesh_cut_measure_contours_builds_meshlib_onemesh_contour_payload() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let payload =
        mesh_cut_measure_contours(&vertices, &faces, &[0, 1, 3], true, f64::INFINITY).unwrap();

    assert!(payload.closed_path);
    assert_eq!(payload.path.vertex_indices, vec![0, 1, 3, 2, 0]);
    assert_eq!(payload.pivot_indices, vec![0, 1, 2, 4]);
    assert_eq!(payload.contours.len(), 1);
    assert!(payload.contours[0].closed);
    assert_eq!(payload.contours[0].intersections.len(), 5);
    assert_eq!(
        payload.contours[0].intersections[0].primitive_type,
        "VertId"
    );
    assert_eq!(payload.contours[0].intersections[0].primitive_id, 0);
    assert_eq!(payload.contours[0].intersections[0].coordinate, vertices[0]);
    assert_eq!(payload.result_cut_vertex_indices, vec![vec![0, 1, 3, 2, 0]]);
    assert!(payload.bad_face_indices.is_empty());
    assert_eq!(
        payload.meshlib_reference,
        "MR::convertSurfacePathsToMeshContours / MR::cutMesh"
    );
}

#[test]
fn mesh_cut_measure_edge_path_topology_cut_splits_shared_edge_seam() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let cut =
        mesh_cut_measure_edge_path_topology_cut(&vertices, &faces, &[1, 2], false, f64::INFINITY)
            .unwrap();

    assert_eq!(cut.vertices.len(), 6);
    assert_eq!(cut.faces, vec![[0_i64, 1, 2], [5, 4, 3]]);
    assert_eq!(cut.source_path_vertex_indices, vec![1, 2]);
    assert_eq!(cut.result_cut_vertex_indices, vec![vec![4, 5]]);
    assert_eq!(cut.duplicate_vertex_map, vec![[1, 4], [2, 5]]);
    assert_eq!(cut.cut_edge_pairs, vec![[1, 2]]);
    assert_eq!(cut.result_cut_edge_pairs, vec![[4, 5]]);
    assert!(cut.bad_face_indices.is_empty());
    assert!((cut.length_mm - 2.0_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        cut.meshlib_reference,
        "MR::convertSurfacePathsToMeshContours / MR::cutMesh edge-path seam subset"
    );
}

#[test]
fn mesh_geodesic_path_rejects_disconnected_vertices() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [4.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [3, 4, 5]];

    assert!(mesh_geodesic_path(&vertices, &faces, 0, 5, f64::INFINITY).is_err());
}

#[test]
fn mesh_geodesic_distance_field_matches_meshlib_surface_distance_seed_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let field = mesh_geodesic_distance_field(&vertices, &faces, &[0], f64::INFINITY).unwrap();

    assert_eq!(field.seed_vertices, vec![0]);
    assert_eq!(field.reachable_vertex_count, 4);
    assert_eq!(field.predecessor_vertices[0], None);
    assert!((field.distances_mm[0] - 0.0).abs() < 1e-9);
    assert!((field.distances_mm[1] - 1.0).abs() < 1e-9);
    assert!((field.distances_mm[2] - 1.0).abs() < 1e-9);
    assert!((field.distances_mm[3] - 1.7071067811865475).abs() < 1e-9);
    assert!((field.max_distance_mm - 1.7071067811865475).abs() < 1e-9);
}

#[test]
fn mesh_fast_marching_surface_path_matches_meshlib_vertex_endpoint_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_fast_marching_surface_path(&vertices, &faces, 0, 3, 16).unwrap();

    assert_eq!(path.start_vertex, 0);
    assert_eq!(path.end_vertex, 3);
    assert_eq!(path.start_face_index, 0);
    assert_eq!(path.start_barycentric, [1.0, 0.0, 0.0]);
    assert!((path.surface_distances_mm[0] - 1.7071067811865475).abs() < 1e-9);
    assert!((path.surface_distances_mm[1] - 1.0).abs() < 1e-9);
    assert!((path.surface_distances_mm[2] - 1.0).abs() < 1e-9);
    assert_eq!(path.surface_distances_mm[3], 0.0);
    assert_eq!(path.edges, vec![[1, 2], [3, 2]]);
    assert!((path.positions[0] - 0.5).abs() < 1e-9);
    assert!(path.positions[1].abs() < 1e-9);
    assert_eq!(path.reached_vertex, Some(3));
    assert_eq!(path.stopped_reason, "end_reached");
    assert_eq!(path.steps, 2);
    assert!((path.length_mm - std::f64::consts::SQRT_2).abs() < 1e-9);
    assert_eq!(path.meshlib_reference, "MR::computeFastMarchingPath");
}

#[test]
fn mesh_fast_marching_surface_path_tri_points_stops_in_end_triangle_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.5, 0.25, 0.25],
        1,
        [0.25, 0.25, 0.5],
        16,
    )
    .unwrap();

    assert_eq!(path.start_face_index, 0);
    assert_eq!(path.end_face_index, 1);
    assert_eq!(path.start_point, [0.25, 0.25, 0.0]);
    assert_eq!(path.end_point, [0.75, 0.75, 0.0]);
    assert_eq!(path.edges, vec![[1, 2]]);
    assert_eq!(path.positions.len(), 1);
    assert!((path.positions[0] - 0.5).abs() < 1e-9);
    assert_eq!(path.points, vec![[0.5, 0.5, 0.0]]);
    assert_eq!(path.reached_face_index, Some(1));
    assert_eq!(path.stopped_reason, "end_triangle_reached");
    assert_eq!(path.steps, 1);
    assert!((path.length_mm - 0.5_f64.sqrt()).abs() < 1e-9);
    assert_eq!(path.meshlib_reference, "MR::computeFastMarchingPath");
}

#[test]
fn voxel_default_iso_value_matches_meshlib_object_voxels_histogram_one_third_bin_contract() {
    let values = [-10.0_f32, 0.0, 10.0, 20.0];

    let iso_value = voxel_default_iso_value(&values).unwrap();

    let expected = -10.0 + 85.0 * ((20.0 - -10.0) / 256.0);
    assert!((iso_value - expected).abs() < 1e-6);
}

#[test]
fn voxel_default_iso_value_matches_meshlib_zero_width_histogram_contract() {
    let values = [42.0_f32, 42.0, 42.0];

    let iso_value = voxel_default_iso_value(&values).unwrap();

    assert_eq!(iso_value, 42.0);
}

#[test]
fn voxel_default_iso_value_preserves_meshlib_raw_infinite_minmax_contract() {
    let iso_value = voxel_default_iso_value_from_min_max(f32::MAX, f32::INFINITY).unwrap();

    assert!(iso_value.is_infinite());
    assert!(iso_value.is_sign_positive());
}

#[test]
fn voxel_path_difference_metric_matches_meshlib_sum_diffs_detour_contract() {
    let values = vec![0.0_f32, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];

    let path = voxel_path_values(
        &values,
        [3, 3, 1],
        [0, 1, 0],
        [2, 1, 0],
        VoxelPathMetric::Difference,
        VoxelPathOptions::default(),
    )
    .unwrap();

    assert_eq!(path.coordinates.first(), Some(&[0, 1, 0]));
    assert_eq!(path.coordinates.last(), Some(&[2, 1, 0]));
    assert_eq!(path.voxel_indices.len(), 5);
    assert!(!path.coordinates.contains(&[1, 1, 0]));
    assert_eq!(path.total_metric, 0.0);
}

#[test]
fn voxel_path_exponent_metric_matches_meshlib_high_density_preference_contract() {
    let values = vec![0.0_f32, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];

    let path = voxel_path_values(
        &values,
        [3, 3, 1],
        [0, 1, 0],
        [2, 1, 0],
        VoxelPathMetric::Exponent,
        VoxelPathOptions::default(),
    )
    .unwrap();

    assert_eq!(path.coordinates, vec![[0, 1, 0], [1, 1, 0], [2, 1, 0]]);
    assert!(path.total_metric < 1.001);
}

#[test]
fn voxel_path_build_four_matches_meshlib_quarter_seed_contract() {
    let values = vec![0.0_f32; 5 * 5 * 5];

    let result = voxel_path_build_four_values(
        &values,
        [5, 5, 5],
        [0, 2, 2],
        [4, 2, 2],
        VoxelPathMetric::Difference,
        VoxelPathOptions::default(),
    )
    .unwrap();

    assert_eq!(
        result
            .paths
            .iter()
            .map(|entry| entry.quarters_mask)
            .collect::<Vec<_>>(),
        vec![
            VoxelPathOptions::QUARTER_LEFT_LEFT,
            VoxelPathOptions::QUARTER_LEFT_RIGHT,
            VoxelPathOptions::QUARTER_RIGHT_LEFT,
            VoxelPathOptions::QUARTER_RIGHT_RIGHT,
        ]
    );
    assert_eq!(result.paths.len(), 4);
    for entry in &result.paths {
        assert_eq!(entry.path.coordinates.first(), Some(&[0, 2, 2]));
        assert_eq!(entry.path.coordinates.last(), Some(&[4, 2, 2]));
        assert_eq!(entry.path.total_metric, 0.0);
    }
    assert!(result.paths[0].path.coordinates.contains(&[2, 0, 0]));
    assert!(result.paths[1].path.coordinates.contains(&[2, 0, 2]));
    assert!(result.paths[2].path.coordinates.contains(&[2, 2, 0]));
    assert!(result.paths[3].path.coordinates.contains(&[2, 2, 2]));
}

#[test]
fn voxel_slice_values_match_meshlib_save_slice_texture_order_contract() {
    let shape = [2, 3, 4];
    let mut values = Vec::new();
    for z in 0..shape[2] {
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                values.push((x + 10 * y + 100 * z) as f32);
            }
        }
    }

    let xy = voxel_slice_values(&values, shape, VoxelPathPlane::XY, 2, 200.0, 221.0).unwrap();
    assert_eq!(xy.width, 2);
    assert_eq!(xy.height, 3);
    assert_eq!(xy.values, vec![200.0, 201.0, 210.0, 211.0, 220.0, 221.0]);
    assert_eq!(xy.coordinates[0], [0, 0, 2]);
    assert_eq!(xy.coordinates[5], [1, 2, 2]);
    assert_eq!(xy.normalized_values[0], 0.0);
    assert_eq!(xy.normalized_values[5], 1.0);

    let yz = voxel_slice_values(&values, shape, VoxelPathPlane::YZ, 1, 1.0, 321.0).unwrap();
    assert_eq!(yz.width, 3);
    assert_eq!(yz.height, 4);
    assert_eq!(
        yz.values,
        vec![1.0, 11.0, 21.0, 101.0, 111.0, 121.0, 201.0, 211.0, 221.0, 301.0, 311.0, 321.0]
    );
    assert_eq!(yz.coordinates[0], [1, 0, 0]);
    assert_eq!(yz.coordinates[11], [1, 2, 3]);

    let zx = voxel_slice_values(&values, shape, VoxelPathPlane::ZX, 2, 20.0, 321.0).unwrap();
    assert_eq!(zx.width, 4);
    assert_eq!(zx.height, 2);
    assert_eq!(
        zx.values,
        vec![20.0, 120.0, 220.0, 320.0, 21.0, 121.0, 221.0, 321.0]
    );
    assert_eq!(zx.coordinates[0], [0, 2, 0]);
    assert_eq!(zx.coordinates[7], [1, 2, 3]);
}

#[test]
fn voxel_line_graph_values_match_meshlib_axis_probe_contract() {
    let shape = [3, 2, 2];
    let mut values = Vec::new();
    for z in 0..shape[2] {
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                values.push((x + 10 * y + 100 * z) as f32);
            }
        }
    }

    let x_line = voxel_line_graph_values(&values, shape, 0, [0, 1, 1]).unwrap();
    assert_eq!(x_line.axis, 0);
    assert_eq!(x_line.positions, vec![0, 1, 2]);
    assert_eq!(x_line.coordinates, vec![[0, 1, 1], [1, 1, 1], [2, 1, 1]]);
    assert_eq!(x_line.voxel_indices, vec![9, 10, 11]);
    assert_eq!(x_line.values, vec![110.0, 111.0, 112.0]);

    let y_line = voxel_line_graph_values(&values, shape, 1, [2, 0, 1]).unwrap();
    assert_eq!(y_line.positions, vec![0, 1]);
    assert_eq!(y_line.coordinates, vec![[2, 0, 1], [2, 1, 1]]);
    assert_eq!(y_line.voxel_indices, vec![8, 11]);
    assert_eq!(y_line.values, vec![102.0, 112.0]);

    let z_line = voxel_line_graph_values(&values, shape, 2, [2, 1, 0]).unwrap();
    assert_eq!(z_line.positions, vec![0, 1]);
    assert_eq!(z_line.coordinates, vec![[2, 1, 0], [2, 1, 1]]);
    assert_eq!(z_line.voxel_indices, vec![5, 11]);
    assert_eq!(z_line.values, vec![12.0, 112.0]);
}

#[test]
fn voxel_active_box_values_match_meshlib_max_excluded_bounds_contract() {
    let shape = [4, 3, 2];
    let mut values = Vec::new();
    for z in 0..shape[2] {
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                values.push((x + 10 * y + 100 * z) as f32);
            }
        }
    }

    let active_box = voxel_active_box_values(&values, shape, [1, 1, 0], [2, 2, 2]).unwrap();

    assert_eq!(active_box.min_corner, [1, 1, 0]);
    assert_eq!(active_box.dimensions, [2, 2, 2]);
    assert_eq!(
        active_box.coordinates,
        vec![
            [1, 1, 0],
            [2, 1, 0],
            [1, 2, 0],
            [2, 2, 0],
            [1, 1, 1],
            [2, 1, 1],
            [1, 2, 1],
            [2, 2, 1],
        ]
    );
    assert_eq!(active_box.source_indices, vec![5, 6, 9, 10, 17, 18, 21, 22]);
    assert_eq!(
        active_box.values,
        vec![11.0, 12.0, 21.0, 22.0, 111.0, 112.0, 121.0, 122.0]
    );
}

#[test]
fn voxel_segmentation_values_match_meshlib_graph_cut_high_density_contract() {
    let values = vec![0.0_f32, 0.0, 0.0, 10.0, 10.0];

    let segmentation = voxel_segmentation_values(
        &values,
        [5, 1, 1],
        &[[4, 0, 0]],
        &[[0, 0, 0]],
        VoxelSegmentationOptions {
            exponent_modifier: 2.0,
            voxels_expansion: 4,
            include_boundary_outside: false,
        },
    )
    .unwrap();

    assert_eq!(segmentation.min_corner, [0, 0, 0]);
    assert_eq!(segmentation.dimensions, [5, 1, 1]);
    assert_eq!(
        segmentation.selected_coordinates,
        vec![[3, 0, 0], [4, 0, 0]]
    );
    assert_eq!(segmentation.source_indices, vec![3, 4]);
    assert_eq!(segmentation.part_indices, vec![3, 4]);
    assert_eq!(segmentation.selected_values, vec![10.0, 10.0]);
}

#[test]
fn voxel_segmentation_values_match_meshlib_volume_part_boundary_seed_contract() {
    let shape = [5, 3, 3];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for coord in [[3, 1, 1], [4, 1, 1]] {
        values[coord[0] + coord[1] * shape[0] + coord[2] * shape[0] * shape[1]] = 10.0;
    }

    let segmentation = voxel_segmentation_values(
        &values,
        shape,
        &[[4, 1, 1]],
        &[],
        VoxelSegmentationOptions {
            exponent_modifier: 2.0,
            voxels_expansion: 4,
            include_boundary_outside: true,
        },
    )
    .unwrap();

    assert_eq!(segmentation.min_corner, [0, 0, 0]);
    assert_eq!(segmentation.dimensions, [5, 3, 3]);
    assert_eq!(
        segmentation.selected_coordinates,
        vec![[3, 1, 1], [4, 1, 1]]
    );
    assert_eq!(segmentation.source_indices, vec![23, 24]);
    assert_eq!(segmentation.part_indices, vec![23, 24]);
}

#[test]
fn voxel_segmentation_mesh_values_match_meshlib_simple_mask_iso_shift_contract() {
    let shape = [5, 5, 5];
    let mut values = vec![0.0_f32; shape.iter().product()];
    values[2 + 2 * shape[0] + 2 * shape[0] * shape[1]] = 10.0;

    let mesh = voxel_segmentation_mesh_values(
        &values,
        shape,
        &[[2, 2, 2]],
        &[],
        VoxelSegmentationOptions {
            exponent_modifier: 2.0,
            voxels_expansion: 1,
            include_boundary_outside: true,
        },
        [0.5, 1.0, 2.0],
    )
    .unwrap();

    assert_eq!(mesh.segmentation.min_corner, [1, 1, 1]);
    assert_eq!(mesh.segmentation.dimensions, [3, 3, 3]);
    assert_eq!(mesh.segmentation.selected_coordinates, vec![[2, 2, 2]]);
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.faces.is_empty());
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(min, [0.75, 1.5, 3.0]);
    assert_eq!(max, [1.25, 2.5, 5.0]);
}

#[test]
fn voxel_mask_to_mesh_values_match_meshlib_smooth_mask_meshing_contract() {
    let shape = [5, 5, 5];
    let mut values = vec![0.0_f32; shape.iter().product()];
    values[2 + 2 * shape[0] + 2 * shape[0] * shape[1]] = 10.0;

    let mesh =
        voxel_mask_to_mesh_values(&values, shape, &[[2, 2, 2]], [0.5, 1.0, 2.0], 1, 3).unwrap();

    assert_eq!(mesh.min_corner, [1, 1, 1]);
    assert_eq!(mesh.dimensions, [3, 3, 3]);
    assert_eq!(mesh.selected_coordinates, vec![[2, 2, 2]]);
    assert_eq!(mesh.source_indices, vec![62]);
    assert_eq!(mesh.part_indices, vec![13]);
    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.faces.is_empty());
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(min, [0.75, 1.5, 3.0]);
    assert_eq!(max, [1.25, 2.5, 5.0]);
}

#[test]
fn voxel_to_mesh_simple_values_match_meshlib_dense_volume_iso_contract() {
    let shape = [5, 5, 5];
    let mut values = vec![0.0_f32; shape.iter().product()];
    values[2 + 2 * shape[0] + 2 * shape[0] * shape[1]] = 10.0;

    let mesh = voxel_to_mesh_simple_values(&values, shape, [0.5, 1.0, 2.0], 5.0, false).unwrap();

    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.faces.is_empty());
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(min, [0.75, 1.5, 3.0]);
    assert_eq!(max, [1.25, 2.5, 5.0]);
}

#[test]
fn voxel_to_mesh_simple_values_match_meshlib_level_set_less_inside_contract() {
    let shape = [5, 5, 5];
    let mut values = vec![10.0_f32; shape.iter().product()];
    values[2 + 2 * shape[0] + 2 * shape[0] * shape[1]] = -10.0;

    let mesh = voxel_to_mesh_simple_values(&values, shape, [1.0, 1.0, 1.0], 0.0, true).unwrap();

    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.faces.is_empty());
    assert!((mesh_signed_volume(&mesh.vertices, &mesh.faces).unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn voxel_to_mesh_dual_values_extracts_meshlib_dense_dual_plane_slice() {
    let shape = [4, 4, 4];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                values[x + y * shape[0] + z * shape[0] * shape[1]] = x as f32;
            }
        }
    }

    let mesh = voxel_to_mesh_dual_values(&values, shape, [0.5, 1.0, 2.0], 1.5, true).unwrap();

    assert_eq!(mesh.vertices.len(), 9);
    assert_eq!(mesh.faces.len(), 8);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(min, [0.75, 0.5, 1.0]);
    assert_eq!(max, [0.75, 2.5, 5.0]);
}

#[test]
fn voxel_to_mesh_dual_values_with_settings_enforces_meshlib_limits() {
    let shape = [4, 4, 4];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                values[x + y * shape[0] + z * shape[0] * shape[1]] = x as f32;
            }
        }
    }
    let mut settings = VoxelDualMeshSettings {
        iso_value: 1.5,
        level_set: true,
        ..VoxelDualMeshSettings::default()
    };

    settings.max_vertices = 8;
    let vertex_error =
        voxel_to_mesh_dual_values_with_settings(&values, shape, [0.5, 1.0, 2.0], settings)
            .unwrap_err();
    assert_eq!(vertex_error.to_string(), "Vertices number limit exceeded.");

    settings.max_vertices = usize::MAX;
    settings.max_faces = 7;
    let face_error =
        voxel_to_mesh_dual_values_with_settings(&values, shape, [0.5, 1.0, 2.0], settings)
            .unwrap_err();
    assert_eq!(face_error.to_string(), "Triangles number limit exceeded.");

    settings.max_faces = 8;
    let mesh =
        voxel_to_mesh_dual_values_with_settings(&values, shape, [0.5, 1.0, 2.0], settings).unwrap();
    assert_eq!(mesh.vertices.len(), 9);
    assert_eq!(mesh.faces.len(), 8);
}

#[test]
fn voxel_to_mesh_dual_values_with_settings_applies_meshlib_planar_adaptivity() {
    let shape = [4, 4, 4];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                values[x + y * shape[0] + z * shape[0] * shape[1]] = x as f32;
            }
        }
    }

    let mesh = voxel_to_mesh_dual_values_with_settings(
        &values,
        shape,
        [0.5, 1.0, 2.0],
        VoxelDualMeshSettings {
            iso_value: 1.5,
            level_set: true,
            adaptivity: 1.0,
            ..VoxelDualMeshSettings::default()
        },
    )
    .unwrap();

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.faces.len(), 2);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(min, [0.75, 0.5, 1.0]);
    assert_eq!(max, [0.75, 2.5, 5.0]);
}

#[test]
fn voxel_to_mesh_dual_values_with_settings_reuses_relaxation_acceleration() {
    let shape = [16, 16, 16];
    let xy = shape[0] * shape[1];
    let center = [7.5_f64, 7.5_f64, 7.5_f64];
    let radius = 5.5_f64;
    let mut values = vec![0.0_f32; shape.iter().product()];
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                let dx = x as f64 - center[0];
                let dy = y as f64 - center[1];
                let dz = z as f64 - center[2];
                values[x + y * shape[0] + z * xy] =
                    ((dx * dx + dy * dy + dz * dz).sqrt() - radius) as f32;
            }
        }
    }

    let started = Instant::now();
    let mesh = voxel_to_mesh_dual_values_with_settings(
        &values,
        shape,
        [1.0, 1.0, 1.0],
        VoxelDualMeshSettings {
            iso_value: 0.0,
            level_set: true,
            relax_disoriented_triangles: true,
            ..VoxelDualMeshSettings::default()
        },
    )
    .unwrap();
    let elapsed = started.elapsed();

    assert!(mesh.vertices.len() > 100);
    assert!(mesh.faces.len() > 100);
    assert!(
        elapsed < Duration::from_secs(4),
        "dual meshing relaxation should reuse its ray acceleration structure, took {elapsed:?}"
    );
}

#[test]
fn relax_disoriented_mesh_triangles_flips_meshlib_ray_invalid_faces() {
    let vertices = vec![
        [1.0, 1.0, 1.0],
        [-1.0, -1.0, 1.0],
        [-1.0, 1.0, -1.0],
        [1.0, -1.0, -1.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    assert_eq!(
        find_disoriented_faces(
            &vertices,
            &faces,
            FindDisorientationRayMode::Shallowest,
            1e-8
        )
        .unwrap(),
        vec![0]
    );

    let relaxed = relax_disoriented_mesh_triangles(MeshArrays {
        vertices: vertices.clone(),
        faces: faces.clone(),
    })
    .unwrap();

    assert_eq!(relaxed.faces[0], [0, 2, 1]);
    assert_eq!(&relaxed.faces[1..], &faces[1..]);
    assert_eq!(
        find_disoriented_faces(
            &relaxed.vertices,
            &relaxed.faces,
            FindDisorientationRayMode::Shallowest,
            1e-8
        )
        .unwrap(),
        Vec::<usize>::new()
    );
}

#[test]
fn meshlib_vdb_payload_to_dual_mesh_extracts_decoded_openvdb_dense_leaf() {
    fn push_u8(bytes: &mut Vec<u8>, value: u8) {
        bytes.push(value);
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i64(bytes: &mut Vec<u8>, value: i64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_f64(bytes: &mut Vec<u8>, value: f64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_string(bytes: &mut Vec<u8>, name: &str, value: &str) {
        push_string(bytes, name);
        push_string(bytes, "string");
        push_u32(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    fn push_metadata_i64(bytes: &mut Vec<u8>, name: &str, value: i64) {
        push_string(bytes, name);
        push_string(bytes, "int64");
        push_u32(bytes, 8);
        push_i64(bytes, value);
    }

    fn push_metadata_vec3i(bytes: &mut Vec<u8>, name: &str, value: [i32; 3]) {
        push_string(bytes, name);
        push_string(bytes, "vec3i");
        push_u32(bytes, 12);
        for component in value {
            push_i32(bytes, component);
        }
    }

    fn push_dvec3(bytes: &mut Vec<u8>, value: [f64; 3]) {
        for component in value {
            push_f64(bytes, component);
        }
    }

    fn push_node_mask(bytes: &mut Vec<u8>, log2_dim: usize, enabled_offsets: &[usize]) {
        let bit_count = 1usize << (3 * log2_dim);
        let byte_count = bit_count / 8;
        let mut mask = vec![0_u8; byte_count];
        for offset in enabled_offsets {
            mask[*offset / 8] |= 1_u8 << (*offset % 8);
        }
        bytes.extend_from_slice(&mask);
    }

    fn push_uncompressed_float_values(bytes: &mut Vec<u8>, count: usize, value: f32) {
        push_u8(bytes, 6);
        for _ in 0..count {
            push_f32(bytes, value);
        }
    }

    fn push_active_mask_values_header(bytes: &mut Vec<u8>) {
        push_u8(bytes, 0);
    }

    fn synthetic_openvdb_single_dense_leaf(values: &[f32]) -> Vec<u8> {
        synthetic_openvdb_single_dense_leaf_at(values, [0, 0, 0], None, None)
    }

    fn synthetic_openvdb_single_dense_leaf_at(
        values: &[f32],
        leaf_origin: [i32; 3],
        topology_offsets: Option<&[usize]>,
        buffer_offsets: Option<&[usize]>,
    ) -> Vec<u8> {
        synthetic_openvdb_single_leaf_with_bbox(
            values,
            leaf_origin,
            leaf_origin,
            [leaf_origin[0] + 7, leaf_origin[1] + 7, leaf_origin[2] + 7],
            topology_offsets,
            buffer_offsets,
            1000.0,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn synthetic_openvdb_single_leaf_with_bbox(
        values: &[f32],
        leaf_origin: [i32; 3],
        file_bbox_min: [i32; 3],
        file_bbox_max: [i32; 3],
        topology_offsets: Option<&[usize]>,
        buffer_offsets: Option<&[usize]>,
        root_background: f32,
        active_mask_compression: bool,
    ) -> Vec<u8> {
        assert_eq!(values.len(), 512);
        assert!(leaf_origin
            .iter()
            .all(|origin| *origin >= 0 && *origin <= 120 && *origin % 8 == 0));
        let default_offsets = (0..512).collect::<Vec<_>>();
        let topology_offsets = topology_offsets.unwrap_or(&default_offsets);
        let buffer_offsets = buffer_offsets.unwrap_or(&default_offsets);

        let mut grid = Vec::new();
        push_u32(&mut grid, if active_mask_compression { 2 } else { 0 });
        push_u32(&mut grid, 5);
        push_metadata_vec3i(&mut grid, "file_bbox_min", file_bbox_min);
        push_metadata_vec3i(&mut grid, "file_bbox_max", file_bbox_max);
        push_metadata_i64(&mut grid, "file_voxel_count", topology_offsets.len() as i64);
        push_metadata_string(&mut grid, "value_type", "float");
        push_metadata_string(&mut grid, "class", "level set");
        push_string(&mut grid, "UniformScaleMap");
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [0.5, 0.5, 0.5]);
        push_dvec3(&mut grid, [2.0, 2.0, 2.0]);
        push_dvec3(&mut grid, [4.0, 4.0, 4.0]);
        push_dvec3(&mut grid, [1.0, 1.0, 1.0]);

        push_f32(&mut grid, root_background);
        push_u32(&mut grid, 0);
        push_u32(&mut grid, 1);
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);
        push_i32(&mut grid, 0);

        push_node_mask(&mut grid, 5, &[0]);
        push_node_mask(&mut grid, 5, &[]);
        if active_mask_compression {
            push_active_mask_values_header(&mut grid);
        } else {
            push_uncompressed_float_values(&mut grid, 1 << 15, root_background);
        }
        let second_level_leaf_offset = (leaf_origin[0] as usize / 8) * 256
            + (leaf_origin[1] as usize / 8) * 16
            + (leaf_origin[2] as usize / 8);
        push_node_mask(&mut grid, 4, &[second_level_leaf_offset]);
        push_node_mask(&mut grid, 4, &[]);
        if active_mask_compression {
            push_active_mask_values_header(&mut grid);
        } else {
            push_uncompressed_float_values(&mut grid, 1 << 12, root_background);
        }
        push_node_mask(&mut grid, 3, topology_offsets);

        push_node_mask(&mut grid, 3, buffer_offsets);
        if active_mask_compression {
            push_active_mask_values_header(&mut grid);
            for offset in buffer_offsets {
                let local = [*offset >> 6, (*offset & 63) >> 3, *offset & 7];
                let dense_index = local[0] + local[1] * 8 + local[2] * 64;
                push_f32(&mut grid, values[dense_index]);
            }
        } else {
            push_u8(&mut grid, 6);
            for x in 0..8 {
                for y in 0..8 {
                    for z in 0..8 {
                        let dense_index = x + y * 8 + z * 64;
                        push_f32(&mut grid, values[dense_index]);
                    }
                }
            }
        }

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0x20, 0x42, 0x44, 0x56, 0x00, 0x00, 0x00, 0x00]);
        push_u32(&mut bytes, 223);
        push_u32(&mut bytes, 12);
        push_u32(&mut bytes, 0);
        push_u8(&mut bytes, 1);
        bytes.extend_from_slice(b"00000000-0000-0000-0000-000000000000");
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, 1);
        push_string(&mut bytes, "dense_leaf");
        push_string(&mut bytes, "Tree_float_5_4_3");
        push_string(&mut bytes, "");
        let grid_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let block_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);
        let end_pos_offset = bytes.len();
        push_u64(&mut bytes, 0);

        let grid_pos = bytes.len() as u64;
        bytes.extend_from_slice(&grid);
        let end_pos = bytes.len() as u64;
        bytes[grid_pos_offset..grid_pos_offset + 8].copy_from_slice(&grid_pos.to_le_bytes());
        bytes[block_pos_offset..block_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes[end_pos_offset..end_pos_offset + 8].copy_from_slice(&end_pos.to_le_bytes());
        bytes
    }

    let values = (0..512)
        .map(|index| {
            let x = index % 8;
            x as f32
        })
        .collect::<Vec<_>>();
    let payload = synthetic_openvdb_single_dense_leaf(&values);

    let mesh = meshlib_vdb_payload_to_dual_mesh(&payload, [1, 1, 1], [9.0, 9.0, 9.0], 3.5).unwrap();

    assert_eq!(mesh.vertices.len(), 49);
    assert_eq!(mesh.faces.len(), 72);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(min, [1.75, 0.25, 0.25]);
    assert_eq!(max, [1.75, 3.25, 3.25]);

    let shifted_payload = synthetic_openvdb_single_dense_leaf_at(&values, [8, 16, 24], None, None);
    let shifted_mesh =
        meshlib_vdb_payload_to_dual_mesh(&shifted_payload, [1, 1, 1], [9.0, 9.0, 9.0], 3.5)
            .unwrap();
    assert_eq!(shifted_mesh.vertices.len(), 49);
    assert_eq!(shifted_mesh.faces.len(), 72);
    let mut shifted_min = [f64::INFINITY; 3];
    let mut shifted_max = [f64::NEG_INFINITY; 3];
    for vertex in &shifted_mesh.vertices {
        for axis in 0..3 {
            shifted_min[axis] = shifted_min[axis].min(vertex[axis]);
            shifted_max[axis] = shifted_max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(shifted_min, [5.75, 8.25, 12.25]);
    assert_eq!(shifted_max, [5.75, 11.25, 15.25]);

    let sparse_topology_payload = synthetic_openvdb_single_dense_leaf_at(
        &values,
        [0, 0, 0],
        Some(&[0usize, 83usize, 511usize]),
        None,
    );
    let sparse_topology_mesh =
        meshlib_vdb_payload_to_dual_mesh(&sparse_topology_payload, [1, 1, 1], [9.0, 9.0, 9.0], 3.5)
            .unwrap();
    assert_eq!(sparse_topology_mesh.vertices.len(), 49);
    assert_eq!(sparse_topology_mesh.faces.len(), 72);

    let mut single_active_values = vec![1.0_f32; 512];
    single_active_values[0] = -1.0;
    let tight_bbox_payload = synthetic_openvdb_single_leaf_with_bbox(
        &single_active_values,
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        Some(&[0]),
        Some(&[0]),
        1.0,
        true,
    );
    let tight_bbox_mesh =
        meshlib_vdb_payload_to_dual_mesh(&tight_bbox_payload, [1, 1, 1], [9.0, 9.0, 9.0], 0.0)
            .unwrap();
    assert_eq!(tight_bbox_mesh.vertices.len(), 8);
    assert_eq!(tight_bbox_mesh.faces.len(), 12);
    let mut tight_min = [f64::INFINITY; 3];
    let mut tight_max = [f64::NEG_INFINITY; 3];
    for vertex in &tight_bbox_mesh.vertices {
        for axis in 0..3 {
            tight_min[axis] = tight_min[axis].min(vertex[axis]);
            tight_max[axis] = tight_max[axis].max(vertex[axis]);
        }
    }
    for axis in 0..3 {
        assert!((tight_min[axis] - (-1.0 / 12.0)).abs() < 1e-12);
        assert!((tight_max[axis] - (1.0 / 12.0)).abs() < 1e-12);
    }

    let sparse_window_offsets = [0usize, 1, 8, 9, 64, 65, 72, 73];
    let mut sparse_window_values = vec![1.0_f32; 512];
    for offset in sparse_window_offsets {
        let local = [offset >> 6, (offset & 63) >> 3, offset & 7];
        sparse_window_values[local[0] + local[1] * 8 + local[2] * 64] = -1.0;
    }
    let sparse_window_payload = synthetic_openvdb_single_leaf_with_bbox(
        &sparse_window_values,
        [0, 0, 0],
        [0, 0, 0],
        [1, 1, 1],
        Some(&sparse_window_offsets),
        Some(&sparse_window_offsets),
        1.0,
        true,
    );
    let sparse_window_mesh =
        meshlib_vdb_payload_to_dual_mesh(&sparse_window_payload, [1, 1, 1], [9.0, 9.0, 9.0], 0.0)
            .unwrap();
    assert_eq!(sparse_window_mesh.vertices.len(), 26);
    assert_eq!(sparse_window_mesh.faces.len(), 48);
    let mut sparse_min = [f64::INFINITY; 3];
    let mut sparse_max = [f64::NEG_INFINITY; 3];
    for vertex in &sparse_window_mesh.vertices {
        for axis in 0..3 {
            sparse_min[axis] = sparse_min[axis].min(vertex[axis]);
            sparse_max[axis] = sparse_max[axis].max(vertex[axis]);
        }
    }
    assert_eq!(sparse_min, [-0.25, -0.25, -0.25]);
    assert_eq!(sparse_max, [0.75, 0.75, 0.75]);

    let full_leaf_sparse_offsets = [0usize, 511];
    let mut full_leaf_sparse_values = vec![1.0_f32; 512];
    full_leaf_sparse_values[0] = -1.0;
    full_leaf_sparse_values[511] = -1.0;
    let full_leaf_sparse_payload = synthetic_openvdb_single_leaf_with_bbox(
        &full_leaf_sparse_values,
        [0, 0, 0],
        [0, 0, 0],
        [7, 7, 7],
        Some(&full_leaf_sparse_offsets),
        Some(&full_leaf_sparse_offsets),
        1.0,
        true,
    );
    let full_leaf_sparse_mesh = meshlib_vdb_payload_to_dual_mesh(
        &full_leaf_sparse_payload,
        [1, 1, 1],
        [9.0, 9.0, 9.0],
        0.0,
    )
    .unwrap();
    assert_eq!(full_leaf_sparse_mesh.vertices.len(), 16);
    assert_eq!(full_leaf_sparse_mesh.faces.len(), 24);
    let mut full_leaf_min = [f64::INFINITY; 3];
    let mut full_leaf_max = [f64::NEG_INFINITY; 3];
    for vertex in &full_leaf_sparse_mesh.vertices {
        for axis in 0..3 {
            full_leaf_min[axis] = full_leaf_min[axis].min(vertex[axis]);
            full_leaf_max[axis] = full_leaf_max[axis].max(vertex[axis]);
        }
    }
    for axis in 0..3 {
        assert!((full_leaf_min[axis] - (-1.0 / 12.0)).abs() < 1e-12);
        assert!((full_leaf_max[axis] - (43.0 / 12.0)).abs() < 1e-12);
    }
}

#[test]
fn voxel_volume_render_data_values_match_meshlib_normalized_active_box_contract() {
    let shape = [4, 3, 2];
    let values = (0..shape.iter().product::<usize>())
        .map(|value| value as f32)
        .collect::<Vec<_>>();

    let render_data = voxel_volume_render_data_values(
        &values,
        shape,
        [0.5, 1.0, 2.0],
        [1, 1, 0],
        [2, 2, 2],
        0.0,
        23.0,
    )
    .unwrap();

    assert_eq!(render_data.dimensions, [2, 2, 2]);
    assert_eq!(render_data.voxel_size, [0.5, 1.0, 2.0]);
    assert_eq!(
        render_data.coordinates,
        vec![
            [1, 1, 0],
            [2, 1, 0],
            [1, 2, 0],
            [2, 2, 0],
            [1, 1, 1],
            [2, 1, 1],
            [1, 2, 1],
            [2, 2, 1],
        ]
    );
    assert_eq!(
        render_data.source_indices,
        vec![5, 6, 9, 10, 17, 18, 21, 22]
    );
    assert_eq!(
        render_data.values,
        vec![
            5.0 / 23.0,
            6.0 / 23.0,
            9.0 / 23.0,
            10.0 / 23.0,
            17.0 / 23.0,
            18.0 / 23.0,
            21.0 / 23.0,
            22.0 / 23.0,
        ]
    );
    assert_eq!(render_data.min_value, 0.0);
    assert_eq!(render_data.max_value, 1.0);
}

#[test]
fn voxel_volume_render_lut_values_match_meshlib_gray_one_color_and_rainbow_alpha_contract() {
    let gray =
        voxel_volume_render_lut_values("gray_shades", "linear_increasing", 10, None).unwrap();
    assert_eq!(gray.colors_rgba, vec![[255, 255, 255, 0], [0, 0, 0, 10]]);

    let one = voxel_volume_render_lut_values(
        "one_color",
        "linear_decreasing",
        10,
        Some([12, 34, 56, 200]),
    )
    .unwrap();
    assert_eq!(one.colors_rgba, vec![[12, 34, 56, 10], [12, 34, 56, 0]]);

    let rainbow = voxel_volume_render_lut_values("rainbow", "linear_increasing", 14, None).unwrap();
    assert_eq!(
        rainbow.colors_rgba,
        vec![
            [255, 0, 0, 0],
            [255, 127, 0, 2],
            [255, 255, 0, 4],
            [0, 255, 0, 6],
            [0, 0, 255, 8],
            [75, 0, 130, 10],
            [148, 0, 211, 12],
        ]
    );
    assert_eq!(
        rainbow.meshlib_reference,
        "RenderVolumeObject::bindVolume_ denseMap"
    );
}

#[test]
fn voxel_volume_render_ray_values_match_meshlib_step_sampling_and_front_to_back_compositing() {
    let shape = [3, 1, 1];
    let values = vec![0.25_f32, 0.5, 0.75];

    let ray = voxel_volume_render_ray_values(
        &values,
        shape,
        [1.0, 1.0, 1.0],
        [0, 0, 0],
        [-0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        1.0,
        0.0,
        1.0,
        "one_color",
        "constant",
        128,
        Some([100, 50, 25, 255]),
        None,
        "none",
        None,
        0.1,
        0.5,
        35.0,
        Some(&[0, 2]),
        16,
    )
    .unwrap();

    let sample_alpha = 128.0_f32 / 255.0;
    let expected_alpha = sample_alpha + sample_alpha * (1.0 - sample_alpha);
    assert_eq!(ray.accepted_indices, vec![0, 2]);
    assert_eq!(ray.visited_indices, vec![0, 1, 2]);
    assert_eq!(ray.first_opaque_world, Some([0.5, 0.5, 0.5]));
    assert!((ray.color_rgba[0] - 100.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[1] - 50.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[2] - 25.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[3] - expected_alpha).abs() < 1e-6);
    assert_eq!(
        ray.meshlib_reference,
        "MRVolumeShader fixed-step ray compositing"
    );
}

#[test]
fn voxel_volume_render_ray_values_match_meshlib_clipping_plane_discard_contract() {
    let shape = [3, 1, 1];
    let values = vec![0.25_f32, 0.5, 0.75];

    let ray = voxel_volume_render_ray_values(
        &values,
        shape,
        [1.0, 1.0, 1.0],
        [0, 0, 0],
        [-0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        1.0,
        0.0,
        1.0,
        "one_color",
        "constant",
        128,
        Some([100, 50, 25, 255]),
        Some([1.0, 0.0, 0.0, 0.75]),
        "none",
        None,
        0.1,
        0.5,
        35.0,
        None,
        16,
    )
    .unwrap();

    let sample_alpha = 128.0_f32 / 255.0;
    assert_eq!(ray.visited_indices, vec![0]);
    assert_eq!(ray.accepted_indices, vec![0]);
    assert_eq!(ray.first_opaque_world, Some([0.5, 0.5, 0.5]));
    assert!((ray.color_rgba[0] - 100.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[1] - 50.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[2] - 25.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[3] - sample_alpha).abs() < 1e-6);
}

#[test]
fn voxel_volume_render_ray_values_match_meshlib_value_gradient_zero_normal_skip_contract() {
    let shape = [3, 1, 1];
    let values = vec![0.5_f32, 0.5, 0.5];

    let ray = voxel_volume_render_ray_values(
        &values,
        shape,
        [1.0, 1.0, 1.0],
        [0, 0, 0],
        [-0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        1.0,
        0.0,
        1.0,
        "one_color",
        "constant",
        128,
        Some([100, 50, 25, 255]),
        None,
        "value_gradient",
        None,
        0.1,
        0.5,
        35.0,
        None,
        16,
    )
    .unwrap();

    assert_eq!(ray.visited_indices, vec![0, 1, 2]);
    assert!(ray.accepted_indices.is_empty());
    assert_eq!(ray.first_opaque_world, None);
    assert_eq!(ray.color_rgba, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn voxel_volume_render_ray_values_match_meshlib_voxel_boundary_traversal_branch() {
    let shape = [3, 1, 1];
    let values = vec![0.25_f32, 0.5, 0.75];

    let ray = voxel_volume_render_ray_values(
        &values,
        shape,
        [1.0, 1.0, 1.0],
        [0, 0, 0],
        [-0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
        1.0,
        "one_color",
        "constant",
        128,
        Some([100, 50, 25, 255]),
        None,
        "none",
        None,
        0.1,
        0.5,
        35.0,
        None,
        3,
    )
    .unwrap();

    let sample_alpha = 128.0_f32 / 255.0;
    let expected_alpha = 1.0 - (1.0 - sample_alpha).powi(3);
    assert_eq!(ray.visited_indices, vec![0, 1, 2]);
    assert_eq!(ray.accepted_indices, vec![0, 1, 2]);
    assert_eq!(ray.first_opaque_world, Some([0.0, 0.5, 0.5]));
    assert!((ray.color_rgba[0] - 100.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[1] - 50.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[2] - 25.0 / 255.0).abs() < 1e-6);
    assert!((ray.color_rgba[3] - expected_alpha).abs() < 1e-6);
}

#[test]
fn voxel_move_mesh_to_max_deriv_values_match_meshlib_cubic_shift_contract() {
    let shape = [2, 2, 6];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for z in 0..shape[2] {
        let value = (z as f32 - 2.5).powi(2);
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                values[x + y * shape[0] + z * shape[0] * shape[1]] = value;
            }
        }
    }
    let vertices = vec![[0.25, 0.25, 2.5], [1.25, 0.25, 2.5], [0.25, 1.25, 2.5]];
    let faces = vec![[0_i64, 1, 2]];

    let refined = voxel_move_mesh_to_max_deriv_values(
        &vertices,
        &faces,
        &values,
        shape,
        [1.0, 1.0, 1.0],
        VoxelMaxDerivSettings {
            iters: 1,
            sample_points: 6,
            degree: 3,
            outlier_threshold: 1.0,
            intermediate_smooth_force: 0.0,
            preparation_smooth_force: 0.0,
            smooth_shift_iterations: 0,
            final_relax_iterations: 0,
            final_relax_force: 0.0,
        },
    )
    .unwrap();

    assert_eq!(refined.corrected_indices, vec![0, 1, 2]);
    for (input, output) in vertices.iter().zip(refined.vertices.iter()) {
        assert!((output[0] - input[0]).abs() < 1e-9);
        assert!((output[1] - input[1]).abs() < 1e-9);
        assert!((output[2] - 2.4).abs() < 1e-9);
    }
}

#[test]
fn voxel_move_mesh_to_max_deriv_values_support_meshlib_degree_six_contract() {
    let shape = [2, 2, 7];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for z in 0..shape[2] {
        let value = (z as f32 - 3.0).powi(2);
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                values[x + y * shape[0] + z * shape[0] * shape[1]] = value;
            }
        }
    }
    let vertices = vec![[0.25, 0.25, 3.0], [1.25, 0.25, 3.0], [0.25, 1.25, 3.0]];
    let faces = vec![[0_i64, 1, 2]];

    let refined = voxel_move_mesh_to_max_deriv_values(
        &vertices,
        &faces,
        &values,
        shape,
        [1.0, 1.0, 1.0],
        VoxelMaxDerivSettings {
            iters: 1,
            sample_points: 7,
            degree: 6,
            outlier_threshold: 2.0,
            intermediate_smooth_force: 0.0,
            preparation_smooth_force: 0.0,
            smooth_shift_iterations: 0,
            final_relax_iterations: 0,
            final_relax_force: 0.0,
        },
    )
    .unwrap();

    assert_eq!(refined.corrected_indices, vec![0, 1, 2]);
    for (input, output) in vertices.iter().zip(refined.vertices.iter()) {
        assert!((output[0] - input[0]).abs() < 1e-9);
        assert!((output[1] - input[1]).abs() < 1e-9);
        assert!((output[2] - 2.9).abs() < 1e-9);
    }
}

#[test]
fn voxel_to_mesh_smart_values_runs_meshlib_smart_conversion_in_rust() {
    let shape = [2, 2, 6];
    let mut values = vec![0.0_f32; shape.iter().product()];
    for z in 0..shape[2] {
        let value = (z as f32 - 2.5).powi(2);
        for y in 0..shape[1] {
            for x in 0..shape[0] {
                values[x + y * shape[0] + z * shape[0] * shape[1]] = value;
            }
        }
    }

    let result = voxel_to_mesh_smart_values(
        &values,
        shape,
        [1.0, 1.0, 1.0],
        0.25,
        false,
        VoxelMaxDerivSettings {
            iters: 1,
            sample_points: 6,
            degree: 3,
            outlier_threshold: 1.0,
            intermediate_smooth_force: 0.0,
            preparation_smooth_force: 0.0,
            smooth_shift_iterations: 0,
            final_relax_iterations: 0,
            final_relax_force: 0.0,
        },
    )
    .unwrap();

    assert!(!result.faces.is_empty());
    assert!(!result.corrected_indices.is_empty());
    assert!(result.corrected_indices.len() <= result.vertices.len());
    assert!(result
        .corrected_indices
        .iter()
        .all(|index| *index < result.vertices.len()));
    assert!(result
        .vertices
        .iter()
        .all(|vertex| vertex.iter().all(|coordinate| coordinate.is_finite())));
}

#[test]
fn mesh_surface_path_tri_points_reduces_single_crossing_like_meshlib_compute_surface_path() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path =
        mesh_surface_path_tri_points(&vertices, &faces, 0, [0.8, 0.1, 0.1], 1, [0.1, 0.3, 0.6], 5)
            .unwrap();

    assert_eq!(path.start_point, [0.1, 0.1, 0.0]);
    assert!((path.end_point[0] - 0.9).abs() < 1e-9);
    assert!((path.end_point[1] - 0.7).abs() < 1e-9);
    assert!(path.end_point[2].abs() < 1e-9);
    assert_eq!(path.edges, vec![[1, 2]]);
    assert_eq!(path.positions.len(), 1);
    assert!((path.positions[0] - 31.0 / 70.0).abs() < 1e-9);
    assert_eq!(path.points.len(), 1);
    assert!((path.points[0][0] - 39.0 / 70.0).abs() < 1e-9);
    assert!((path.points[0][1] - 31.0 / 70.0).abs() < 1e-9);
    assert_eq!(path.reached_face_index, Some(1));
    assert_eq!(path.reduce_iterations, 1);
    assert_eq!(path.steps, 1);
    assert!((path.segment_lengths[0] - 4.0 / 7.0).abs() < 1e-9);
    assert!((path.segment_lengths[1] - 3.0 / 7.0).abs() < 1e-9);
    assert!((path.length_mm - 1.0).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_reduces_unfolded_triangle_strip_like_meshlib_compute_surface_path()
{
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3], [2, 3, 4], [4, 3, 5]];

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.8, 0.1, 0.1],
        3,
        [0.1, 0.1, 0.8],
        8,
    )
    .unwrap();
    let path =
        mesh_surface_path_tri_points(&vertices, &faces, 0, [0.8, 0.1, 0.1], 3, [0.1, 0.1, 0.8], 5)
            .unwrap();

    assert_eq!(approximate.edges, vec![[1, 2], [3, 2], [3, 4]]);
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(path.edges, vec![[2, 1], [2, 3], [4, 3]]);
    assert_eq!(path.approximate_edges, vec![[1, 2], [3, 2], [3, 4]]);
    assert_eq!(path.reached_face_index, Some(3));
    assert_eq!(path.reduce_iterations, 1);
    assert_eq!(path.steps, 3);
    assert!((path.positions[0] - 9.0 / 26.0).abs() < 1e-9);
    assert!((path.positions[1] - 0.5).abs() < 1e-9);
    assert!((path.positions[2] - 17.0 / 26.0).abs() < 1e-9);
    assert!((path.points[0][0] - 9.0 / 26.0).abs() < 1e-9);
    assert!((path.points[0][1] - 17.0 / 26.0).abs() < 1e-9);
    assert!((path.points[1][0] - 0.5).abs() < 1e-9);
    assert!((path.points[1][1] - 1.0).abs() < 1e-9);
    assert!((path.points[2][0] - 17.0 / 26.0).abs() < 1e-9);
    assert!((path.points[2][1] - 35.0 / 26.0).abs() < 1e-9);
    assert!((path.length_mm - 3.88_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_collapses_strip_vertex_run_like_meshlib_reduce_path() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
        [0.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![
        [0_i64, 1, 3],
        [4, 3, 1],
        [1, 2, 4],
        [5, 4, 2],
        [3, 4, 6],
        [7, 6, 4],
        [4, 5, 7],
        [8, 7, 5],
    ];

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.8, 0.1, 0.1],
        7,
        [0.8, 0.1, 0.1],
        18,
    )
    .unwrap();
    let path =
        mesh_surface_path_tri_points(&vertices, &faces, 0, [0.8, 0.1, 0.1], 7, [0.8, 0.1, 0.1], 5)
            .unwrap();
    let one_iter_path =
        mesh_surface_path_tri_points(&vertices, &faces, 0, [0.8, 0.1, 0.1], 7, [0.8, 0.1, 0.1], 1)
            .unwrap();

    assert_eq!(
        approximate.edges,
        vec![[1, 3], [4, 3], [4, 6], [4, 7], [5, 7]]
    );
    assert_eq!(
        one_iter_path.edges,
        vec![[3, 1], [3, 4], [6, 4], [7, 4], [7, 5]]
    );
    assert_eq!(one_iter_path.reduce_iterations, 1);
    assert_eq!(one_iter_path.steps, 5);
    assert_eq!(path.edges, vec![[3, 1], [7, 4], [7, 5]]);
    assert_eq!(path.reached_face_index, Some(7));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 3);
    assert!((path.positions[0] - 0.5).abs() < 1e-9);
    assert!((path.positions[1] - 1.0).abs() < 1e-9);
    assert!((path.positions[2] - 0.5).abs() < 1e-9);
    assert!((path.points[1][0] - 1.0).abs() < 1e-9);
    assert!((path.points[1][1] - 1.0).abs() < 1e-9);
    assert!((path.length_mm - (1.8_f64 * 2.0_f64.sqrt())).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_avoids_adjacent_face_vertex_like_meshlib_reduce_path() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]];

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.1, 0.6, 0.3],
        1,
        [0.4, 0.1, 0.5],
        8,
    )
    .unwrap();
    let path =
        mesh_surface_path_tri_points(&vertices, &faces, 0, [0.1, 0.6, 0.3], 1, [0.4, 0.1, 0.5], 5)
            .unwrap();

    assert_eq!(approximate.edges, vec![[0, 1], [0, 1]]);
    assert_eq!(approximate.positions[1], 0.0);
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(path.edges, vec![[0, 2]]);
    assert_eq!(path.approximate_edges, vec![[0, 1], [0, 1]]);
    assert_eq!(path.reached_face_index, Some(1));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 1);
    assert!((path.positions[0] - 21.0 / 110.0).abs() < 1e-9);
    assert!((path.points[0][0]).abs() < 1e-9);
    assert!((path.points[0][1] - 21.0 / 110.0).abs() < 1e-9);
    assert!((path.length_mm - 1.25_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_avoids_non_adjacent_vertex_fan_like_meshlib_reduce_path() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]];

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.1, 0.6, 0.3],
        2,
        [0.4, 0.5, 0.1],
        8,
    )
    .unwrap();
    let path =
        mesh_surface_path_tri_points(&vertices, &faces, 0, [0.1, 0.6, 0.3], 2, [0.4, 0.5, 0.1], 5)
            .unwrap();

    assert_eq!(approximate.edges, vec![[0, 1], [0, 1]]);
    assert!((approximate.positions[1]).abs() < 1e-9);
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(path.edges, vec![[0, 2], [0, 3]]);
    assert_eq!(path.approximate_edges, vec![[0, 1], [0, 1]]);
    assert_eq!(path.reached_face_index, Some(2));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 2);
    assert!((path.positions[0] - 9.0 / 110.0).abs() < 1e-9);
    assert!((path.positions[1] - 9.0 / 40.0).abs() < 1e-9);
    assert!((path.points[0][0]).abs() < 1e-9);
    assert!((path.points[0][1] - 9.0 / 110.0).abs() < 1e-9);
    assert!((path.points[1][0] + 9.0 / 40.0).abs() < 1e-9);
    assert!((path.points[1][1]).abs() < 1e-9);
    assert!((path.length_mm - 1.37_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_removes_repeated_edge_vertex_detour_like_meshlib_reduce_path() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
        [0.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![
        [0_i64, 1, 3],
        [3, 1, 4],
        [1, 2, 4],
        [4, 2, 5],
        [3, 4, 6],
        [6, 4, 7],
        [4, 5, 7],
        [7, 5, 8],
    ];

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.05, 0.1, 0.85],
        2,
        [0.1, 0.05, 0.85],
        16,
    )
    .unwrap();
    let path = mesh_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.05, 0.1, 0.85],
        2,
        [0.1, 0.05, 0.85],
        5,
    )
    .unwrap();

    assert_eq!(approximate.edges, vec![[1, 3], [4, 3], [4, 3]]);
    assert!((approximate.positions[2]).abs() < 1e-9);
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(path.edges, vec![[3, 1], [4, 1]]);
    assert_eq!(path.approximate_edges, vec![[1, 3], [4, 3], [4, 3]]);
    assert_eq!(path.reached_face_index, Some(2));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 2);
    assert!((path.positions[0] - 0.15).abs() < 1e-9);
    assert!((path.positions[1] - 0.15).abs() < 1e-9);
    assert!((path.points[0][0] - 0.15).abs() < 1e-9);
    assert!((path.points[0][1] - 0.85).abs() < 1e-9);
    assert!((path.points[1][0] - 1.0).abs() < 1e-9);
    assert!((path.points[1][1] - 0.85).abs() < 1e-9);
    assert!((path.length_mm - 0.95).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_removes_duplicate_nonvertex_location_like_meshlib_reduce_path() {
    let mut vertices = Vec::new();
    for y in 0..=6 {
        for x in 0..=6 {
            vertices.push([x as f64, y as f64, 0.0]);
        }
    }
    let mut faces = Vec::new();
    let vertex_id = |x: usize, y: usize| y * 7 + x;
    for y in 0..6 {
        for x in 0..6 {
            faces.push([
                vertex_id(x, y) as i64,
                vertex_id(x + 1, y) as i64,
                vertex_id(x, y + 1) as i64,
            ]);
            faces.push([
                vertex_id(x + 1, y + 1) as i64,
                vertex_id(x, y + 1) as i64,
                vertex_id(x + 1, y) as i64,
            ]);
        }
    }

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [
            0.46671207298740064,
            0.48702168304170673,
            0.04626624397089257,
        ],
        12,
        [0.7272053095059872, 0.0827160816645162, 0.19007860882949656],
        80,
    )
    .unwrap();
    let path = mesh_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [
            0.46671207298740064,
            0.48702168304170673,
            0.04626624397089257,
        ],
        12,
        [0.7272053095059872, 0.0827160816645162, 0.19007860882949656],
        5,
    )
    .unwrap();

    assert_eq!(approximate.edges, vec![[1, 7], [7, 1]]);
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(path.edges, vec![[7, 1], [7, 8]]);
    assert_eq!(path.reached_face_index, Some(12));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 2);
    assert!((path.positions[0] - 0.23185930365948093).abs() < 1e-12);
    assert!((path.positions[1] - 0.14990354056320485).abs() < 1e-12);
    assert!((path.points[1][0] - 0.14990354056320485).abs() < 1e-12);
    assert!((path.points[1][1] - 1.0).abs() < 1e-12);
    assert!((path.length_mm - 1.2131651764324607).abs() < 1e-12);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_removes_same_triangle_nonvertex_detour_like_meshlib_reduce_path() {
    let mut vertices = Vec::new();
    for y in 0..=7 {
        for x in 0..=7 {
            vertices.push([x as f64, y as f64, 0.0]);
        }
    }
    let mut faces = Vec::new();
    let vertex_id = |x: usize, y: usize| y * 8 + x;
    for y in 0..7 {
        for x in 0..7 {
            faces.push([
                vertex_id(x, y) as i64,
                vertex_id(x + 1, y) as i64,
                vertex_id(x, y + 1) as i64,
            ]);
            faces.push([
                vertex_id(x + 1, y + 1) as i64,
                vertex_id(x, y + 1) as i64,
                vertex_id(x + 1, y) as i64,
            ]);
        }
    }

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        2,
        [0.2675321287343475, 0.523820181254268, 0.2086476900113846],
        0,
        [0.42935248132098075, 0.41316891553417373, 0.1574786031448455],
        80,
    )
    .unwrap();
    let path = mesh_surface_path_tri_points(
        &vertices,
        &faces,
        2,
        [0.2675321287343475, 0.523820181254268, 0.2086476900113846],
        0,
        [0.42935248132098075, 0.41316891553417373, 0.1574786031448455],
        5,
    )
    .unwrap();

    assert_eq!(approximate.edges, vec![[1, 2], [1, 2]]);
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(path.edges, vec![[1, 9], [1, 8]]);
    assert_eq!(path.reached_face_index, Some(0));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 2);
    assert!((path.positions[0] - 0.18451464196622006).abs() < 1e-12);
    assert!((path.positions[1] - 0.1763882171520069).abs() < 1e-12);
    assert!((path.points[0][0] - 1.0).abs() < 1e-12);
    assert!((path.points[0][1] - 0.18451464196622006).abs() < 1e-12);
    assert!((path.points[1][0] - 0.8236117828479931).abs() < 1e-12);
    assert!((path.points[1][1] - 0.1763882171520069).abs() < 1e-12);
    assert!((path.length_mm - 1.1118293526870044).abs() < 1e-12);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_surface_path_tri_points_collapses_repeated_location_strip_vertex_run_like_meshlib_reduce_path(
) {
    let mut vertices = Vec::new();
    for y in 0..=8 {
        for x in 0..=8 {
            vertices.push([x as f64, y as f64, 0.0]);
        }
    }
    let mut faces = Vec::new();
    let vertex_id = |x: usize, y: usize| y * 9 + x;
    for y in 0..8 {
        for x in 0..8 {
            faces.push([
                vertex_id(x, y) as i64,
                vertex_id(x + 1, y) as i64,
                vertex_id(x, y + 1) as i64,
            ]);
            faces.push([
                vertex_id(x + 1, y + 1) as i64,
                vertex_id(x, y + 1) as i64,
                vertex_id(x + 1, y) as i64,
            ]);
        }
    }

    let approximate = mesh_fast_marching_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.27924020234514274, 0.3661046568847575, 0.35465514077009963],
        66,
        [0.04181337750965728, 0.8199993556651197, 0.13818726682522295],
        120,
    )
    .unwrap();
    let path = mesh_surface_path_tri_points(
        &vertices,
        &faces,
        0,
        [0.27924020234514274, 0.3661046568847575, 0.35465514077009963],
        66,
        [0.04181337750965728, 0.8199993556651197, 0.13818726682522295],
        5,
    )
    .unwrap();

    assert_eq!(
        approximate.edges,
        vec![
            [1, 9],
            [10, 9],
            [10, 18],
            [10, 19],
            [11, 19],
            [20, 19],
            [20, 28],
            [29, 28],
            [29, 37],
            [29, 38],
            [38, 29],
        ]
    );
    assert!(path.length_mm < approximate.length_mm);
    assert_eq!(
        path.edges,
        vec![
            [9, 1],
            [10, 11],
            [19, 11],
            [19, 20],
            [28, 20],
            [28, 29],
            [37, 29],
            [37, 38],
        ]
    );
    assert_eq!(path.reached_face_index, Some(66));
    assert_eq!(path.reduce_iterations, 2);
    assert_eq!(path.steps, 8);
    assert!((path.positions[0] - 0.5044751236295062).abs() < 1e-12);
    assert!((path.positions[1]).abs() < 1e-12);
    assert!((path.positions[7] - 0.78389141814473).abs() < 1e-12);
    assert!((path.points[1][0] - 1.0).abs() < 1e-12);
    assert!((path.points[1][1] - 1.0).abs() < 1e-12);
    assert!((path.length_mm - 4.148145908127404).abs() < 1e-12);
    assert_eq!(
        path.meshlib_reference,
        "MR::computeSurfacePath / MR::reducePath"
    );
}

#[test]
fn mesh_geodesic_distance_field_marks_disconnected_vertices_unreachable() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [4.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [3, 4, 5]];

    let field = mesh_geodesic_distance_field(&vertices, &faces, &[0], f64::INFINITY).unwrap();

    assert_eq!(field.reachable_vertex_count, 3);
    assert!(field.distances_mm[5].is_infinite());
    assert_eq!(field.predecessor_vertices[5], None);
}

#[test]
fn mesh_closest_surface_path_targets_match_meshlib_target_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [4.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3], [4, 5, 6]];

    let targets =
        mesh_closest_surface_path_targets(&vertices, &faces, &[3, 2, 6], &[0, 1], f64::INFINITY)
            .unwrap();

    assert_eq!(targets.start_vertices, vec![2, 3, 6]);
    assert_eq!(targets.end_vertices, vec![0, 1]);
    assert_eq!(targets.target_vertices, vec![Some(0), Some(1), None]);
    assert_eq!(targets.target_distances_mm[0], 1.0);
    assert_eq!(targets.target_distances_mm[1], 1.0);
    assert!(targets.target_distances_mm[2].is_infinite());
    assert_eq!(targets.predecessor_vertices[3], Some(1));
}

#[test]
fn mesh_geodesic_iso_region_extracts_surface_distance_iso_segment() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let region = mesh_geodesic_iso_region(&vertices, &faces, &[0], 0.5, f64::INFINITY).unwrap();

    assert_eq!(region.selected_vertex_indices, vec![0]);
    assert_eq!(region.selected_face_indices, Vec::<usize>::new());
    assert_eq!(region.crossing_face_indices, vec![0]);
    assert_eq!(region.boundary_edges, vec![[0, 1], [0, 2]]);
    assert_eq!(region.iso_segments.len(), 1);
    let segment = region.iso_segments[0];
    assert!((segment[0][0] - 0.5).abs() < 1e-9);
    assert!((segment[0][1] - 0.0).abs() < 1e-9);
    assert!((segment[1][0] - 0.0).abs() < 1e-9);
    assert!((segment[1][1] - 0.5).abs() < 1e-9);
    assert_eq!(region.clipped_vertices.len(), 3);
    assert_eq!(region.clipped_faces, vec![[0, 1, 2]]);
    assert_eq!(region.clipped_source_face_indices, vec![0]);
    assert_eq!(
        region.clipped_source_vertex_indices,
        vec![Some(0), None, None]
    );
    assert!((region.clipped_vertices[1][0] - 0.5).abs() < 1e-9);
    assert!((region.clipped_vertices[2][1] - 0.5).abs() < 1e-9);
}

#[test]
fn mesh_geodesic_extreme_edges_match_meshlib_ridge_and_gorge_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let ridge = mesh_geodesic_extreme_edges(
        &vertices,
        &faces,
        &[0.0, 1.0, 1.0, 0.0],
        MeshExtremeEdgeType::Ridge,
    )
    .unwrap();

    assert_eq!(ridge.edge_indices, vec![[1, 2]]);
    assert_eq!(ridge.meshlib_reference, "MR::findExtremeEdges");

    let gorge = mesh_geodesic_extreme_edges(
        &vertices,
        &faces,
        &[1.0, 0.0, 0.0, 1.0],
        MeshExtremeEdgeType::Gorge,
    )
    .unwrap();

    assert_eq!(gorge.edge_indices, vec![[1, 2]]);
}

#[test]
fn mesh_geodesic_quadrangle_path_matches_meshlib_reduce_path_crossing_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_geodesic_quadrangle_path(&vertices, &faces, 0, 3).unwrap();

    assert_eq!(path.start_face_index, 0);
    assert_eq!(path.end_face_index, 1);
    assert_eq!(path.shared_edge, [1, 2]);
    assert!((path.crossing_t - 0.5).abs() < 1e-9);
    assert!((path.crossing_point[0] - 0.5).abs() < 1e-9);
    assert!((path.crossing_point[1] - 0.5).abs() < 1e-9);
    assert!(
        path.graph_vertex_indices == vec![0, 1, 3] || path.graph_vertex_indices == vec![0, 2, 3]
    );
    assert!((path.graph_length_mm - 2.0).abs() < 1e-9);
    assert!((path.length_mm - 2.0_f64.sqrt()).abs() < 1e-9);
    assert!(path.unfolded_quadrangle_convex);
    assert_eq!(
        path.meshlib_reference,
        "MR::shortestPathInQuadrangle / MR::reducePath"
    );
}

#[test]
fn mesh_planar_triangle_strip_path_matches_meshlib_funnel_crossing_contract() {
    let strip = mesh_planar_triangle_strip_path(
        [0.0, 0.0],
        &[[[0.0, 1.0], [1.0, 0.0]], [[1.0, 1.0], [1.0, 0.0]]],
        [2.0, 1.0],
    )
    .unwrap();

    assert!((strip.crossing_positions[0] - (2.0 / 3.0)).abs() < 1e-9);
    assert!((strip.crossing_positions[1] - 0.5).abs() < 1e-9);
    assert_eq!(strip.points.len(), 4);
    assert!((strip.crossing_points[0][0] - (2.0 / 3.0)).abs() < 1e-9);
    assert!((strip.crossing_points[0][1] - (1.0 / 3.0)).abs() < 1e-9);
    assert!((strip.crossing_points[1][0] - 1.0).abs() < 1e-9);
    assert!((strip.crossing_points[1][1] - 0.5).abs() < 1e-9);
    assert!((strip.length_mm - 5.0_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        strip.meshlib_reference,
        "MR::PathInPlanarTriangleStrip / MR::reducePath"
    );
}

#[test]
fn mesh_triangle_strip_unfolded_path_matches_meshlib_unfolder_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3], [1, 4, 3]];

    let path = mesh_triangle_strip_unfolded_path(
        &vertices,
        &faces,
        0,
        &[[1_i64, 2], [1, 3]],
        2,
        [0.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
    )
    .unwrap();

    assert_eq!(path.strip_face_indices, vec![0, 1, 2]);
    assert_eq!(path.crossed_edges, vec![[1, 2], [1, 3]]);
    assert_eq!(path.oriented_edges, vec![[2, 1], [3, 1]]);
    assert!((path.crossing_positions[0] - (2.0 / 3.0)).abs() < 1e-9);
    assert!((path.crossing_positions[1] - 0.5).abs() < 1e-9);
    assert!((path.crossing_points[0][0] - (2.0 / 3.0)).abs() < 1e-9);
    assert!((path.crossing_points[0][1] - (1.0 / 3.0)).abs() < 1e-9);
    assert!((path.crossing_points[1][0] - 1.0).abs() < 1e-9);
    assert!((path.crossing_points[1][1] - 0.5).abs() < 1e-9);
    assert_eq!(path.points.len(), 4);
    assert!((path.length_mm - 5.0_f64.sqrt()).abs() < 1e-9);
    assert!((path.planar_length_mm - 5.0_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::TriangleStripUnfolder / MR::reducePath"
    );
}

#[test]
fn mesh_surface_edge_point_path_matches_meshlib_surface_path_length_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_surface_edge_point_path(
        &vertices,
        &faces,
        &[[0_i64, 1], [1, 3], [2, 3]],
        &[0.5, 0.5, 0.5],
    )
    .unwrap();

    assert_eq!(path.edges, vec![[0, 1], [1, 3], [2, 3]]);
    assert_eq!(path.positions, vec![0.5, 0.5, 0.5]);
    assert_eq!(
        path.points,
        vec![[0.5, 0.0, 0.0], [1.0, 0.5, 0.0], [0.5, 1.0, 0.0]]
    );
    assert_eq!(path.segment_lengths.len(), 2);
    assert!((path.segment_lengths[0] - 0.5_f64.sqrt()).abs() < 1e-9);
    assert!((path.segment_lengths[1] - 0.5_f64.sqrt()).abs() < 1e-9);
    assert!((path.length_mm - 2.0_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::surfacePathLength / MR::surfacePathToContour3f"
    );
}

#[test]
fn mesh_geodesic_edge_point_path_matches_meshlib_geodesic_path_length_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let path = mesh_geodesic_edge_point_path(
        &vertices,
        &faces,
        [0.0, 0.0, 0.0],
        &[[1_i64, 2]],
        &[0.5],
        [1.0, 1.0, 0.0],
    )
    .unwrap();

    assert_eq!(path.start_point, [0.0, 0.0, 0.0]);
    assert_eq!(path.end_point, [1.0, 1.0, 0.0]);
    assert_eq!(path.mid_points, vec![[0.5, 0.5, 0.0]]);
    assert_eq!(
        path.points,
        vec![[0.0, 0.0, 0.0], [0.5, 0.5, 0.0], [1.0, 1.0, 0.0]]
    );
    assert_eq!(path.segment_lengths.len(), 2);
    assert!((path.segment_lengths[0] - 0.5_f64.sqrt()).abs() < 1e-9);
    assert!((path.segment_lengths[1] - 0.5_f64.sqrt()).abs() < 1e-9);
    assert!((path.length_mm - 2.0_f64.sqrt()).abs() < 1e-9);
    assert_eq!(
        path.meshlib_reference,
        "MR::geodesicPathLength / MR::geodesicPathToContour3f"
    );
}

#[test]
fn mesh_steepest_descent_triangle_step_matches_meshlib_triangle_exit_contract() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let faces = vec![[0_i64, 1, 2]];
    let scalars = vec![0.0, 1.0, 0.0];

    let step =
        mesh_steepest_descent_triangle_step(&vertices, &faces, &scalars, 0, [0.5, 0.25, 0.25])
            .unwrap();

    assert_eq!(step.face_index, 0);
    assert_eq!(step.start_barycentric, [0.5, 0.25, 0.25]);
    assert_eq!(step.start_point, [0.25, 0.25, 0.0]);
    assert!((step.start_value - 0.25).abs() < 1e-9);
    assert_eq!(step.gradient, [1.0, 0.0, 0.0]);
    assert!((step.gradient_norm - 1.0).abs() < 1e-9);
    assert_eq!(step.crossed_edge, Some([2, 0]));
    assert!((step.edge_position.unwrap() - 0.75).abs() < 1e-9);
    assert_eq!(step.crossing_point, Some([0.0, 0.25, 0.0]));
    assert_eq!(step.kind, "edge");
    assert_eq!(
        step.meshlib_reference,
        "MR::findSteepestDescentPoint(MeshTriPoint)"
    );
}

#[test]
fn mesh_steepest_descent_edge_step_matches_meshlib_edgepoint_vertex_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];
    let scalars = vec![0.0, 1.0, 1.0, 1.0];

    let step = mesh_steepest_descent_edge_step(&vertices, &faces, &scalars, [1, 2], 0.5).unwrap();

    assert_eq!(step.start_edge, [1, 2]);
    assert!((step.edge_position - 0.5).abs() < 1e-9);
    assert_eq!(step.start_point, [0.5, 0.5, 0.0]);
    assert!((step.start_value - 1.0).abs() < 1e-9);
    assert_eq!(step.crossed_edge, Some([0, 1]));
    let crossing_point = step.crossing_point.unwrap();
    assert!(crossing_point[0].abs() < 1e-9);
    assert!(crossing_point[1].abs() < 1e-9);
    assert!(crossing_point[2].abs() < 1e-9);
    assert!(step.crossing_edge_position.unwrap().abs() < 1e-9);
    assert_eq!(step.kind, "vertex");
    assert_eq!(step.side, "left");
    assert_eq!(
        step.meshlib_reference,
        "MR::findSteepestDescentPoint(MeshEdgePoint)"
    );
}

#[test]
fn mesh_steepest_descent_vertex_step_matches_meshlib_vertid_triangle_exit_contract() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let faces = vec![[0_i64, 1, 2]];
    let scalars = vec![1.0, 0.0, 0.0];

    let step = mesh_steepest_descent_vertex_step(&vertices, &faces, &scalars, 0).unwrap();

    assert_eq!(step.start_vertex, 0);
    assert_eq!(step.start_point, [0.0, 0.0, 0.0]);
    assert!((step.start_value - 1.0).abs() < 1e-9);
    assert_eq!(step.crossed_edge, Some([1, 2]));
    assert!((step.edge_position.unwrap() - 0.5).abs() < 1e-9);
    assert_eq!(step.crossing_point, Some([0.5, 0.5, 0.0]));
    assert_eq!(step.kind, "edge");
    assert_eq!(step.source, "face");
    assert_eq!(step.gradient_norm, Some(2.0_f64.sqrt()));
    assert_eq!(
        step.meshlib_reference,
        "MR::findSteepestDescentPoint(VertId)"
    );
}

#[test]
fn mesh_steepest_descent_path_matches_meshlib_descent_path_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];
    let scalars = vec![0.0, 1.0, 1.0, 2.0];

    let path =
        mesh_steepest_descent_path(&vertices, &faces, &scalars, 1, [0.25, 0.25, 0.5], 8).unwrap();

    assert_eq!(path.start_face_index, 1);
    assert_eq!(path.start_barycentric, [0.25, 0.25, 0.5]);
    assert_eq!(path.start_point, [0.75, 0.75, 0.0]);
    assert!((path.start_value - 1.5).abs() < 1e-9);
    assert_eq!(path.edges, vec![[2, 1], [0, 1]]);
    assert!((path.positions[0] - 0.5).abs() < 1e-9);
    assert!(path.positions[1].abs() < 1e-9);
    assert_eq!(path.points[0], [0.5, 0.5, 0.0]);
    assert!(path.points[1][0].abs() < 1e-9);
    assert!(path.points[1][1].abs() < 1e-9);
    assert!(path.points[1][2].abs() < 1e-9);
    assert_eq!(path.reached_vertex, Some(0));
    assert_eq!(path.stopped_reason, "local_minimum");
    assert_eq!(path.steps, 2);
    assert!((path.length_mm - 1.5 * 0.5_f64.sqrt()).abs() < 1e-9);
    assert_eq!(path.meshlib_reference, "MR::computeSteepestDescentPath");
}

#[test]
fn mesh_surface_distance_seed_vertices_supports_selected_edges_and_triangle_boundaries() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [2, 1, 3]];

    let sources =
        mesh_surface_distance_seed_vertices(&vertices, &faces, &[0], &[[1_i64, 3]], &[0]).unwrap();

    assert_eq!(sources.seed_vertices, vec![0, 1, 2, 3]);
    assert_eq!(sources.selected_edges, vec![[1, 3]]);
    assert_eq!(sources.selected_face_indices, vec![0]);
    assert_eq!(
        sources.selected_face_boundary_edges,
        vec![[0, 1], [0, 2], [1, 2]]
    );
    assert_eq!(
        sources.meshlib_reference,
        "Surface Distance selected edges / selected triangles boundary"
    );
}

#[test]
fn extract_selected_faces_as_mesh_rejects_empty_selection() {
    let (vertices, faces) = cube();
    let error = extract_selected_faces_as_mesh(&vertices, &faces, &[]).unwrap_err();

    assert!(error.to_string().contains("selected_face_ids"));
}

#[test]
fn select_outer_layer_faces_matches_meshlib_double_layer_seed_contract() {
    let vertices = vec![
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let selected = select_outer_layer_faces(&vertices, &faces, 1e-8).unwrap();

    assert_eq!(selected, vec![0]);
}

#[test]
fn select_overlapping_faces_matches_meshlib_opposite_close_triangle_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 5e-6],
        [1.0, 0.0, 5e-6],
        [0.0, 1.0, 5e-6],
    ];
    let faces = vec![[0, 1, 2], [3, 5, 4]];

    let selected = select_overlapping_faces(&vertices, &faces, 1e-10, -0.99, 1e-5).unwrap();

    assert_eq!(selected, vec![0, 1]);
}

#[test]
fn select_overlapping_faces_rejects_same_orientation_and_far_triangles_like_meshlib() {
    let same_orientation_vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 5e-6],
        [1.0, 0.0, 5e-6],
        [0.0, 1.0, 5e-6],
    ];
    let same_orientation_faces = vec![[0, 1, 2], [3, 4, 5]];
    assert_eq!(
        select_overlapping_faces(
            &same_orientation_vertices,
            &same_orientation_faces,
            1e-10,
            -0.99,
            1e-5
        )
        .unwrap(),
        Vec::<i64>::new()
    );

    let far_vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1e-4],
        [1.0, 0.0, 1e-4],
        [0.0, 1.0, 1e-4],
    ];
    let far_faces = vec![[0, 1, 2], [3, 5, 4]];
    assert_eq!(
        select_overlapping_faces(&far_vertices, &far_faces, 1e-10, -0.99, 1e-5).unwrap(),
        Vec::<i64>::new()
    );
}

#[test]
fn graph_cut_select_region_matches_meshlib_source_sink_edge_length_cut_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [5.0, 5.0, 0.0],
        [0.0, 1.0, 0.0],
        [10.0, 1.0, 0.0],
        [5.0, 5.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3], [0, 3, 4], [3, 4, 5]];

    let selected = graph_cut_select_region(&vertices, &faces, &[0], &[3], 1.0).unwrap();

    assert_eq!(selected, vec![0, 1]);
}

#[test]
fn graph_cut_select_region_auto_not_region_uses_uncertainty_distance_sink_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [5.0, 5.0, 0.0],
        [0.0, 1.0, 0.0],
        [10.0, 1.0, 0.0],
        [5.0, 5.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3], [0, 3, 4], [3, 4, 5]];

    let selected =
        graph_cut_select_region_auto_not_region(&vertices, &faces, &[0], 12.0, 1.0).unwrap();

    assert_eq!(selected, vec![0, 1]);
}

#[test]
fn graph_cut_select_region_uses_meshlib_curvature_preference_metric() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, -1.0],
        [2.0, 1.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3], [3, 0, 4], [3, 4, 5]];

    assert_eq!(
        graph_cut_select_region(&vertices, &faces, &[0], &[3], 1.0).unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        graph_cut_select_region_with_curvature_preference(
            &vertices,
            &faces,
            &[0],
            &[3],
            1.0,
            "convex"
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        graph_cut_select_region_with_curvature_preference(
            &vertices,
            &faces,
            &[0],
            &[3],
            1.0,
            "concave"
        )
        .unwrap(),
        vec![0, 1, 2]
    );
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn unite_close_vertices_merges_boundary_vertices_like_meshlib_default() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.001, 0.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [4, 2, 3]];

    let repaired = unite_close_vertices(&vertices, &faces, 0.01, true).unwrap();

    assert_eq!(repaired.changed_count, 1);
    assert_eq!(repaired.vertices.len(), 4);
    assert_eq!(repaired.faces, vec![[0, 1, 2], [0, 2, 3]]);
}

#[test]
fn unite_close_vertices_boundary_mode_preserves_closed_internal_vertices_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.001, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    let faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];

    let boundary_only = unite_close_vertices(&vertices, &faces, 0.01, true).unwrap();

    assert_eq!(boundary_only.changed_count, 0);
    assert_eq!(boundary_only.vertices, vertices);
    assert_eq!(boundary_only.faces, faces);
}

#[test]
fn pairwise_point_to_point_icp_recovers_meshlib_style_rigid_transform() {
    let angle = 0.15_f64;
    let cos = angle.cos();
    let sin = angle.sin();
    let translation = [0.03, -0.02, 0.04];
    let reference = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.1],
        [0.2, 0.9, -0.2],
        [-0.1, 0.2, 1.1],
        [0.7, 0.4, 0.8],
    ];
    let floating = reference
        .iter()
        .map(|point| {
            [
                cos * point[0] - sin * point[1] + translation[0],
                sin * point[0] + cos * point[1] + translation[1],
                point[2] + translation[2],
            ]
        })
        .collect::<Vec<_>>();

    let result =
        pairwise_point_to_point_icp(&floating, &reference, 25, 1e-12, IcpMode::AnyRigidXf).unwrap();

    assert!(result.mean_square_distance < 1e-12);
    assert_eq!(result.active_pair_count, floating.len());
    assert!((result.rotation[0][0] - cos).abs() < 1e-8);
    assert!((result.rotation[0][1] - sin).abs() < 1e-8);
    assert!((result.rotation[1][0] + sin).abs() < 1e-8);
    assert!((result.rotation[1][1] - cos).abs() < 1e-8);
}

#[test]
fn pairwise_point_to_point_icp_supports_meshlib_translation_only_mode() {
    let reference = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.2, 0.1, 1.0],
    ];
    let translation = [0.25, -0.1, 0.05];
    let floating = reference
        .iter()
        .map(|point| {
            [
                point[0] + translation[0],
                point[1] + translation[1],
                point[2] + translation[2],
            ]
        })
        .collect::<Vec<_>>();

    let result =
        pairwise_point_to_point_icp(&floating, &reference, 10, 1e-12, IcpMode::TranslationOnly)
            .unwrap();

    assert!(result.mean_square_distance < 1e-12);
    assert_eq!(
        result.rotation,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    );
    assert!((result.translation[0] + translation[0]).abs() < 1e-10);
    assert!((result.translation[1] + translation[1]).abs() < 1e-10);
    assert!((result.translation[2] + translation[2]).abs() < 1e-10);
}

#[test]
fn distance_map_from_contours_marks_inside_negative_like_meshlib_winding_rule() {
    let contours = vec![vec![
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [0.0, 2.0],
        [0.0, 0.0],
    ]];

    let map = distance_map_from_contours(&contours, 3, 3, [0.0, 0.0], [1.0, 1.0], true).unwrap();

    assert_eq!(map.width, 3);
    assert_eq!(map.height, 3);
    assert_eq!(map.valid_count, 9);
    assert_eq!(map.values.len(), 9);
    assert!((map.values[0] + 0.5).abs() < 1e-6);
    assert!((map.values[4] + 0.5).abs() < 1e-6);
    assert!((map.values[2] - 0.5).abs() < 1e-6);
    assert!(map.min_value < 0.0);
    assert!(map.max_value > 0.0);
}

#[test]
fn distance_map_from_open_contour_stays_unsigned_even_when_sign_requested() {
    let contours = vec![vec![[0.0, 0.0], [2.0, 0.0]]];

    let map = distance_map_from_contours(&contours, 3, 2, [0.0, 0.0], [1.0, 1.0], true).unwrap();

    assert_eq!(map.valid_count, 6);
    assert!(map.values.iter().all(|value| *value >= 0.0));
    assert!((map.values[0] - 0.5).abs() < 1e-6);
    assert!((map.values[1] - 0.5).abs() < 1e-6);
    assert!((map.values[2] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
}

#[test]
fn object_lines_from_contours_serializes_meshlib_polyline_edges() {
    let object = lines::object_lines_from_contours(
        &[
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            vec![[2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 1.0, 0.0]],
        ],
        lines::ObjectLinesOptions {
            line_width: 2.5,
            show_points: 1,
            smooth_connections: 0,
            ..lines::ObjectLinesOptions::default()
        },
    )
    .unwrap();

    assert_eq!(
        object.points,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.0, 1.0, 0.0],
        ]
    );
    assert_eq!(object.lines, vec![[0, 1], [1, 2], [2, 0], [3, 4], [4, 5]]);
    assert_eq!(object.line_width, 2.5);
    assert_eq!(object.show_points, 1);
    assert_eq!(object.smooth_connections, 0);
    assert_eq!(object.coloring_type, lines::ObjectLinesColoringType::Solid);
}

#[test]
fn object_lines_to_contours_roundtrips_meshlib_closed_and_open_components() {
    let object = lines::ObjectLinesDocument {
        points: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [3.0, 1.0, 0.0],
        ],
        lines: vec![[0, 1], [1, 2], [2, 0], [3, 4], [4, 5]],
        ..lines::ObjectLinesDocument::default()
    };

    let contours = lines::object_lines_to_contours(&object).unwrap();

    assert_eq!(
        contours,
        vec![
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            vec![[2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 1.0, 0.0]],
        ]
    );
}

#[test]
fn offset_contours_matches_meshlib_closed_clockwise_round_corner_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours(&contours, 0.25, std::f64::consts::PI / 9.0).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 25);
    let expected = [
        [0.0, -0.25, 0.0],
        [-0.077254, -0.237764, 0.0],
        [-0.146946, -0.202254, 0.0],
        [-0.202254, -0.146946, 0.0],
        [-0.237764, -0.077254, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.237764, 2.077254, 0.0],
        [-0.202254, 2.146946, 0.0],
        [-0.146946, 2.202254, 0.0],
        [-0.077254, 2.237764, 0.0],
        [0.0, 2.25, 0.0],
        [2.0, 2.25, 0.0],
        [2.077254, 2.237764, 0.0],
        [2.146946, 2.202254, 0.0],
        [2.202254, 2.146946, 0.0],
        [2.237764, 2.077254, 0.0],
        [2.25, 2.0, 0.0],
        [2.25, 0.0, 0.0],
        [2.237764, -0.077254, 0.0],
        [2.202254, -0.146946, 0.0],
        [2.146946, -0.202254, 0.0],
        [2.077254, -0.237764, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_round_corner_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 25);
    let expected = [
        [-0.0, -0.200000, 0.0],
        [-0.077044, -0.185038, 0.0],
        [-0.132326, -0.163134, 0.0],
        [-0.169086, -0.128711, 0.0],
        [-0.190564, -0.076192, 0.0],
        [-0.200000, -0.0, 0.0],
        [-0.300000, 2.000000, 0.0],
        [-0.294688, 2.116414, 0.0],
        [-0.263973, 2.199443, 0.0],
        [-0.205915, 2.254265, 0.0],
        [-0.118571, 2.286058, 0.0],
        [-0.0, 2.300000, 0.0],
        [2.000000, 2.400000, 0.0],
        [2.155218, 2.392917, 0.0],
        [2.265924, 2.351964, 0.0],
        [2.339020, 2.274553, 0.0],
        [2.381411, 2.158094, 0.0],
        [2.400000, 2.000000, 0.0],
        [2.500000, -0.0, 0.0],
        [2.490793, -0.201161, 0.0],
        [2.438895, -0.353819, 0.0],
        [2.341601, -0.455896, 0.0],
        [2.196205, -0.505316, 0.0],
        [2.000000, -0.500000, 0.0],
        [-0.0, -0.200000, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_sharp_corner_max_angle_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            corner_type: lines::OffsetContoursCornerType::Sharp,
            max_sharp_angle: std::f64::consts::PI / 6.0,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 17);
    let expected = [
        [0.000000, -0.200000, 0.0],
        [-0.063326, -0.190501, 0.0],
        [-0.197180, -0.056394, 0.0],
        [-0.200000, -0.000000, 0.0],
        [-0.300000, 2.000000, 0.0],
        [-0.303800, 2.076005, 0.0],
        [-0.084555, 2.295772, 0.0],
        [0.000000, 2.300000, 0.0],
        [2.000000, 2.400000, 0.0],
        [2.101339, 2.405067, 0.0],
        [2.394363, 2.112740, 0.0],
        [2.400000, 2.000000, 0.0],
        [2.500000, -0.000000, 0.0],
        [2.506352, -0.127042, 0.0],
        [2.115096, -0.517264, 0.0],
        [2.000000, -0.500000, 0.0],
        [0.000000, -0.200000, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_negative_offset_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours(&contours, -0.25, std::f64::consts::PI / 9.0).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
    let expected = [
        [0.25, 0.25, 0.0],
        [0.25, 1.75, 0.0],
        [1.75, 1.75, 0.0],
        [1.75, 0.25, 0.0],
        [0.25, 0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_negative_offset_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![-0.20, -0.30, -0.40, -0.50, -0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
    let expected = [
        [0.211587, 0.231738, 0.0],
        [0.284289, 1.685786, 0.0],
        [1.581047, 1.620948, 0.0],
        [1.521411, 0.428212, 0.0],
        [0.211587, 0.231738, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_sharp_corner_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            corner_type: lines::OffsetContoursCornerType::Sharp,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 13);
    let expected = [
        [0.0, -0.25, 0.0],
        [-0.25, -0.25, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.25, 0.0],
        [0.0, 2.25, 0.0],
        [2.0, 2.25, 0.0],
        [2.25, 2.25, 0.0],
        [2.25, 2.0, 0.0],
        [2.25, 0.0, 0.0],
        [2.25, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_sharp_corner_max_angle_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            corner_type: lines::OffsetContoursCornerType::Sharp,
            max_sharp_angle: std::f64::consts::PI / 6.0,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 17);
    let expected = [
        [0.0, -0.25, 0.0],
        [-0.066987, -0.25, 0.0],
        [-0.25, -0.066987, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.066988, 0.0],
        [-0.066987, 2.25, 0.0],
        [0.0, 2.25, 0.0],
        [2.0, 2.25, 0.0],
        [2.066988, 2.25, 0.0],
        [2.25, 2.066988, 0.0],
        [2.25, 2.0, 0.0],
        [2.25, 0.0, 0.0],
        [2.25, -0.066987, 0.0],
        [2.066988, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_default_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours(&contours, 0.25, std::f64::consts::PI / 9.0).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 25);
    let expected_z = [
        0.111672, 0.000000, 0.000000, 0.000000, 0.000000, 0.037224, 1.962776, 2.000000, 2.000000,
        2.000000, 2.000000, 2.037224, 3.962776, 4.000000, 4.000000, 4.000000, 4.000000, 4.037224,
        5.962776, 6.000000, 6.000000, 6.000000, 6.000000, 5.888328, 0.111672,
    ];
    for (actual, expected) in result[0].iter().zip(expected_z) {
        assert!((actual[2] - expected).abs() <= 1e-5);
    }
}

#[test]
fn offset_contours_exposes_meshlib_restore_z_relax_iterations() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options_and_z_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
        lines::OffsetContoursZOptions {
            relax_iterations: 0,
            ..lines::OffsetContoursZOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 25);
    let expected_z = [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0,
        6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 0.0,
    ];
    for (actual, expected) in result[0].iter().zip(expected_z) {
        assert!((actual[2] - expected).abs() <= 1e-5);
    }
}

#[test]
fn offset_contours_exposes_meshlib_constant_z_callback_mode() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];
    let z_options = lines::OffsetContoursZOptions {
        restore_mode: lines::OffsetContoursZRestoreMode::Constant(9.0),
        relax_iterations: 2,
    };

    let result = lines::offset_contours_with_options_and_z_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
        z_options.clone(),
    )
    .unwrap();
    assert!(result
        .iter()
        .flatten()
        .all(|point| (point[2] - 9.0).abs() <= 1e-8));

    let with_origins = lines::offset_contours_with_options_and_origins_and_z_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
        z_options,
    )
    .unwrap();
    assert!(with_origins
        .contours
        .iter()
        .flatten()
        .all(|point| (point[2] - 9.0).abs() <= 1e-8));
    assert_eq!(
        with_origins.contours[0].len(),
        with_origins.origins[0].len()
    );
}

#[test]
fn offset_contours_exposes_meshlib_custom_z_callback_mode() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];
    let z_options = lines::OffsetContoursZOptions {
        restore_mode: lines::OffsetContoursZRestoreMode::Custom(vec![vec![
            10.0, 12.0, 14.0, 16.0, 10.0,
        ]]),
        relax_iterations: 0,
    };

    let result = lines::offset_contours_with_options_and_z_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
        z_options,
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 25);
    let expected_z = [
        10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 12.0, 12.0, 12.0, 12.0, 12.0, 12.0, 14.0, 14.0, 14.0,
        14.0, 14.0, 14.0, 16.0, 16.0, 16.0, 16.0, 16.0, 16.0, 10.0,
    ];
    for (actual, expected) in result[0].iter().zip(expected_z) {
        assert!((actual[2] - expected).abs() <= 1e-8);
    }

    let invalid = lines::offset_contours_with_options_and_z_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
        lines::OffsetContoursZOptions {
            restore_mode: lines::OffsetContoursZRestoreMode::Custom(vec![vec![10.0, 12.0, 14.0]]),
            relax_iterations: 0,
        },
    );
    assert!(invalid.unwrap_err().contains("z_values"));
}

#[test]
fn offset_contours_exposes_meshlib_callable_z_callback_context() {
    use std::cell::RefCell;

    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];
    let seen = RefCell::new(Vec::new());

    let result = lines::offset_contours_with_options_and_z_callback(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
        0,
        |offset_contours, offset_index, origin| {
            let contour_id = offset_index.contour_id as usize;
            let vert_id = offset_index.vert_id as usize;
            let point = offset_contours[contour_id][vert_id];
            seen.borrow_mut().push((
                offset_index.contour_id,
                offset_index.vert_id,
                origin.l_org.vert_id,
            ));
            Ok(point[0] + 10.0 * point[1] + vert_id as f64 * 0.01 + origin.l_org.vert_id as f64)
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 25);
    let seen = seen.borrow();
    assert_eq!(seen.len(), result[0].len());
    assert_eq!(seen[0], (0, 0, 0));
    assert_eq!(seen[6], (0, 6, 1));
    assert_eq!(seen[12], (0, 12, 2));
    for (vert_id, point) in result[0].iter().enumerate() {
        let expected = point[0] + 10.0 * point[1] + vert_id as f64 * 0.01 + seen[vert_id].2 as f64;
        assert!((point[2] - expected).abs() <= 1e-8);
    }
}

#[test]
fn offset_contours_matches_meshlib_fixed_shell_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[1].len(), 5);
    let expected_inner = [
        [0.250000, 1.750000, 2.125000],
        [0.250000, 0.250000, 2.125000],
        [1.750000, 0.250000, 3.875000],
        [1.750000, 1.750000, 3.875000],
        [0.250000, 1.750000, 2.125000],
    ];
    for (actual, expected) in result[1].iter().zip(expected_inner) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_variable_shell_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[1].len(), 5);
    let expected_inner = [
        [0.284289, 1.685786, 2.196915],
        [0.211587, 0.231738, 2.070361],
        [1.521411, 0.428212, 3.713774],
        [1.581047, 1.620948, 3.817585],
        [0.284289, 1.685786, 2.196915],
    ];
    for (actual, expected) in result[1].iter().zip(expected_inner) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_negative_offset_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours(&contours, -0.25, std::f64::consts::PI / 9.0).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
    let expected = [
        [0.250000, 0.250000, 2.125000],
        [0.250000, 1.750000, 2.125000],
        [1.750000, 1.750000, 3.875000],
        [1.750000, 0.250000, 3.875000],
        [0.250000, 0.250000, 2.125000],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_variable_negative_offset_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![-0.20, -0.30, -0.40, -0.50, -0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
    let expected = [
        [0.211587, 0.231738, 2.070361],
        [0.284289, 1.685786, 2.196915],
        [1.581047, 1.620948, 3.817585],
        [1.521411, 0.428212, 3.713774],
        [0.211587, 0.231738, 2.070361],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_sharp_max_angle_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            corner_type: lines::OffsetContoursCornerType::Sharp,
            max_sharp_angle: std::f64::consts::PI / 6.0,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 17);
    let expected = [
        [0.000000, -0.250000, 0.097225],
        [-0.066987, -0.250000, 0.000000],
        [-0.250000, -0.066987, 0.000000],
        [-0.250000, 0.000000, 0.032408],
        [-0.250000, 2.000000, 1.967592],
        [-0.250000, 2.066988, 2.000000],
        [-0.066987, 2.250000, 2.000000],
        [0.000000, 2.250000, 2.032408],
        [2.000000, 2.250000, 3.967592],
        [2.066988, 2.250000, 4.000000],
        [2.250000, 2.066988, 4.000000],
        [2.250000, 2.000000, 4.032408],
        [2.250000, 0.000000, 5.967592],
        [2.250000, -0.066987, 6.000000],
        [2.066988, -0.250000, 6.000000],
        [2.000000, -0.250000, 5.902775],
        [0.000000, -0.250000, 0.097225],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_mixed_signed_offset_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, -0.10, 0.30, -0.20, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 15);
    let expected = [
        [0.000000, -0.200000, 0.0],
        [-0.079418, -0.204737, 0.0],
        [-0.140350, -0.185030, 0.0],
        [-0.181574, -0.142955, 0.0],
        [-0.201865, -0.080587, 0.0],
        [-0.200000, 0.000000, 0.0],
        [0.087629, 1.917526, 0.0],
        [2.000000, 2.300000, 0.0],
        [2.121161, 2.306700, 0.0],
        [2.216629, 2.276328, 0.0],
        [2.281516, 2.212606, 0.0],
        [2.310935, 2.119256, 0.0],
        [2.300000, 2.000000, 0.0],
        [1.842105, 0.168421, 0.0],
        [0.000000, -0.200000, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_positive_round_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 25);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_lorg_vertices = [
        0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 0,
    ];
    for (origin, expected_vert) in result.origins[0].iter().zip(expected_lorg_vertices) {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        assert!(!origin.is_intersection());
    }
}

#[test]
fn offset_contours_exposes_meshlib_positive_fixed_self_overlap_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0],
        [1.0, 3.0, 0.0],
        [1.0, 1.0, 0.0],
        [3.0, 1.0, 0.0],
        [3.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.20,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 32);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.0, -0.2, 0.0],
        [-0.061803, -0.190211, 0.0],
        [-0.117557, -0.161803, 0.0],
        [-0.161803, -0.117557, 0.0],
        [-0.190211, -0.061803, 0.0],
        [-0.2, 0.0, 0.0],
        [-0.2, 3.0, 0.0],
        [-0.190211, 3.061803, 0.0],
        [-0.161803, 3.117557, 0.0],
        [-0.117557, 3.161803, 0.0],
        [-0.061803, 3.190211, 0.0],
        [0.0, 3.2, 0.0],
        [1.0, 3.2, 0.0],
        [1.061803, 3.190211, 0.0],
        [1.117557, 3.161803, 0.0],
        [1.161803, 3.117557, 0.0],
        [1.190211, 3.061803, 0.0],
        [1.2, 3.0, 0.0],
        [1.2, 1.2, 0.0],
        [3.0, 1.2, 0.0],
        [3.061803, 1.190211, 0.0],
        [3.117557, 1.161803, 0.0],
        [3.161803, 1.117557, 0.0],
        [3.190211, 1.061804, 0.0],
        [3.2, 1.0, 0.0],
        [3.2, 0.0, 0.0],
        [3.190211, -0.061803, 0.0],
        [3.161803, -0.117557, 0.0],
        [3.117557, -0.161803, 0.0],
        [3.061803, -0.190211, 0.0],
        [3.0, -0.2, 0.0],
        [0.0, -0.2, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [
        0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5,
        5, 0,
    ];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        if index == 18 {
            assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 2));
            assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 3));
            assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 4));
            assert!((origin.l_ratio - 0.1).abs() <= 1e-6);
            assert!((origin.u_ratio - 0.1).abs() <= 1e-6);
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_positive_variable_self_overlap_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0],
        [1.0, 3.0, 0.0],
        [1.0, 1.0, 0.0],
        [3.0, 1.0, 0.0],
        [3.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let options = lines::OffsetContoursOptions {
        min_angle_precision: std::f64::consts::PI / 9.0,
        ..lines::OffsetContoursOptions::default()
    };
    let fixed =
        lines::offset_contours_with_options_and_origins(&contours, 0.20, options.clone()).unwrap();
    let offsets = vec![vec![0.20, 0.20, 0.20, 0.20, 0.20, 0.20, 0.20]];

    let variable =
        lines::offset_contours_with_variable_offsets_and_origins(&contours, &offsets, options)
            .unwrap();

    assert_eq!(variable.contours, fixed.contours);
    assert_eq!(variable.origins, fixed.origins);
}

#[test]
fn offset_contours_exposes_meshlib_positive_variable_unequal_self_overlap_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0],
        [1.0, 3.0, 0.0],
        [1.0, 1.0, 0.0],
        [3.0, 1.0, 0.0],
        [3.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.24, 0.18, 0.28, 0.22, 0.26, 0.20]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 32);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.0, -0.2, 0.0],
        [-0.078197, -0.192447, 0.0],
        [-0.134611, -0.1715, 0.0],
        [-0.171927, -0.13433, 0.0],
        [-0.192829, -0.078107, 0.0],
        [-0.2, 0.0, 0.0],
        [-0.24, 3.0, 0.0],
        [-0.233211, 3.095109, 0.0],
        [-0.208304, 3.165338, 0.0],
        [-0.162792, 3.212013, 0.0],
        [-0.094186, 3.236458, 0.0],
        [0.0, 3.24, 0.0],
        [1.0, 3.18, 0.0],
        [1.06982, 3.171119, 0.0],
        [1.119634, 3.151979, 0.0],
        [1.152538, 3.119279, 0.0],
        [1.171628, 3.069719, 0.0],
        [1.18, 3.0, 0.0],
        [1.2664, 1.272008, 0.0],
        [3.0, 1.22, 0.0],
        [3.085579, 1.211048, 0.0],
        [3.146789, 1.187905, 0.0],
        [3.18721, 1.147238, 0.0],
        [3.210421, 1.085714, 0.0],
        [3.22, 1.0, 0.0],
        [3.26, 0.0, 0.0],
        [3.254669, -0.102235, 0.0],
        [3.227996, -0.176816, 0.0],
        [3.177988, -0.22628, 0.0],
        [3.102653, -0.253162, 0.0],
        [3.0, -0.26, 0.0],
        [0.0, -0.2, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let origin = result.origins[0][18];
    assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), (0, 3));
    assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 4));
    assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
    assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 3));
    assert!((origin.l_ratio - 0.1332).abs() <= 1e-6);
    assert!((origin.u_ratio - 0.863996).abs() <= 1e-6);
    assert!(origin.is_intersection());
}

#[test]
fn offset_contours_exposes_meshlib_negative_intersection_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        -0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 5);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        ((0, 0), (0, 3), (0, 1), (0, 0), 0.125, 0.875),
        ((0, 1), (0, 2), (0, 1), (0, 0), 0.125, 0.125),
        ((0, 3), (0, 2), (0, 1), (0, 2), 0.875, 0.875),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.125, 0.875),
        ((0, 0), (0, 3), (0, 1), (0, 0), 0.125, 0.875),
    ];
    for (origin, expected) in result.origins[0].iter().zip(expected) {
        assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), expected.0);
        assert_eq!(
            (origin.l_dest.contour_id, origin.l_dest.vert_id),
            expected.1
        );
        assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), expected.2);
        assert_eq!(
            (origin.u_dest.contour_id, origin.u_dest.vert_id),
            expected.3
        );
        assert!((origin.l_ratio - expected.4).abs() <= 1e-6);
        assert!((origin.u_ratio - expected.5).abs() <= 1e-6);
        assert!(origin.is_intersection());
    }
}

#[test]
fn offset_contours_exposes_meshlib_zero_offset_identity_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 1.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 3.0],
        [2.0, 0.0, 4.0],
        [0.0, 0.0, 1.0],
    ]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.0,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(
        result.contours,
        vec![vec![
            [0.0, 0.0, 2.0],
            [0.0, 2.0, 2.0],
            [2.0, 2.0, 3.0],
            [2.0, 0.0, 3.0],
            [0.0, 0.0, 2.0],
        ]]
    );
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), contours[0].len());
    let expected_lorg_vertices = [0, 1, 2, 3, 0];
    for (origin, expected_vert) in result.origins[0].iter().zip(expected_lorg_vertices) {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        assert!(!origin.is_intersection());
    }
}

#[test]
fn offset_contours_exposes_meshlib_positive_variable_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 25);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_lorg_vertices = [
        0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 0,
    ];
    for (origin, expected_vert) in result.origins[0].iter().zip(expected_lorg_vertices) {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        assert!(!origin.is_intersection());
    }
}

#[test]
fn offset_contours_exposes_meshlib_mixed_signed_variable_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, -0.10, 0.30, -0.20, 0.20]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 15);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.958763, 0.043814, true),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 2), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.084211, 0.921053, true),
        ((0, 0), (-1, -1), (-1, -1), (-1, -1), 0.0, 0.0, false),
    ];
    for (origin, expected) in result.origins[0].iter().zip(expected) {
        assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), expected.0);
        assert_eq!(
            (origin.l_dest.contour_id, origin.l_dest.vert_id),
            expected.1
        );
        assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), expected.2);
        assert_eq!(
            (origin.u_dest.contour_id, origin.u_dest.vert_id),
            expected.3
        );
        assert!((origin.l_ratio - expected.4).abs() <= 1e-6);
        assert!((origin.u_ratio - expected.5).abs() <= 1e-6);
        assert_eq!(origin.is_intersection(), expected.6);
    }
}

#[test]
fn offset_contours_exposes_meshlib_negative_variable_intersection_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![-0.20, -0.30, -0.40, -0.50, -0.20]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 5);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.115869, 0.105793),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.842893, 0.142145),
        ((0, 3), (0, 2), (0, 1), (0, 2), 0.810474, 0.790524),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.214106, 0.760705),
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.115869, 0.105793),
    ];
    for (origin, expected) in result.origins[0].iter().zip(expected) {
        assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), expected.0);
        assert_eq!(
            (origin.l_dest.contour_id, origin.l_dest.vert_id),
            expected.1
        );
        assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), expected.2);
        assert_eq!(
            (origin.u_dest.contour_id, origin.u_dest.vert_id),
            expected.3
        );
        assert!((origin.l_ratio - expected.4).abs() <= 1e-6);
        assert!((origin.u_ratio - expected.5).abs() <= 1e-6);
        assert!(origin.is_intersection());
    }
}

#[test]
fn offset_contours_exposes_meshlib_fixed_shell_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 2);
    assert_eq!(result.contours[0].len(), 25);
    assert_eq!(result.contours[1].len(), 5);
    assert_eq!(result.origins.len(), 2);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    assert_eq!(result.origins[1].len(), result.contours[1].len());
    let expected_inner = [
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.875, 0.125),
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.125, 0.125),
        ((0, 0), (0, 3), (0, 2), (0, 3), 0.875, 0.875),
        ((0, 1), (0, 2), (0, 2), (0, 3), 0.875, 0.125),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.875, 0.125),
    ];
    for (origin, expected) in result.origins[1].iter().zip(expected_inner) {
        assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), expected.0);
        assert_eq!(
            (origin.l_dest.contour_id, origin.l_dest.vert_id),
            expected.1
        );
        assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), expected.2);
        assert_eq!(
            (origin.u_dest.contour_id, origin.u_dest.vert_id),
            expected.3
        );
        assert!((origin.l_ratio - expected.4).abs() <= 1e-6);
        assert!((origin.u_ratio - expected.5).abs() <= 1e-6);
        assert!(origin.is_intersection());
    }
}

#[test]
fn offset_contours_exposes_meshlib_variable_shell_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 2);
    assert_eq!(result.contours[0].len(), 25);
    assert_eq!(result.contours[1].len(), 5);
    assert_eq!(result.origins.len(), 2);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    assert_eq!(result.origins[1].len(), result.contours[1].len());
    let expected_inner = [
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.842893, 0.142145),
        ((0, 0), (0, 1), (0, 0), (0, 3), 0.115869, 0.105793),
        ((0, 3), (0, 2), (0, 0), (0, 3), 0.214106, 0.760705),
        ((0, 3), (0, 2), (0, 1), (0, 2), 0.810474, 0.790524),
        ((0, 0), (0, 1), (0, 1), (0, 2), 0.842893, 0.142145),
    ];
    for (origin, expected) in result.origins[1].iter().zip(expected_inner) {
        assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), expected.0);
        assert_eq!(
            (origin.l_dest.contour_id, origin.l_dest.vert_id),
            expected.1
        );
        assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), expected.2);
        assert_eq!(
            (origin.u_dest.contour_id, origin.u_dest.vert_id),
            expected.3
        );
        assert!((origin.l_ratio - expected.4).abs() <= 1e-6);
        assert!((origin.u_ratio - expected.5).abs() <= 1e-6);
        assert!(origin.is_intersection());
    }
}

#[test]
fn offset_contours_matches_meshlib_variable_sharp_max_angle_3d_z_restore_relaxation_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 2.0],
        [2.0, 2.0, 4.0],
        [2.0, 0.0, 6.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            corner_type: lines::OffsetContoursCornerType::Sharp,
            max_sharp_angle: std::f64::consts::PI / 6.0,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 17);
    let expected = [
        [0.000000, -0.200000, 0.092073],
        [-0.063326, -0.190501, 0.000000],
        [-0.197180, -0.056394, 0.000000],
        [-0.200000, -0.000000, 0.027423],
        [-0.300000, 2.000000, 1.963389],
        [-0.303800, 2.076005, 2.000000],
        [-0.084555, 2.295772, 2.000000],
        [0.000000, 2.300000, 2.040563],
        [2.000000, 2.400000, 3.951774],
        [2.101339, 2.405067, 4.000000],
        [2.394363, 2.112740, 4.000000],
        [2.400000, 2.000000, 4.053362],
        [2.500000, -0.000000, 5.940273],
        [2.506352, -0.127042, 6.000000],
        [2.115096, -0.517264, 6.000000],
        [2.000000, -0.500000, 5.836751],
        [0.000000, -0.200000, 0.092073],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_shell_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 25);
    assert_eq!(result[1].len(), 5);
    let expected_inner = [
        [0.25, 1.75, 0.0],
        [0.25, 0.25, 0.0],
        [1.75, 0.25, 0.0],
        [1.75, 1.75, 0.0],
        [0.25, 1.75, 0.0],
    ];
    for (actual, expected) in result[1].iter().zip(expected_inner) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_shell_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 25);
    assert_eq!(result[1].len(), 5);
    let expected_inner = [
        [0.284289, 1.685786, 0.0],
        [0.211587, 0.231738, 0.0],
        [1.521411, 0.428212, 0.0],
        [1.581047, 1.620948, 0.0],
        [0.284289, 1.685786, 0.0],
    ];
    for (actual, expected) in result[1].iter().zip(expected_inner) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_sharp_shell_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![0.20, 0.30, 0.40, 0.50, 0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            corner_type: lines::OffsetContoursCornerType::Sharp,
            max_sharp_angle: std::f64::consts::PI / 6.0,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 17);
    assert_eq!(result[1].len(), 5);
    let expected_outer = [
        [0.000000, -0.200000, 0.0],
        [-0.063326, -0.190501, 0.0],
        [-0.197180, -0.056394, 0.0],
        [-0.200000, -0.000000, 0.0],
        [-0.300000, 2.000000, 0.0],
        [-0.303800, 2.076005, 0.0],
        [-0.084555, 2.295772, 0.0],
        [0.000000, 2.300000, 0.0],
        [2.000000, 2.400000, 0.0],
        [2.101339, 2.405067, 0.0],
        [2.394363, 2.112740, 0.0],
        [2.400000, 2.000000, 0.0],
        [2.500000, -0.000000, 0.0],
        [2.506352, -0.127042, 0.0],
        [2.115096, -0.517264, 0.0],
        [2.000000, -0.500000, 0.0],
        [0.000000, -0.200000, 0.0],
    ];
    let expected_inner = [
        [0.284289, 1.685786, 0.0],
        [0.211587, 0.231738, 0.0],
        [1.521411, 0.428212, 0.0],
        [1.581047, 1.620948, 0.0],
        [0.284289, 1.685786, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected_outer) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
    for (actual, expected) in result[1].iter().zip(expected_inner) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-5);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_closed_negative_shell_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];

    let result = lines::offset_contours_with_options(
        &contours,
        -0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert!(result.is_empty());
}

#[test]
fn offset_contours_matches_meshlib_closed_variable_negative_shell_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]];
    let offsets = vec![vec![-0.20, -0.30, -0.40, -0.50, -0.20]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Shell,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert!(result.is_empty());
}

#[test]
fn offset_contours_matches_meshlib_open_round_end_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]];

    let result = lines::offset_contours(&contours, 0.25, std::f64::consts::PI / 9.0).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 23);
    let expected = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.077254, 0.237764, 0.0],
        [2.146946, 0.202254, 0.0],
        [2.202254, 0.146946, 0.0],
        [2.237764, 0.077254, 0.0],
        [2.25, 0.0, 0.0],
        [2.237764, -0.077254, 0.0],
        [2.202254, -0.146946, 0.0],
        [2.146946, -0.202254, 0.0],
        [2.077254, -0.237764, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [-0.077254, -0.237764, 0.0],
        [-0.146946, -0.202254, 0.0],
        [-0.202254, -0.146946, 0.0],
        [-0.237764, -0.077254, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.237764, 0.077254, 0.0],
        [-0.202254, 0.146946, 0.0],
        [-0.146946, 0.202254, 0.0],
        [-0.077254, 0.237764, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_round_end_index_map_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_lorg_vertices = [
        0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    for (origin, expected_vert) in result.origins[0].iter().zip(expected_lorg_vertices) {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        assert!(!origin.is_intersection());
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_fixed_round_end_bend_index_map_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Round,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 30);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.75, 1.0, 0.0],
        [0.762236, 1.077254, 0.0],
        [0.797746, 1.146946, 0.0],
        [0.853054, 1.202254, 0.0],
        [0.922746, 1.237764, 0.0],
        [1.0, 1.25, 0.0],
        [1.077254, 1.237764, 0.0],
        [1.146946, 1.202254, 0.0],
        [1.202254, 1.146946, 0.0],
        [1.237764, 1.077254, 0.0],
        [1.25, 1.0, 0.0],
        [1.25, 0.0, 0.0],
        [1.237764, -0.077254, 0.0],
        [1.202254, -0.146946, 0.0],
        [1.146946, -0.202254, 0.0],
        [1.077254, -0.237764, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [-0.077254, -0.237764, 0.0],
        [-0.146946, -0.202254, 0.0],
        [-0.202254, -0.146946, 0.0],
        [-0.237764, -0.077254, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.237764, 0.077254, 0.0],
        [-0.202254, 0.146946, 0.0],
        [-0.146946, 0.202254, 0.0],
        [-0.077254, 0.237764, 0.0],
        [0.0, 0.25, 0.0],
        [0.75, 0.25, 0.0],
        [0.75, 1.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
    ];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        if index == 28 {
            assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 1));
            assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
            assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 1));
            assert!((origin.l_ratio - 0.75).abs() <= 1e-6);
            assert!((origin.u_ratio - 0.75).abs() <= 1e-6);
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_fixed_round_end_zig_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.2, 0.4, 0.0],
        [1.2, 0.8, 0.0],
    ]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Round,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 40);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let penultimate_point = result.contours[0][38];
    assert!((penultimate_point[0] - 0.003986).abs() <= 1e-6);
    assert!((penultimate_point[1] - 0.25).abs() <= 1e-6);

    let origin = result.origins[0][38];
    assert_eq!((origin.l_org.contour_id, origin.l_org.vert_id), (0, 0));
    assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 1));
    assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
    assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 2));
    assert!((origin.l_ratio - 0.003986).abs() <= 1e-6);
    assert!((origin.u_ratio - 0.615854).abs() <= 1e-6);
    assert!(origin.is_intersection());
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
    let expected = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_perpendicular_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[1.0, -1.0, 0.0], [1.0, 1.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 13);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [1.25, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, -0.25, 0.0],
        [1.25, -0.25, 0.0],
        [1.25, -1.0, 0.0],
        [0.75, -1.0, 0.0],
        [0.75, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [0.75, 0.25, 0.0],
        [0.75, 1.0, 0.0],
        [1.25, 1.0, 0.0],
        [1.25, 0.25, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        (
            (1, 0),
            Some((1, 1)),
            Some((0, 0)),
            Some((0, 1)),
            0.625,
            0.625,
        ),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        (
            (1, 0),
            Some((1, 1)),
            Some((0, 0)),
            Some((0, 1)),
            0.375,
            0.625,
        ),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        (
            (0, 0),
            Some((0, 1)),
            Some((1, 1)),
            Some((1, 0)),
            0.375,
            0.625,
        ),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        (
            (0, 0),
            Some((0, 1)),
            Some((1, 1)),
            Some((1, 0)),
            0.375,
            0.375,
        ),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        (
            (1, 0),
            Some((1, 1)),
            Some((0, 0)),
            Some((0, 1)),
            0.625,
            0.625,
        ),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[1.0, 0.1, 0.0], [3.0, 0.1, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [1.0, 0.25, 0.0],
        [1.0, 0.35, 0.0],
        [3.0, 0.35, 0.0],
        [3.0, -0.15, 0.0],
        [2.0, -0.15, 0.0],
        [2.0, -0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_overlapping_parallel_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[1.0, 0.1, 0.0], [3.0, 0.1, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [1.0, 0.25, 0.0],
        [1.0, 0.35, 0.0],
        [3.0, 0.35, 0.0],
        [3.0, -0.15, 0.0],
        [2.0, -0.15, 0.0],
        [2.0, -0.25, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 0)), Some((0, 1)), 0.8, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((0, 1), Some((0, 1)), Some((1, 0)), Some((1, 1)), 0.2, 0.5),
        ((0, 1), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_touching_horizontal_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [4.0, 0.25, 0.0],
        [4.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_touching_horizontal_direction_variants_global_outline_index_map_contract(
) {
    let assert_case = |contours: Vec<Vec<[f64; 3]>>,
                       expected_points: [[f64; 3]; 9],
                       expected_origins: [(
        (i32, i32),
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        f64,
        f64,
    ); 9]| {
        let result = lines::offset_contours_with_options_and_origins(
            &contours,
            0.25,
            lines::OffsetContoursOptions {
                mode: lines::OffsetContoursMode::Offset,
                end_type: lines::OffsetContoursEndType::Cut,
                min_angle_precision: std::f64::consts::PI / 9.0,
                ..lines::OffsetContoursOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.contours.len(), 1);
        assert_eq!(result.origins.len(), 1);
        assert_eq!(result.contours[0].len(), 9);
        assert_eq!(result.origins[0].len(), result.contours[0].len());
        for (actual, expected) in result.contours[0].iter().zip(expected_points) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
            }
        }

        for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
            result.origins[0].iter().zip(expected_origins)
        {
            assert_eq!(
                (origin.l_org.contour_id, origin.l_org.vert_id),
                (l_org.0, l_org.1)
            );
            if let Some((contour_id, vert_id)) = l_dest {
                assert_eq!(
                    (origin.l_dest.contour_id, origin.l_dest.vert_id),
                    (contour_id, vert_id)
                );
                assert!(origin.is_intersection());
            } else {
                assert!(!origin.is_intersection());
            }
            if let Some((contour_id, vert_id)) = u_org {
                assert_eq!(
                    (origin.u_org.contour_id, origin.u_org.vert_id),
                    (contour_id, vert_id)
                );
            }
            if let Some((contour_id, vert_id)) = u_dest {
                assert_eq!(
                    (origin.u_dest.contour_id, origin.u_dest.vert_id),
                    (contour_id, vert_id)
                );
            }
            assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
            assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
        }
    };

    let forward_points = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [4.0, 0.25, 0.0],
        [4.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    let first_reversed_points = [
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [4.0, 0.25, 0.0],
        [4.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
    ];

    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
        ],
        forward_points,
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 1)), Some((1, 0)), Some((1, 1)), 1.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 0)), Some((0, 0)), Some((0, 1)), 0.0, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            vec![[4.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        ],
        forward_points,
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 1)), Some((1, 1)), Some((1, 0)), 1.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 1.0, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
        ],
        first_reversed_points,
        [
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 1)), Some((0, 0)), Some((0, 0)), 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 0)), Some((0, 1)), Some((0, 0)), 0.0, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[4.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        ],
        first_reversed_points,
        [
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 0)), Some((0, 0)), Some((0, 0)), 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 1)), Some((1, 1)), 1.0, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
        ],
    );
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_contract()
{
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0.0, 2.0, 0.0], [0.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 4.0, 0.0],
        [0.25, 4.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_touching_vertical_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0.0, 2.0, 0.0], [0.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 4.0, 0.0],
        [0.25, 4.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 1)), Some((0, 0)), 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 1)), Some((0, 1)), 0.0, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_vertical_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0.0, 4.0, 0.0], [0.0, 2.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 4.0, 0.0],
        [0.25, 4.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 1), Some((1, 1)), Some((0, 1)), Some((0, 0)), 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), Some((1, 0)), Some((0, 1)), Some((0, 1)), 0.0, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_vertical_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
        vec![[0.0, 2.0, 0.0], [0.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 4.0, 0.0],
        [0.25, 4.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 0)), Some((0, 1)), 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 0)), Some((0, 0)), 0.0, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_contract()
{
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [2.0, 2.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [1.823223, 2.176777, 0.0],
        [2.176777, 1.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_touching_diagonal_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [2.0, 2.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [1.823223, 2.176777, 0.0],
        [2.176777, 1.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 1)), Some((0, 1)), 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 1.0, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_touching_diagonal_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        vec![[2.0, 2.0, 0.0], [1.0, 1.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [1.823223, 2.176777, 0.0],
        [2.176777, 1.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((1, 1), Some((1, 0)), Some((0, 1)), Some((0, 1)), 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 1.0, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_touching_diagonal_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[1.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [2.0, 2.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [1.823223, 2.176777, 0.0],
        [2.176777, 1.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 0)), Some((0, 0)), 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 1), Some((0, 0)), Some((1, 0)), Some((1, 0)), 1.0, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_contract(
) {
    let shift = std::f64::consts::FRAC_1_SQRT_2 * 0.1;
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
        vec![
            [1.0 - shift, 1.0 + shift, 0.0],
            [3.0 - shift, 3.0 + shift, 0.0],
        ],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [2.106066, 1.893934, 0.0],
        [2.176777, 1.823223, 0.0],
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.752513, 1.247487, 0.0],
        [2.752513, 3.247487, 0.0],
        [3.106066, 2.893934, 0.0],
        [2.106066, 1.893934, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_rotated_shifted_parallel_segments_global_outline_index_map_contract(
) {
    let shift = std::f64::consts::FRAC_1_SQRT_2 * 0.1;
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
        vec![
            [1.0 - shift, 1.0 + shift, 0.0],
            [3.0 - shift, 3.0 + shift, 0.0],
        ],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [2.106066, 1.893934, 0.0],
        [2.176777, 1.823223, 0.0],
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.752513, 1.247487, 0.0],
        [2.752513, 3.247487, 0.0],
        [3.106066, 2.893934, 0.0],
        [2.106066, 1.893934, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((1, 0), Some((1, 1)), Some((0, 1)), Some((0, 1)), 0.5, 0.8),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 0.2),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 1)), Some((0, 1)), 0.5, 0.8),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [2.823223, 3.176777, 0.0],
        [3.176777, 2.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [2.823223, 3.176777, 0.0],
        [3.176777, 2.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
        vec![[2.0, 2.0, 0.0], [4.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 11);
    let expected = [
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [2.023223, 2.376777, 0.0],
        [3.823223, 4.176777, 0.0],
        [4.176777, 3.823223, 0.0],
        [2.376777, 2.023223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
        vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
        vec![[2.0, 2.0, 0.0], [4.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 11);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.176777, -0.176777, 0.0],
        [-0.176777, 0.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [0.823223, 1.176777, 0.0],
        [2.023223, 2.376777, 0.0],
        [3.823223, 4.176777, 0.0],
        [4.176777, 3.823223, 0.0],
        [2.376777, 2.023223, 0.0],
        [1.176777, 0.823223, 0.0],
        [1.176777, 0.823223, 0.0],
        [0.176777, -0.176777, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((2, 0), Some((2, 1)), Some((1, 0)), Some((1, 1)), 0.1, 0.6),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((2, 0)), Some((2, 1)), 0.6, 0.1),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_three_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract(
) {
    let assert_case = |contours: Vec<Vec<[f64; 3]>>,
                       expected_origins: [(
        (i32, i32),
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        f64,
        f64,
    ); 11]| {
        let result = lines::offset_contours_with_options_and_origins(
            &contours,
            0.25,
            lines::OffsetContoursOptions {
                mode: lines::OffsetContoursMode::Offset,
                end_type: lines::OffsetContoursEndType::Cut,
                min_angle_precision: std::f64::consts::PI / 9.0,
                ..lines::OffsetContoursOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.contours.len(), 1);
        assert_eq!(result.origins.len(), 1);
        assert_eq!(result.contours[0].len(), 11);
        assert_eq!(result.origins[0].len(), result.contours[0].len());
        let expected_points = [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.023223, 2.376777, 0.0],
            [3.823223, 4.176777, 0.0],
            [4.176777, 3.823223, 0.0],
            [2.376777, 2.023223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ];
        for (actual, expected) in result.contours[0].iter().zip(expected_points) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
            }
        }

        for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
            result.origins[0].iter().zip(expected_origins)
        {
            assert_eq!(
                (origin.l_org.contour_id, origin.l_org.vert_id),
                (l_org.0, l_org.1)
            );
            if let Some((contour_id, vert_id)) = l_dest {
                assert_eq!(
                    (origin.l_dest.contour_id, origin.l_dest.vert_id),
                    (contour_id, vert_id)
                );
                assert!(origin.is_intersection());
            } else {
                assert!(!origin.is_intersection());
            }
            if let Some((contour_id, vert_id)) = u_org {
                assert_eq!(
                    (origin.u_org.contour_id, origin.u_org.vert_id),
                    (contour_id, vert_id)
                );
            }
            if let Some((contour_id, vert_id)) = u_dest {
                assert_eq!(
                    (origin.u_dest.contour_id, origin.u_dest.vert_id),
                    (contour_id, vert_id)
                );
            }
            assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
            assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
        }
    };

    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
            vec![[3.0, 3.0, 0.0], [1.0, 1.0, 0.0]],
            vec![[2.0, 2.0, 0.0], [4.0, 4.0, 0.0]],
        ],
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 0.5, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((2, 0), Some((2, 1)), Some((1, 1)), Some((1, 0)), 0.1, 0.6),
            ((2, 1), None, None, None, 0.0, 0.0),
            ((2, 1), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 0)), Some((2, 0)), Some((2, 1)), 0.6, 0.1),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 0.5, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[2.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
            vec![[2.0, 2.0, 0.0], [4.0, 4.0, 0.0]],
        ],
        [
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 0)), Some((1, 0)), 0.5, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((2, 0), Some((2, 1)), Some((1, 0)), Some((1, 1)), 0.1, 0.6),
            ((2, 1), None, None, None, 0.0, 0.0),
            ((2, 1), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 1)), Some((2, 0)), Some((2, 1)), 0.6, 0.1),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 0)), Some((1, 0)), 0.5, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
            vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
            vec![[4.0, 4.0, 0.0], [2.0, 2.0, 0.0]],
        ],
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((2, 1), Some((2, 0)), Some((1, 0)), Some((1, 1)), 0.1, 0.6),
            ((2, 0), None, None, None, 0.0, 0.0),
            ((2, 0), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 1)), Some((2, 1)), Some((2, 0)), 0.6, 0.1),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 0)), Some((1, 0)), 0.5, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[2.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[3.0, 3.0, 0.0], [1.0, 1.0, 0.0]],
            vec![[4.0, 4.0, 0.0], [2.0, 2.0, 0.0]],
        ],
        [
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 1)), Some((1, 1)), 0.5, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((2, 1), Some((2, 0)), Some((1, 1)), Some((1, 0)), 0.1, 0.6),
            ((2, 0), None, None, None, 0.0, 0.0),
            ((2, 0), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 0)), Some((2, 1)), Some((2, 0)), 0.6, 0.1),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 1)), Some((1, 1)), 0.5, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
        ],
    );
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_diagonal_collinear_overlapping_direction_variants_global_outline_index_map_contract(
) {
    let assert_case = |contours: Vec<Vec<[f64; 3]>>,
                       expected_origins: [(
        (i32, i32),
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        f64,
        f64,
    ); 9]| {
        let result = lines::offset_contours_with_options_and_origins(
            &contours,
            0.25,
            lines::OffsetContoursOptions {
                mode: lines::OffsetContoursMode::Offset,
                end_type: lines::OffsetContoursEndType::Cut,
                min_angle_precision: std::f64::consts::PI / 9.0,
                ..lines::OffsetContoursOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.contours.len(), 1);
        assert_eq!(result.origins.len(), 1);
        assert_eq!(result.contours[0].len(), 9);
        assert_eq!(result.origins[0].len(), result.contours[0].len());
        let expected_points = [
            [0.176777, -0.176777, 0.0],
            [-0.176777, 0.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [0.823223, 1.176777, 0.0],
            [2.823223, 3.176777, 0.0],
            [3.176777, 2.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [1.176777, 0.823223, 0.0],
            [0.176777, -0.176777, 0.0],
        ];
        for (actual, expected) in result.contours[0].iter().zip(expected_points) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
            }
        }

        for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
            result.origins[0].iter().zip(expected_origins)
        {
            assert_eq!(
                (origin.l_org.contour_id, origin.l_org.vert_id),
                (l_org.0, l_org.1)
            );
            if let Some((contour_id, vert_id)) = l_dest {
                assert_eq!(
                    (origin.l_dest.contour_id, origin.l_dest.vert_id),
                    (contour_id, vert_id)
                );
                assert!(origin.is_intersection());
            } else {
                assert!(!origin.is_intersection());
            }
            if let Some((contour_id, vert_id)) = u_org {
                assert_eq!(
                    (origin.u_org.contour_id, origin.u_org.vert_id),
                    (contour_id, vert_id)
                );
            }
            if let Some((contour_id, vert_id)) = u_dest {
                assert_eq!(
                    (origin.u_dest.contour_id, origin.u_dest.vert_id),
                    (contour_id, vert_id)
                );
            }
            assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
            assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
        }
    };

    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [2.0, 2.0, 0.0]],
            vec![[3.0, 3.0, 0.0], [1.0, 1.0, 0.0]],
        ],
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 0.5, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 0.5, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[2.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[1.0, 1.0, 0.0], [3.0, 3.0, 0.0]],
        ],
        [
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 0)), Some((1, 0)), 0.5, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 0)), Some((1, 0)), 0.5, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[2.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[3.0, 3.0, 0.0], [1.0, 1.0, 0.0]],
        ],
        [
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 1)), Some((1, 1)), 0.5, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((0, 1), Some((0, 0)), Some((1, 1)), Some((1, 1)), 0.5, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
        ],
    );
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [3.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [3.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), Some((0, 1)), Some((1, 0)), Some((1, 1)), 1.0, 0.5),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 0)), Some((0, 1)), 0.0, 0.5),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_three_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
        vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 13);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [4.0, 0.25, 0.0],
        [4.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [2.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), Some((0, 1)), Some((1, 0)), Some((1, 1)), 1.0, 0.5),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), Some((1, 1)), Some((2, 0)), Some((2, 1)), 1.0, 0.5),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 0), None, None, None, 0.0, 0.0),
        ((2, 0), Some((2, 0)), Some((1, 0)), Some((1, 1)), 0.0, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 0)), Some((0, 1)), 0.0, 0.5),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0.0, 1.0, 0.0], [0.0, 3.0, 0.0]],
        vec![[0.0, 2.0, 0.0], [0.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 13);
    let expected = [
        [-0.25, 0.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 4.0, 0.0],
        [0.25, 4.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_three_vertical_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0.0, 1.0, 0.0], [0.0, 3.0, 0.0]],
        vec![[0.0, 2.0, 0.0], [0.0, 4.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 13);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [-0.25, 0.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 2.0, 0.0],
        [-0.25, 4.0, 0.0],
        [0.25, 4.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 1)), Some((0, 0)), 0.0, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((2, 0), Some((2, 0)), Some((1, 1)), Some((1, 0)), 0.0, 0.5),
        ((2, 0), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 1), None, None, None, 0.0, 0.0),
        ((2, 0), Some((2, 1)), Some((1, 1)), Some((1, 1)), 0.5, 1.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 1)), Some((0, 1)), 0.5, 1.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_reversed_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[3.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [3.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), Some((0, 1)), Some((1, 1)), Some((1, 0)), 1.0, 0.5),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((0, 0), Some((0, 1)), Some((1, 1)), Some((1, 1)), 0.5, 1.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_first_reversed_collinear_overlapping_segments_global_outline_index_map_contract(
) {
    let contours = vec![
        vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        vec![[1.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.contours[0].len(), 9);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected_points = [
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [3.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected_points) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }

    let expected_origins = [
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 1), None, None, None, 0.0, 0.0),
        ((0, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 1)), Some((0, 0)), Some((0, 0)), 0.5, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 1), None, None, None, 0.0, 0.0),
        ((1, 0), None, None, None, 0.0, 0.0),
        ((1, 0), Some((1, 0)), Some((0, 1)), Some((0, 0)), 0.0, 0.5),
        ((0, 1), None, None, None, 0.0, 0.0),
    ];
    for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
        result.origins[0].iter().zip(expected_origins)
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (l_org.0, l_org.1)
        );
        if let Some((contour_id, vert_id)) = l_dest {
            assert_eq!(
                (origin.l_dest.contour_id, origin.l_dest.vert_id),
                (contour_id, vert_id)
            );
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
        if let Some((contour_id, vert_id)) = u_org {
            assert_eq!(
                (origin.u_org.contour_id, origin.u_org.vert_id),
                (contour_id, vert_id)
            );
        }
        if let Some((contour_id, vert_id)) = u_dest {
            assert_eq!(
                (origin.u_dest.contour_id, origin.u_dest.vert_id),
                (contour_id, vert_id)
            );
        }
        assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
        assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_both_reversed_collinear_overlapping_segments_global_outline_contract(
) {
    let contours = vec![
        vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        vec![[3.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
    ];

    let result = lines::offset_contours_with_options(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            mode: lines::OffsetContoursMode::Offset,
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
    let expected = [
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [2.0, 0.25, 0.0],
        [3.0, 0.25, 0.0],
        [3.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_contract(
) {
    let assert_case = |contours: Vec<Vec<[f64; 3]>>, expected_points: [[f64; 3]; 9]| {
        let result = lines::offset_contours_with_options(
            &contours,
            0.25,
            lines::OffsetContoursOptions {
                mode: lines::OffsetContoursMode::Offset,
                end_type: lines::OffsetContoursEndType::Cut,
                min_angle_precision: std::f64::consts::PI / 9.0,
                ..lines::OffsetContoursOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 9);
        for (actual, expected) in result[0].iter().zip(expected_points) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
            }
        }
    };

    let forward_points = [
        [-0.25, 0.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 3.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    let first_reversed_points = [
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 3.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
    ];

    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            vec![[0.0, 1.0, 0.0], [0.0, 3.0, 0.0]],
        ],
        forward_points,
    );
    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            vec![[0.0, 3.0, 0.0], [0.0, 1.0, 0.0]],
        ],
        forward_points,
    );
    assert_case(
        vec![
            vec![[0.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[0.0, 1.0, 0.0], [0.0, 3.0, 0.0]],
        ],
        first_reversed_points,
    );
    assert_case(
        vec![
            vec![[0.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[0.0, 3.0, 0.0], [0.0, 1.0, 0.0]],
        ],
        first_reversed_points,
    );
}

#[test]
fn offset_contours_with_origins_matches_meshlib_open_cut_end_vertical_collinear_overlapping_direction_variants_global_outline_index_map_contract(
) {
    let assert_case = |contours: Vec<Vec<[f64; 3]>>,
                       expected_points: [[f64; 3]; 9],
                       expected_origins: [(
        (i32, i32),
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        Option<(i32, i32)>,
        f64,
        f64,
    ); 9]| {
        let result = lines::offset_contours_with_options_and_origins(
            &contours,
            0.25,
            lines::OffsetContoursOptions {
                mode: lines::OffsetContoursMode::Offset,
                end_type: lines::OffsetContoursEndType::Cut,
                min_angle_precision: std::f64::consts::PI / 9.0,
                ..lines::OffsetContoursOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.contours.len(), 1);
        assert_eq!(result.origins.len(), 1);
        assert_eq!(result.contours[0].len(), 9);
        assert_eq!(result.origins[0].len(), result.contours[0].len());
        for (actual, expected) in result.contours[0].iter().zip(expected_points) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
            }
        }

        for (origin, (l_org, l_dest, u_org, u_dest, l_ratio, u_ratio)) in
            result.origins[0].iter().zip(expected_origins)
        {
            assert_eq!(
                (origin.l_org.contour_id, origin.l_org.vert_id),
                (l_org.0, l_org.1)
            );
            if let Some((contour_id, vert_id)) = l_dest {
                assert_eq!(
                    (origin.l_dest.contour_id, origin.l_dest.vert_id),
                    (contour_id, vert_id)
                );
                assert!(origin.is_intersection());
            } else {
                assert!(!origin.is_intersection());
            }
            if let Some((contour_id, vert_id)) = u_org {
                assert_eq!(
                    (origin.u_org.contour_id, origin.u_org.vert_id),
                    (contour_id, vert_id)
                );
            }
            if let Some((contour_id, vert_id)) = u_dest {
                assert_eq!(
                    (origin.u_dest.contour_id, origin.u_dest.vert_id),
                    (contour_id, vert_id)
                );
            }
            assert!((origin.l_ratio - l_ratio).abs() <= 1e-6);
            assert!((origin.u_ratio - u_ratio).abs() <= 1e-6);
        }
    };

    let forward_points = [
        [-0.25, 0.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 3.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
    ];
    let first_reversed_points = [
        [0.25, 2.0, 0.0],
        [0.25, 0.0, 0.0],
        [-0.25, 0.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 1.0, 0.0],
        [-0.25, 3.0, 0.0],
        [0.25, 3.0, 0.0],
        [0.25, 2.0, 0.0],
        [0.25, 2.0, 0.0],
    ];

    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            vec![[0.0, 1.0, 0.0], [0.0, 3.0, 0.0]],
        ],
        forward_points,
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 0)), Some((0, 1)), Some((0, 0)), 0.0, 0.5),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 1)), Some((0, 1)), Some((0, 1)), 0.5, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[0.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            vec![[0.0, 3.0, 0.0], [0.0, 1.0, 0.0]],
        ],
        forward_points,
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 1)), Some((0, 1)), Some((0, 0)), 0.0, 0.5),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 0)), Some((0, 1)), Some((0, 1)), 0.5, 1.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[0.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[0.0, 1.0, 0.0], [0.0, 3.0, 0.0]],
        ],
        first_reversed_points,
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 0)), Some((0, 0)), Some((0, 1)), 0.0, 0.5),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), Some((1, 1)), Some((0, 0)), Some((0, 0)), 0.5, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
    assert_case(
        vec![
            vec![[0.0, 2.0, 0.0], [0.0, 0.0, 0.0]],
            vec![[0.0, 3.0, 0.0], [0.0, 1.0, 0.0]],
        ],
        first_reversed_points,
        [
            ((0, 0), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((0, 1), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 1)), Some((0, 0)), Some((0, 1)), 0.0, 0.5),
            ((1, 1), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 0), None, None, None, 0.0, 0.0),
            ((1, 1), Some((1, 0)), Some((0, 0)), Some((0, 0)), 0.5, 1.0),
            ((0, 0), None, None, None, 0.0, 0.0),
        ],
    );
}

#[test]
fn offset_contours_matches_meshlib_open_variable_cut_end_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]];
    let offsets = vec![vec![0.25, 0.5]];

    let result = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 5);
    let expected = [
        [0.0, 0.25, 0.0],
        [2.0, 0.5, 0.0],
        [2.0, -0.5, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
    ];
    for (actual, expected) in result[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_open_fixed_cut_end_bend_index_map_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];

    let result = lines::offset_contours_with_options_and_origins(
        &contours,
        0.25,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 12);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.75, 1.0, 0.0],
        [1.25, 1.0, 0.0],
        [1.25, 0.0, 0.0],
        [1.237764, -0.077254, 0.0],
        [1.202254, -0.146946, 0.0],
        [1.146946, -0.202254, 0.0],
        [1.077254, -0.237764, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.25, 0.0],
        [0.0, 0.25, 0.0],
        [0.75, 0.25, 0.0],
        [0.75, 1.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 2];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (0, expected_vert)
        );
        if index == 10 {
            assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 1));
            assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
            assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 1));
            assert!((origin.l_ratio - 0.75).abs() <= 1e-6);
            assert!((origin.u_ratio - 0.75).abs() <= 1e-6);
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_open_variable_cut_end_bend_index_map_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];
    let offsets = vec![vec![0.18, 0.25, 0.32]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 12);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.68, 1.0, 0.0],
        [1.32, 1.0, 0.0],
        [1.25, 0.0, 0.0],
        [1.236928, -0.099081, 0.0],
        [1.210212, -0.172573, 0.0],
        [1.165032, -0.221524, 0.0],
        [1.096567, -0.246984, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.18, 0.0],
        [0.0, 0.18, 0.0],
        [0.733804, 0.231366, 0.0],
        [0.68, 1.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 2];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(
            (origin.l_org.contour_id, origin.l_org.vert_id),
            (0, expected_vert)
        );
        if index == 10 {
            assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 1));
            assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
            assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 1));
            assert!((origin.l_ratio - 0.733804).abs() <= 1e-6);
            assert!((origin.u_ratio - 0.768634).abs() <= 1e-6);
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_open_variable_round_end_self_overlap_index_map_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];
    let offsets = vec![vec![0.25, 0.40, 0.20]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Round,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 30);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.670103, 0.350515, 0.0],
        [0.8, 1.0, 0.0],
        [0.823908, 1.079427, 0.0],
        [0.858545, 1.141204, 0.0],
        [0.901226, 1.18533, 0.0],
        [0.949272, 1.211805, 0.0],
        [1.0, 1.220631, 0.0],
        [1.050728, 1.211805, 0.0],
        [1.098774, 1.18533, 0.0],
        [1.141456, 1.141204, 0.0],
        [1.176092, 1.079427, 0.0],
        [1.2, 1.0, 0.0],
        [1.4, 0.0, 0.0],
        [1.409474, -0.158835, 0.0],
        [1.370061, -0.2807, 0.0],
        [1.285911, -0.363147, 0.0],
        [1.161174, -0.40373, 0.0],
        [1.0, -0.4, 0.0],
        [0.0, -0.25, 0.0],
        [-0.10013, -0.223984, 0.0],
        [-0.178009, -0.181979, 0.0],
        [-0.233636, -0.127982, 0.0],
        [-0.267013, -0.06599, 0.0],
        [-0.278138, 0.0, 0.0],
        [-0.267013, 0.06599, 0.0],
        [-0.233636, 0.127982, 0.0],
        [-0.178009, 0.181979, 0.0],
        [-0.10013, 0.223984, 0.0],
        [0.0, 0.25, 0.0],
        [0.670103, 0.350515, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [
        1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        if index == 0 || index + 1 == result.origins[0].len() {
            assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 2));
            assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 0));
            assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 1));
            assert!((origin.l_ratio - 0.350516).abs() <= 1e-6);
            assert!((origin.u_ratio - 0.670103).abs() <= 1e-6);
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_open_variable_increasing_round_end_index_map_contract() {
    let contours = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];
    let offsets = vec![vec![0.18, 0.25, 0.32]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Round,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 30);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [0.68, 1.0, 0.0],
        [0.69068, 1.129284, 0.0],
        [0.736907, 1.229838, 0.0],
        [0.809793, 1.301662, 0.0],
        [0.900453, 1.344756, 0.0],
        [1.0, 1.359121, 0.0],
        [1.099547, 1.344756, 0.0],
        [1.190207, 1.301662, 0.0],
        [1.263093, 1.229838, 0.0],
        [1.30932, 1.129284, 0.0],
        [1.32, 1.0, 0.0],
        [1.25, 0.0, 0.0],
        [1.236928, -0.099081, 0.0],
        [1.210212, -0.172573, 0.0],
        [1.165032, -0.221524, 0.0],
        [1.096567, -0.246984, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.18, 0.0],
        [-0.072722, -0.165848, 0.0],
        [-0.129284, -0.13713, 0.0],
        [-0.169685, -0.097489, 0.0],
        [-0.193925, -0.050565, 0.0],
        [-0.202006, 0.0, 0.0],
        [-0.193925, 0.050565, 0.0],
        [-0.169685, 0.097489, 0.0],
        [-0.129284, 0.13713, 0.0],
        [-0.072722, 0.165848, 0.0],
        [0.0, 0.18, 0.0],
        [0.733804, 0.231366, 0.0],
        [0.68, 1.0, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
    ];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        if index == 28 {
            assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 1));
            assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
            assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 1));
            assert!((origin.l_ratio - 0.733804).abs() <= 1e-6);
            assert!((origin.u_ratio - 0.768634).abs() <= 1e-6);
            assert!(origin.is_intersection());
        } else {
            assert!(!origin.is_intersection());
        }
    }
}

#[test]
fn offset_contours_exposes_meshlib_open_variable_zig_round_end_index_map_contract() {
    let contours = vec![vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.2, 0.4, 0.0],
        [1.2, 0.8, 0.0],
    ]];
    let offsets = vec![vec![0.18, 0.25, 0.32, 0.18]];

    let result = lines::offset_contours_with_variable_offsets_and_origins(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Round,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].len(), 40);
    assert_eq!(result.origins.len(), 1);
    assert_eq!(result.origins[0].len(), result.contours[0].len());
    let expected = [
        [-0.06202, 0.183081, 0.0],
        [-0.140014, 0.271892, 0.0],
        [-0.177165, 0.371316, 0.0],
        [-0.173549, 0.47245, 0.0],
        [-0.12924, 0.566395, 0.0],
        [-0.044314, 0.644249, 0.0],
        [0.081155, 0.697113, 0.0],
        [1.13315, 0.967126, 0.0],
        [1.206807, 0.977635, 0.0],
        [1.270104, 0.970788, 0.0],
        [1.321903, 0.949431, 0.0],
        [1.361064, 0.916412, 0.0],
        [1.386448, 0.874579, 0.0],
        [1.396917, 0.82678, 0.0],
        [1.39133, 0.775862, 0.0],
        [1.36855, 0.724674, 0.0],
        [1.327436, 0.676062, 0.0],
        [1.26685, 0.632874, 0.0],
        [0.833918, 0.390841, 0.0],
        [1.111803, 0.223607, 0.0],
        [1.198713, 0.155018, 0.0],
        [1.254721, 0.076931, 0.0],
        [1.280866, -0.004564, 0.0],
        [1.278187, -0.083377, 0.0],
        [1.247722, -0.153417, 0.0],
        [1.19051, -0.208594, 0.0],
        [1.10759, -0.242819, 0.0],
        [1.0, -0.25, 0.0],
        [0.0, -0.18, 0.0],
        [-0.072722, -0.165848, 0.0],
        [-0.129284, -0.13713, 0.0],
        [-0.169685, -0.097489, 0.0],
        [-0.193925, -0.050565, 0.0],
        [-0.202006, 0.0, 0.0],
        [-0.193925, 0.050565, 0.0],
        [-0.169685, 0.097489, 0.0],
        [-0.129284, 0.13713, 0.0],
        [-0.072722, 0.165848, 0.0],
        [-0.04253, 0.171723, 0.0],
        [-0.06202, 0.183081, 0.0],
    ];
    for (actual, expected) in result.contours[0].iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() <= 1e-6);
        }
    }
    let expected_lorg_vertices = [
        2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
    ];
    for (index, (origin, expected_vert)) in result.origins[0]
        .iter()
        .zip(expected_lorg_vertices)
        .enumerate()
    {
        assert_eq!(origin.l_org.contour_id, 0);
        assert_eq!(origin.l_org.vert_id, expected_vert);
        match index {
            18 => {
                assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 3));
                assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
                assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 1));
                assert!((origin.l_ratio - 0.543323).abs() <= 1e-6);
                assert!((origin.u_ratio - 0.638497).abs() <= 1e-6);
                assert!(origin.is_intersection());
            }
            38 => {
                assert_eq!((origin.l_dest.contour_id, origin.l_dest.vert_id), (0, 0));
                assert_eq!((origin.u_org.contour_id, origin.u_org.vert_id), (0, 2));
                assert_eq!((origin.u_dest.contour_id, origin.u_dest.vert_id), (0, 2));
                assert!((origin.l_ratio - 0.415170).abs() <= 1e-6);
                assert!((origin.u_ratio - 0.163901).abs() <= 1e-6);
                assert!(origin.is_intersection());
            }
            _ => assert!(!origin.is_intersection()),
        }
    }
}

#[test]
fn offset_contours_variable_rejects_single_point_contours() {
    let contours = vec![vec![[0.0, 0.0, 0.0]]];
    let offsets = vec![vec![0.25]];

    let err = lines::offset_contours_with_variable_offsets(
        &contours,
        &offsets,
        lines::OffsetContoursOptions {
            end_type: lines::OffsetContoursEndType::Cut,
            min_angle_precision: std::f64::consts::PI / 9.0,
            ..lines::OffsetContoursOptions::default()
        },
    )
    .unwrap_err();

    assert!(err.contains("requires closed contours"));
}

#[test]
fn object_lines_pts_roundtrips_meshlib_polyline_blocks() {
    let object = lines::object_lines_from_contours(
        &[
            vec![
                [0.0, 0.0, 0.0],
                [1.25, 0.0, 0.0],
                [1.25, 1.5, 0.0],
                [0.0, 0.0, 0.0],
            ],
            vec![[2.0, -1.0, 0.5], [3.0, -1.0, 0.5]],
        ],
        lines::ObjectLinesOptions::default(),
    )
    .unwrap();

    let source = lines::object_lines_to_pts(&object).unwrap();

    assert_eq!(
        source,
        concat!(
            "BEGIN_Polyline\n",
            "0 0 0\n",
            "1.25 0 0\n",
            "1.25 1.5 0\n",
            "0 0 0\n",
            "END_Polyline\n",
            "BEGIN_Polyline\n",
            "2 -1 0.5\n",
            "3 -1 0.5\n",
            "END_Polyline\n",
        )
    );
    let reparsed = lines::object_lines_from_pts(&source).unwrap();
    assert_eq!(
        lines::object_lines_to_contours(&reparsed).unwrap(),
        lines::object_lines_to_contours(&object).unwrap()
    );
}

#[test]
fn object_lines_pts_import_accepts_meshlib_trailing_point_fields() {
    let source = concat!(
        "BEGIN_Polyline\n",
        "0 0 0 0.75 255 128 64\n",
        "1.25 0 0 0.5 12 34 56\n",
        "1.25 1.5 0 ignored trailing tokens\n",
        "END_Polyline\n",
    );

    let object = lines::object_lines_from_pts(source).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.25, 0.0, 0.0], [1.25, 1.5, 0.0],]]
    );
}

#[test]
fn object_lines_pts_import_accepts_meshlib_last_coordinate_prefix_suffix() {
    let source = concat!(
        "BEGIN_Polyline\n",
        "0 0 3.5mm\n",
        "1 2 1e+2suffix trailing tokens\n",
        "END_Polyline\n",
    );

    let object = lines::object_lines_from_pts(source).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 3.5], [1.0, 2.0, 100.0],]]
    );
}

#[test]
fn object_lines_pts_import_rejects_meshlib_nonlast_coordinate_suffixes() {
    let first_coordinate_suffix = concat!("BEGIN_Polyline\n", "1x 2 3\n", "END_Polyline\n");
    let second_coordinate_suffix = concat!("BEGIN_Polyline\n", "1 2y 3\n", "END_Polyline\n");

    assert!(lines::object_lines_from_pts(first_coordinate_suffix).is_err());
    assert!(lines::object_lines_from_pts(second_coordinate_suffix).is_err());
}

#[test]
fn object_lines_svg_import_matches_meshlib_line_polyline_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<line x1="1" y1="2" x2="4" y2="6" />"#,
        r#"<polyline points="0,0 2,0 2,2" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![
            vec![[1.0, -2.0, 0.0], [4.0, -6.0, 0.0]],
            vec![[0.0, -0.0, 0.0], [2.0, -0.0, 0.0], [2.0, -2.0, 0.0]],
        ]
    );
}

#[test]
fn object_lines_svg_import_matches_meshlib_polygon_rect_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<polygon points="0,0 2,0 2,2" />"#,
        r#"<rect x="1" y="2" width="3" height="4" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![
            vec![
                [0.0, -0.0, 0.0],
                [2.0, -0.0, 0.0],
                [2.0, -2.0, 0.0],
                [0.0, -0.0, 0.0],
            ],
            vec![
                [1.0, -2.0, 0.0],
                [1.0, -6.0, 0.0],
                [4.0, -6.0, 0.0],
                [4.0, -2.0, 0.0],
                [1.0, -2.0, 0.0],
            ],
        ]
    );
}

#[test]
fn object_lines_svg_import_accepts_meshlib_compact_signed_points_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<polyline points="0,0 10-10 20,0" />"#,
        r#"<polygon points="0,0 2-2 4,0" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![
            vec![[0.0, -0.0, 0.0], [10.0, 10.0, 0.0], [20.0, -0.0, 0.0]],
            vec![
                [0.0, -0.0, 0.0],
                [2.0, 2.0, 0.0],
                [4.0, -0.0, 0.0],
                [0.0, -0.0, 0.0],
            ],
        ]
    );
}

#[test]
fn object_lines_svg_import_matches_meshlib_circle_ellipse_sampling_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<circle cx="1" cy="2" r="3" />"#,
        r#"<ellipse cx="-1" cy="4" rx="2" ry="1" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();
    let contours = lines::object_lines_to_contours(&object).unwrap();

    assert_eq!(contours.len(), 2);
    assert_eq!(contours[0].len(), 33);
    assert_eq!(contours[1].len(), 33);

    let assert_point_close = |actual: [f64; 3], expected: [f64; 3]| {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() <= 1e-9,
                "expected {actual:?} to be within 1e-9 of {expected:?}",
            );
        }
    };

    assert_point_close(contours[0][0], [4.0, -2.0, 0.0]);
    assert_point_close(contours[0][8], [1.0, -5.0, 0.0]);
    assert_point_close(contours[0][16], [-2.0, -2.0, 0.0]);
    assert_point_close(contours[0][24], [1.0, 1.0, 0.0]);
    assert_point_close(contours[0][32], [4.0, -2.0, 0.0]);

    assert_point_close(contours[1][0], [1.0, -4.0, 0.0]);
    assert_point_close(contours[1][8], [-1.0, -5.0, 0.0]);
    assert_point_close(contours[1][16], [-3.0, -4.0, 0.0]);
    assert_point_close(contours[1][24], [-1.0, -3.0, 0.0]);
    assert_point_close(contours[1][32], [1.0, -4.0, 0.0]);
}

#[test]
fn object_lines_svg_import_matches_meshlib_rounded_rect_sampling_y_flip() {
    let source = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect x="1" y="2" width="6" height="4" rx="2" ry="1" /></svg>"#;

    let object = lines::object_lines_from_svg(source).unwrap();
    let contours = lines::object_lines_to_contours(&object).unwrap();

    assert_eq!(contours.len(), 1);
    assert_eq!(contours[0].len(), 133);

    let assert_point_close = |actual: [f64; 3], expected: [f64; 3]| {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() <= 1e-9,
                "expected {actual:?} to be within 1e-9 of {expected:?}",
            );
        }
    };

    assert_point_close(contours[0][0], [5.0, -2.0, 0.0]);
    assert_point_close(
        contours[0][16],
        [
            5.0 + std::f64::consts::SQRT_2,
            -3.0 + std::f64::consts::SQRT_2 / 2.0,
            0.0,
        ],
    );
    assert_point_close(contours[0][32], [7.0, -3.0, 0.0]);
    assert_point_close(contours[0][33], [7.0, -5.0, 0.0]);
    assert_point_close(contours[0][65], [5.0, -6.0, 0.0]);
    assert_point_close(contours[0][66], [3.0, -6.0, 0.0]);
    assert_point_close(contours[0][98], [1.0, -5.0, 0.0]);
    assert_point_close(contours[0][99], [1.0, -3.0, 0.0]);
    assert_point_close(contours[0][131], [3.0, -2.0, 0.0]);
    assert_point_close(contours[0][132], [5.0, -2.0, 0.0]);
}

#[test]
fn object_lines_svg_import_matches_meshlib_linear_path_commands_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<path d="M 0 0 L 2 0 H 3 V 2 h -1 v 1 z M 10 0 l 0 2 2 0 z m 8 0 0 2 2 0 z" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![
            vec![
                [0.0, -0.0, 0.0],
                [2.0, -0.0, 0.0],
                [3.0, -0.0, 0.0],
                [3.0, -2.0, 0.0],
                [2.0, -2.0, 0.0],
                [2.0, -3.0, 0.0],
                [0.0, -0.0, 0.0],
            ],
            vec![
                [10.0, -0.0, 0.0],
                [10.0, -2.0, 0.0],
                [12.0, -2.0, 0.0],
                [10.0, -0.0, 0.0],
            ],
            vec![
                [18.0, -0.0, 0.0],
                [18.0, -2.0, 0.0],
                [20.0, -2.0, 0.0],
                [18.0, -0.0, 0.0],
            ],
        ]
    );
}

#[test]
fn object_lines_svg_import_matches_meshlib_curve_path_commands_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<path d="M 0 0 C 0 32 32 32 32 0 S 64 -32 64 0 Q 64 32 96 32 T 128 0" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();
    let contours = lines::object_lines_to_contours(&object).unwrap();

    assert_eq!(contours.len(), 1);
    let contour = &contours[0];
    assert_eq!(contour.len(), 129);
    let assert_point_close = |actual: [f64; 3], expected: [f64; 3]| {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() <= 1e-9,
                "axis {axis}: actual={actual:?} expected={expected:?}"
            );
        }
    };

    assert_point_close(contour[0], [0.0, -0.0, 0.0]);
    assert_point_close(contour[16], [16.0, -24.0, 0.0]);
    assert_point_close(contour[32], [32.0, -0.0, 0.0]);
    assert_point_close(contour[48], [48.0, 24.0, 0.0]);
    assert_point_close(contour[64], [64.0, -0.0, 0.0]);
    assert_point_close(contour[80], [72.0, -24.0, 0.0]);
    assert_point_close(contour[96], [96.0, -32.0, 0.0]);
    assert_point_close(contour[112], [120.0, -24.0, 0.0]);
    assert_point_close(contour[128], [128.0, -0.0, 0.0]);
}

#[test]
fn object_lines_svg_import_matches_meshlib_arc_path_commands_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<path d="M 0 0 A 10 10 0 0 1 20 0" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();
    let contours = lines::object_lines_to_contours(&object).unwrap();

    assert_eq!(contours.len(), 1);
    let contour = &contours[0];
    assert_eq!(contour.len(), 33);
    let assert_point_close = |actual: [f64; 3], expected: [f64; 3]| {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() <= 1e-9,
                "axis {axis}: actual={actual:?} expected={expected:?}"
            );
        }
    };

    assert_point_close(contour[0], [0.0, -0.0, 0.0]);
    assert_point_close(contour[16], [10.0, 10.0, 0.0]);
    assert_point_close(contour[32], [20.0, -0.0, 0.0]);
}

#[test]
fn object_lines_svg_import_matches_meshlib_transform_attributes_y_flip() {
    let source = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">"#,
        r#"<g transform="translate(10, 20)">"#,
        r#"<line x1="1" y1="2" x2="3" y2="4" transform="scale(2)" />"#,
        "</g>",
        r#"<line x1="1" y1="2" x2="3" y2="4" transform="matrix(1 2 3 4 5 6)" />"#,
        r#"<line x1="2" y1="1" x2="2" y2="2" transform="rotate(90, 1, 1)" />"#,
        r#"<line x1="1" y1="2" x2="3" y2="4" transform="skewX(45)" />"#,
        r#"<line x1="1" y1="2" x2="3" y2="4" transform="skewY(45)" />"#,
        "</svg>",
    );

    let object = lines::object_lines_from_svg(source).unwrap();
    let contours = lines::object_lines_to_contours(&object).unwrap();

    let expected = vec![
        vec![[12.0, -24.0, 0.0], [16.0, -28.0, 0.0]],
        vec![[12.0, -16.0, 0.0], [20.0, -28.0, 0.0]],
        vec![[1.0, -2.0, 0.0], [0.0, -2.0, 0.0]],
        vec![[3.0, -2.0, 0.0], [7.0, -4.0, 0.0]],
        vec![[1.0, -3.0, 0.0], [3.0, -7.0, 0.0]],
    ];
    assert_eq!(contours.len(), expected.len());
    for (actual_contour, expected_contour) in contours.iter().zip(expected.iter()) {
        assert_eq!(actual_contour.len(), expected_contour.len());
        for (actual, expected) in actual_contour.iter().zip(expected_contour.iter()) {
            for axis in 0..3 {
                assert!(
                    (actual[axis] - expected[axis]).abs() <= 1e-9,
                    "axis {axis}: actual={actual:?} expected={expected:?}"
                );
            }
        }
    }
}

#[test]
fn object_lines_dxf_export_matches_meshlib_polyline_entities() {
    let object = lines::object_lines_from_contours(
        &[vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]]],
        lines::ObjectLinesOptions::default(),
    )
    .unwrap();

    let source = lines::object_lines_to_dxf(&object).unwrap();

    assert!(source.starts_with("0\nSECTION\n2\nENTITIES\n"));
    assert!(source.contains("0\nPOLYLINE\n8\n0\n66\n1\n70\n9\n"));
    assert!(source.contains("0\nVERTEX\n8\n0\n70\n32\n10\n1\n20\n0\n30\n0\n"));
    assert!(source.ends_with("0\nENDSEC\n0\nEOF\n"));
}

#[test]
fn object_lines_mrlines_roundtrips_meshlib_binary_topology() {
    let object = lines::object_lines_from_contours(
        &[vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]],
        lines::ObjectLinesOptions::default(),
    )
    .unwrap();

    let bytes = lines::object_lines_to_mrlines(&object).unwrap();

    let mut expected = Vec::new();
    expected.extend_from_slice(&2_u32.to_le_bytes());
    expected.extend_from_slice(&0_i32.to_le_bytes());
    expected.extend_from_slice(&0_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    expected.extend_from_slice(&2_u32.to_le_bytes());
    expected.extend_from_slice(&0_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    expected.extend_from_slice(&3_u32.to_le_bytes());
    expected.extend_from_slice(&2_u32.to_le_bytes());
    for value in [0.0_f32, 0.0, 0.0, 1.0, 2.0, 3.0] {
        expected.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(bytes, expected);
    assert_eq!(
        lines::object_lines_to_contours(&lines::object_lines_from_mrlines(&bytes).unwrap())
            .unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]]
    );
}

#[test]
fn object_lines_ply_roundtrips_meshlib_binary_edges() {
    let object = lines::object_lines_from_contours(
        &[vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]],
        lines::ObjectLinesOptions::default(),
    )
    .unwrap();

    let bytes = lines::object_lines_to_ply(&object).unwrap();

    let mut expected = b"ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n\
element vertex 2\nproperty float x\nproperty float y\nproperty float z\n\
element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
        .to_vec();
    for value in [0.0_f32, 0.0, 0.0, 1.0, 2.0, 3.0] {
        expected.extend_from_slice(&value.to_le_bytes());
    }
    expected.extend_from_slice(&0_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    assert_eq!(bytes, expected);
    assert_eq!(
        lines::object_lines_to_contours(&lines::object_lines_from_ply(&bytes).unwrap()).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]]]
    );
}

#[test]
fn object_lines_ascii_ply_import_matches_meshlib_vertex_edge_loader() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "comment ascii line fixture\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 1\n",
        "1 2\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]]
    );
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_format_version_tuple() {
    let source = concat!(
        "ply\n",
        "format ascii 1.1\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_magic() {
    let source = concat!(
        "ply   \n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_trailing_format_line_tokens() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0 generated-by-tool\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_format_minor_prefix_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_rejects_meshlib_format_minor_alpha_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0alpha\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ascii_ply_import_rejects_meshlib_format_minor_underscore_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0_alpha\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_trailing_element_line_tokens() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2 generated-by-tool\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1 generated-by-tool\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_rejects_meshlib_element_count_alpha_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2vertices\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1edges\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ascii_ply_import_rejects_meshlib_element_count_underscore_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2_vertices\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1_edges\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_trailing_property_line_tokens() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x generated-by-tool\n",
        "property float y generated-by-tool\n",
        "property float z generated-by-tool\n",
        "element face 1\n",
        "property list uchar int vertex_indices generated-by-tool\n",
        "element edge 1\n",
        "property int vertex1 generated-by-tool\n",
        "property int vertex2 generated-by-tool\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "2 0 1\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_rejects_leading_header_keyword_whitespace_like_meshlib() {
    let base_lines = [
        "ply",
        "format ascii 1.0",
        "element vertex 2",
        "property float x",
        "property float y",
        "property float z",
        "element edge 1",
        "property int vertex1",
        "property int vertex2",
        "end_header",
        "0 0 0",
        "1 0 0",
        "0 1",
    ];
    for (line_index, line) in [
        (1, " format ascii 1.0"),
        (2, " element vertex 2"),
        (3, " property float x"),
    ] {
        let mut lines = base_lines;
        lines[line_index] = line;
        let source = lines.join("\n") + "\n";

        assert!(lines::object_lines_from_ply(source.as_bytes()).is_err());
    }
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_spaced_format_version_tuple() {
    let source = concat!(
        "ply\n",
        "format ascii 1 . 0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_trailing_space_after_end_header() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header \n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_ascii_ply_import_rejects_unknown_header_directives_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "made_up_header value\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ascii_ply_import_accepts_vertex_only_files_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "comment vertex only line fixture\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        object.points,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    );
    assert!(object.lines.is_empty());
    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        Vec::<Vec<[f64; 3]>>::new()
    );
}

#[test]
fn object_lines_ascii_ply_import_trims_meshlib_texturefile_comment_trailing_spaces() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "comment TextureFile brushed-metal.jpg   \t\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property float s\n",
        "property float t\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0 0.25 0.75\n",
        "1 0 0 0.5 0.125\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.texture_files, vec!["brushed-metal.jpg".to_string()]);
    assert_eq!(object.uv_coords, vec![[0.25, 0.75], [0.5, 0.125]]);
}

#[test]
fn object_lines_ascii_ply_import_casts_coordinates_to_vector3f_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property double x\n",
        "property double y\n",
        "property double z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0.123456789123 100000000.25 0.000000123456789\n",
        "1.987654321987 -100000000.25 3.141592653589793\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        object.points,
        vec![
            [0.12345679104328156, 100000000.0, 0.0000001234567861274627],
            [1.9876543283462524, -100000000.0, 3.1415927410125732],
        ]
    );
}

#[test]
fn object_lines_ascii_ply_import_wraps_narrow_vertex_coordinates_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property char x\n",
        "property short y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "257 65537 0\n",
        "0 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.points, vec![[1.0, 1.0, 0.0], [0.0, 0.0, 0.0]]);
    assert_eq!(object.lines, vec![[0, 1]]);
}

#[test]
fn object_lines_ascii_ply_import_preserves_meshlib_vertex_colors() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "comment ascii colored line fixture\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property uchar red\n",
        "property uchar green\n",
        "property uchar blue\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0 255 0 0\n",
        "1 0 0 0 255 0\n",
        "1 1 0 0 0 255\n",
        "0 1\n",
        "1 2\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        object.coloring_type,
        lines::ObjectLinesColoringType::PerVertex
    );
    assert_eq!(
        object.vert_colors,
        vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
    );
    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]]
    );
}

#[test]
fn object_lines_ply_import_prefers_meshlib_rgb_short_names_over_long_color_names() {
    let ascii = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 1\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property uchar r\n",
        "property uchar g\n",
        "property uchar b\n",
        "property uchar red\n",
        "property uchar green\n",
        "property uchar blue\n",
        "end_header\n",
        "0 0 0 1 2 3 200 201 202\n",
    );
    let ascii_object = lines::object_lines_from_ply(ascii.as_bytes()).unwrap();
    assert_eq!(ascii_object.vert_colors, vec![[1, 2, 3, 255]]);

    let mut binary = b"ply\nformat binary_little_endian 1.0\n\
element vertex 1\nproperty float x\nproperty float y\nproperty float z\n\
property uchar r\nproperty uchar g\nproperty uchar b\n\
property uchar red\nproperty uchar green\nproperty uchar blue\nend_header\n"
        .to_vec();
    for value in [0.0_f32, 0.0, 0.0] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    binary.extend_from_slice(&[1, 2, 3, 200, 201, 202]);

    let binary_object = lines::object_lines_from_ply(&binary).unwrap();
    assert_eq!(binary_object.vert_colors, ascii_object.vert_colors);
}

#[test]
fn mesh_ply_import_prefers_meshlib_uv_short_names_over_texture_names() {
    let source = [
        "ply\n",
        "format ascii 1.0\n",
        "comment TextureFile jewel_surface.jpg\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property float texture_u\n",
        "property float texture_v\n",
        "property float u\n",
        "property float v\n",
        "property uchar r\n",
        "property uchar g\n",
        "property uchar b\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
        "0 0 0 9.1 9.2 0.1 0.2 10 20 30\n",
        "1 0 0 9.3 9.4 0.3 0.4 40 50 60\n",
        "0 1 0 9.5 9.6 0.5 0.6 70 80 90\n",
        "3 0 1 2\n",
    ]
    .concat();

    let mesh = crate::mesh_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.faces, vec![[0, 1, 2]]);
    assert_eq!(
        mesh.vertex_uvs,
        vec![
            [0.1_f32 as f64, 0.2_f32 as f64],
            [0.3_f32 as f64, 0.4_f32 as f64],
            [0.5_f32 as f64, 0.6_f32 as f64]
        ]
    );
    assert_eq!(
        mesh.vertex_colors,
        vec![[10, 20, 30, 255], [40, 50, 60, 255], [70, 80, 90, 255]]
    );
    assert_eq!(mesh.texture_files, vec!["jewel_surface.jpg".to_string()]);
}

#[test]
fn mesh_ply_import_reads_binary_little_endian_meshlib_s_t_uvs() {
    let mut source = concat!(
        "ply\n",
        "format binary_little_endian 1.0\n",
        "comment TextureFile binary_surface.png\n",
        "element vertex 3\n",
        "property double x\n",
        "property double y\n",
        "property double z\n",
        "property float s\n",
        "property float t\n",
        "property uchar red\n",
        "property uchar green\n",
        "property uchar blue\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "property uchar red\n",
        "property uchar green\n",
        "property uchar blue\n",
        "property list uchar float texcoord\n",
        "end_header\n",
    )
    .as_bytes()
    .to_vec();
    for (point, uv, color) in [
        (
            [0.125_f64, 0.0, 0.0],
            [0.125_f32, 0.25_f32],
            [10_u8, 20, 30],
        ),
        ([1.25_f64, 0.0, 0.0], [0.375_f32, 0.5_f32], [40_u8, 50, 60]),
        ([0.0_f64, 1.5, 0.0], [0.625_f32, 0.75_f32], [70_u8, 80, 90]),
    ] {
        for value in point {
            source.extend_from_slice(&value.to_le_bytes());
        }
        for value in uv {
            source.extend_from_slice(&value.to_le_bytes());
        }
        source.extend_from_slice(&color);
    }
    source.push(3);
    for value in [0_i32, 1, 2] {
        source.extend_from_slice(&value.to_le_bytes());
    }
    source.extend_from_slice(&[1, 2, 3]);
    source.push(6);
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
        source.extend_from_slice(&value.to_le_bytes());
    }

    let mesh = crate::mesh_from_ply(&source).unwrap();

    assert_eq!(
        mesh.vertices,
        vec![[0.125, 0.0, 0.0], [1.25, 0.0, 0.0], [0.0, 1.5, 0.0]]
    );
    assert_eq!(mesh.faces, vec![[0, 1, 2]]);
    assert_eq!(
        mesh.vertex_uvs,
        vec![[0.125, 0.25], [0.375, 0.5], [0.625, 0.75]]
    );
    assert_eq!(
        mesh.vertex_colors,
        vec![[10, 20, 30, 255], [40, 50, 60, 255], [70, 80, 90, 255]]
    );
    assert_eq!(mesh.face_colors, vec![[1, 2, 3, 255]]);
    assert_eq!(
        mesh.tri_corner_uvs,
        vec![[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]]
    );
    assert_eq!(mesh.texture_files, vec!["binary_surface.png".to_string()]);
}

#[test]
fn mesh_ply_export_preserves_meshlib_texture_uvs_and_colors_through_rust() {
    let document = crate::MeshPlyDocument {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![[0, 1, 2]],
        vertex_colors: vec![[10, 20, 30, 255], [40, 50, 60, 255], [70, 80, 90, 255]],
        face_colors: vec![[1, 2, 3, 255]],
        vertex_uvs: vec![[0.125, 0.25], [0.375, 0.5], [0.625, 0.75]],
        vertex_normals: Vec::new(),
        tri_corner_uvs: vec![[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]],
        edges: Vec::new(),
        texture_files: vec!["jewel_surface.png".to_string()],
        texture_images: Vec::new(),
    };

    let bytes = crate::mesh_to_ply(&document).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();

    assert!(text.contains("comment TextureFile jewel_surface.png\n"));
    assert!(text.contains("property double s\nproperty double t\n"));
    assert!(text.contains("property list uchar float texcoord\n"));
    let parsed = crate::mesh_from_ply(&bytes).unwrap();

    assert_eq!(parsed.texture_files, vec!["jewel_surface.png".to_string()]);
    assert_eq!(parsed.vertices, document.vertices);
    assert_eq!(parsed.faces, document.faces);
    assert_eq!(parsed.vertex_colors, document.vertex_colors);
    assert_eq!(parsed.face_colors, document.face_colors);
    assert_eq!(parsed.vertex_uvs, document.vertex_uvs);
    assert_eq!(parsed.tri_corner_uvs, document.tri_corner_uvs);
}

#[test]
fn mesh_ply_import_packs_polygon_texcoord_lists_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 4\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "property list uchar float texcoord\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 1 0\n",
        "4 0 1 2 3 8 0.0 0.0 1.0 0.0 1.0 1.0 0.0 1.0\n",
    );

    let mesh = crate::mesh_from_ply(source.as_bytes()).unwrap();

    assert_eq!(mesh.faces, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(
        mesh.tri_corner_uvs,
        vec![
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [[0.0, 1.0], [0.0, 0.0], [0.0, 0.0]]
        ]
    );
}

#[test]
fn mesh_ply_import_keeps_polygon_face_colors_per_meshlib_source_face_row() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 4\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "property uchar red\n",
        "property uchar green\n",
        "property uchar blue\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 1 0\n",
        "4 0 1 2 3 7 8 9\n",
    );

    let mesh = crate::mesh_from_ply(source.as_bytes()).unwrap();

    assert_eq!(mesh.faces, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(mesh.face_colors, vec![[7, 8, 9, 255]]);
}

#[test]
fn mesh_ply_import_loads_first_existing_texture_like_meshlib_texturefile() {
    let texture_dir = std::env::temp_dir().join(format!(
        "zennah-ply-texture-{}-{}",
        std::process::id(),
        "load"
    ));
    std::fs::create_dir_all(&texture_dir).unwrap();
    let first_texture = texture_dir.join("jewel_surface.png");
    let second_texture = texture_dir.join("ignored_surface.png");
    std::fs::write(&first_texture, opaque_white_png()).unwrap();
    std::fs::write(&second_texture, opaque_white_png()).unwrap();
    let source = [
        "ply\n",
        "format ascii 1.0\n",
        "comment TextureFile missing_surface.png\n",
        "comment TextureFile jewel_surface.png\n",
        "comment TextureFile ignored_surface.png\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1 0\n",
        "3 0 1 2\n",
    ]
    .concat();

    let mesh = crate::mesh_from_ply_with_textures(source.as_bytes(), &texture_dir).unwrap();

    assert_eq!(
        mesh.texture_files,
        vec![
            "missing_surface.png".to_string(),
            "jewel_surface.png".to_string(),
            "ignored_surface.png".to_string()
        ]
    );
    assert_eq!(mesh.texture_images.len(), 1);
    let texture = &mesh.texture_images[0];
    assert_eq!(texture.file, "jewel_surface.png");
    assert_eq!(texture.width, 1);
    assert_eq!(texture.height, 1);
    assert_eq!(texture.filter, "Linear");
    assert_eq!(texture.wrap, "Clamp");
    assert_eq!(texture.pixels_rgba, vec![[255, 255, 255, 255]]);

    std::fs::remove_file(first_texture).unwrap();
    std::fs::remove_file(second_texture).unwrap();
    std::fs::remove_dir(texture_dir).unwrap();
}

#[test]
fn mesh_ply_import_trims_meshlib_texturefile_comment_trailing_spaces() {
    let texture_dir = std::env::temp_dir().join(format!(
        "zennah-ply-texture-{}-{}",
        std::process::id(),
        "trailing-spaces"
    ));
    std::fs::create_dir_all(&texture_dir).unwrap();
    let texture = texture_dir.join("jewel_surface.png");
    std::fs::write(&texture, opaque_white_png()).unwrap();
    let source = [
        "ply\n",
        "format ascii 1.0\n",
        "comment TextureFile jewel_surface.png   \n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1 0\n",
        "3 0 1 2\n",
    ]
    .concat();

    let mesh = crate::mesh_from_ply_with_textures(source.as_bytes(), &texture_dir).unwrap();

    assert_eq!(mesh.texture_files, vec!["jewel_surface.png".to_string()]);
    assert_eq!(mesh.texture_images.len(), 1);
    let loaded = &mesh.texture_images[0];
    assert_eq!(loaded.file, "jewel_surface.png");
    assert_eq!(loaded.pixels_rgba, vec![[255, 255, 255, 255]]);

    std::fs::remove_file(texture).unwrap();
    std::fs::remove_dir(texture_dir).unwrap();
}

#[test]
fn mesh_ply_import_reads_binary_big_endian_meshlib_scalars() {
    let mut source = concat!(
        "ply\n",
        "format binary_big_endian 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property uchar r\n",
        "property uchar g\n",
        "property uchar b\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "end_header\n",
    )
    .as_bytes()
    .to_vec();
    for (point, color) in [
        ([0.0_f32, 0.0, 0.0], [1_u8, 2, 3]),
        ([1.0_f32, 0.0, 0.0], [4_u8, 5, 6]),
        ([0.0_f32, 1.0, 0.0], [7_u8, 8, 9]),
    ] {
        for value in point {
            source.extend_from_slice(&value.to_be_bytes());
        }
        source.extend_from_slice(&color);
    }
    source.push(3);
    for value in [0_i32, 1, 2] {
        source.extend_from_slice(&value.to_be_bytes());
    }

    let mesh = crate::mesh_from_ply(&source).unwrap();

    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.faces, vec![[0, 1, 2]]);
    assert_eq!(
        mesh.vertex_colors,
        vec![[1, 2, 3, 255], [4, 5, 6, 255], [7, 8, 9, 255]]
    );
}

fn opaque_white_png() -> &'static [u8] {
    &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0x0f, 0x04, 0x00, 0x09, 0xfb, 0x03, 0xfd, 0xfb, 0x5e, 0x6b, 0x2b, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

#[test]
fn mesh_ply_import_exposes_meshlib_vertex_normals_and_edges() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property double nx\n",
        "property double ny\n",
        "property double nz\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "element edge 2\n",
        "property short vertex1\n",
        "property uint vertex2\n",
        "end_header\n",
        "0 0 0 0.0 0.0 1.0\n",
        "1 0 0 0.0 0.0 1.0\n",
        "0 1 0 0.0 0.0 1.0\n",
        "3 0 1 2\n",
        "0 1\n",
        "1 2\n",
    );

    let mesh = crate::mesh_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        mesh.vertex_normals,
        vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]
    );
    assert_eq!(mesh.edges, vec![[0, 1], [1, 2]]);
}

#[test]
fn object_lines_ascii_ply_import_casts_float_vertex_colors_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "comment ascii float-colored line fixture\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property float red\n",
        "property float green\n",
        "property float blue\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0 1.0 0.0 0.0\n",
        "1 0 0 0.0 0.5 1.0\n",
        "1 1 0 255.9 128.2 2.8\n",
        "0 1\n",
        "1 2\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        object.coloring_type,
        lines::ObjectLinesColoringType::PerVertex
    );
    assert_eq!(
        object.vert_colors,
        vec![[1, 0, 0, 255], [0, 0, 1, 255], [255, 128, 2, 255]]
    );
    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]]
    );
}

#[test]
fn object_lines_ply_import_wraps_integer_vertex_colors_like_meshlib() {
    let ascii = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property int red\n",
        "property int green\n",
        "property int blue\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0 -1 256 300\n",
        "1 0 0 -255 -256 -257\n",
        "1 1 0 511 512 513\n",
        "0 1\n",
        "1 2\n",
    );
    let ascii_object = lines::object_lines_from_ply(ascii.as_bytes()).unwrap();

    assert_eq!(
        ascii_object.vert_colors,
        vec![[255, 0, 44, 255], [1, 0, 255, 255], [255, 0, 1, 255]]
    );

    let mut binary = b"ply\nformat binary_little_endian 1.0\n\
element vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
property int red\nproperty int green\nproperty int blue\n\
element edge 2\nproperty int vertex1\nproperty int vertex2\nend_header\n"
        .to_vec();
    for (point, color) in [
        ([0.0_f32, 0.0, 0.0], [-1_i32, 256, 300]),
        ([1.0_f32, 0.0, 0.0], [-255_i32, -256, -257]),
        ([1.0_f32, 1.0, 0.0], [511_i32, 512, 513]),
    ] {
        for value in point {
            binary.extend_from_slice(&value.to_le_bytes());
        }
        for value in color {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [0_i32, 1, 1, 2] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    let binary_object = lines::object_lines_from_ply(&binary).unwrap();

    assert_eq!(binary_object.vert_colors, ascii_object.vert_colors);
}

#[test]
fn object_lines_ascii_ply_import_ignores_unneeded_list_properties_like_meshlib() {
    let with_vertex_list_before_xyz = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property list uchar int adjacent_vertices\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "2 1 2 0 0 0\n",
        "1 0 1 0 0\n",
        "0 1 1 0\n",
        "0 1\n",
        "1 2\n",
    );
    let with_vertex_list_after_xyz = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "property list uchar int adjacent_vertices\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0 2 1 2\n",
        "1 0 0 1 0\n",
        "1 1 0 0\n",
        "0 1\n",
        "1 2\n",
    );
    let with_edge_list = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "property list uchar float weights\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 1 2 0.5 0.25\n",
        "1 2 0\n",
    );

    for source in [
        with_vertex_list_before_xyz,
        with_vertex_list_after_xyz,
        with_edge_list,
    ] {
        let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();
        assert_eq!(
            lines::object_lines_to_contours(&object).unwrap(),
            vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]]
        );
    }
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_property_name_prefix_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float bad-name\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "99 0 0 0\n",
        "99 1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]]
    );
}

#[test]
fn object_lines_ascii_ply_import_rejects_non_identifier_property_names_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float 1bad\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "99 0 0 0\n",
        "99 1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ascii_ply_import_accepts_meshlib_last_integer_prefix_suffix() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1.9\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
}

#[test]
fn object_lines_ascii_ply_import_skips_meshlib_unsigned_negative_edge_endpoint() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property uint vertex1\n",
        "property uint vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 -1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert!(object.lines.is_empty());
}

#[test]
fn object_lines_ascii_ply_import_rejects_float64_type_alias_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float64 x\n",
        "property float64 y\n",
        "property float64 z\n",
        "element edge 1\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    assert_eq!(
        lines::object_lines_from_ply(source.as_bytes()).unwrap_err(),
        "unsupported .PLY file with polylines"
    );
}

#[test]
fn object_lines_ply_export_writes_meshlib_vertex_colors() {
    let object = lines::ObjectLinesDocument {
        points: vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]],
        lines: vec![[0, 1]],
        coloring_type: lines::ObjectLinesColoringType::PerVertex,
        vert_colors: vec![[255, 0, 0, 255], [0, 127, 255, 255]],
        ..lines::ObjectLinesDocument::default()
    };

    let bytes = lines::object_lines_to_ply(&object).unwrap();

    let mut expected = b"ply\nformat binary_little_endian 1.0\ncomment MeshInspector.com\n\
element vertex 2\nproperty float x\nproperty float y\nproperty float z\n\
property uchar red\nproperty uchar green\nproperty uchar blue\n\
element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
        .to_vec();
    for (point, color) in [
        ([0.0_f32, 0.0, 0.0], [255_u8, 0, 0]),
        ([1.0_f32, 2.0, 3.0], [0_u8, 127, 255]),
    ] {
        for value in point {
            expected.extend_from_slice(&value.to_le_bytes());
        }
        expected.extend_from_slice(&color);
    }
    expected.extend_from_slice(&0_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    assert_eq!(bytes, expected);
    assert_eq!(lines::object_lines_from_ply(&bytes).unwrap(), object);
}

#[test]
fn object_lines_ascii_ply_import_skips_mesh_face_elements_like_meshlib() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "comment ascii mesh and line fixture\n",
        "element vertex 4\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element face 1\n",
        "property list uchar int vertex_indices\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 1 0\n",
        "3 0 1 2\n",
        "0 3\n",
        "3 2\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]]]
    );
}

#[test]
fn object_lines_ascii_ply_import_ignores_invalid_edges_like_meshlib() {
    let self_loop = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 0\n",
        "0 1\n",
    );
    let out_of_range = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0 3\n",
        "0 1\n",
    );
    let negative = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "-1 1\n",
        "0 1\n",
    );

    for source in [self_loop, out_of_range, negative] {
        let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();
        assert_eq!(object.lines, vec![[0, 1]]);
    }

    let over_degree = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 4\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 3\n",
        "property int vertex1\n",
        "property int vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1 0\n",
        "0 -1 0\n",
        "0 1\n",
        "0 2\n",
        "0 3\n",
    );
    let object = lines::object_lines_from_ply(over_degree.as_bytes()).unwrap();
    assert_eq!(object.lines, vec![[0, 1], [0, 2]]);
}

#[test]
fn object_lines_ascii_ply_import_skips_edge_elements_without_meshlib_vertex_properties() {
    let source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property int source\n",
        "property int target\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 1\n",
    );

    let object = lines::object_lines_from_ply(source.as_bytes()).unwrap();

    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
    assert!(object.lines.is_empty());
}

#[test]
fn object_lines_ply_import_casts_float_edge_indices_like_meshlib() {
    let ascii = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 3\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 2\n",
        "property float vertex1\n",
        "property float vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "1 1 0\n",
        "0.9 1.2\n",
        "1.8 2.9\n",
    );
    let ascii_object = lines::object_lines_from_ply(ascii.as_bytes()).unwrap();
    assert_eq!(ascii_object.lines, vec![[0, 1], [1, 2]]);

    let mut binary = b"ply\nformat binary_little_endian 1.0\n\
element vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
element edge 2\nproperty float vertex1\nproperty float vertex2\nend_header\n"
        .to_vec();
    for point in [
        [0.0_f32, 0.0, 0.0],
        [1.0_f32, 0.0, 0.0],
        [1.0_f32, 1.0, 0.0],
    ] {
        for value in point {
            binary.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [0.9_f32, 1.2, 1.8, 2.9] {
        binary.extend_from_slice(&value.to_le_bytes());
    }

    let binary_object = lines::object_lines_from_ply(&binary).unwrap();
    assert_eq!(binary_object.lines, ascii_object.lines);
}

#[test]
fn object_lines_ascii_ply_import_wraps_narrow_edge_indices_like_meshlib() {
    let char_source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property char vertex1\n",
        "property char vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 257\n",
    );
    let char_object = lines::object_lines_from_ply(char_source.as_bytes()).unwrap();
    assert_eq!(char_object.lines, vec![[0, 1]]);

    let short_source = concat!(
        "ply\n",
        "format ascii 1.0\n",
        "element vertex 2\n",
        "property float x\n",
        "property float y\n",
        "property float z\n",
        "element edge 1\n",
        "property short vertex1\n",
        "property short vertex2\n",
        "end_header\n",
        "0 0 0\n",
        "1 0 0\n",
        "0 65537\n",
    );
    let short_object = lines::object_lines_from_ply(short_source.as_bytes()).unwrap();
    assert_eq!(short_object.lines, vec![[0, 1]]);
}

#[test]
fn object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_unneeded_vertex_property() {
    let mut bytes = b"ply\nformat binary_little_endian 1.0\n\
element vertex 2\nproperty list float int ghost\nproperty float x\nproperty float y\nproperty float z\n\
element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
        .to_vec();
    for point in [[0.0_f32, 0.0, 0.0], [1.0_f32, 0.0, 0.0]] {
        bytes.extend_from_slice(&1.9_f32.to_le_bytes());
        bytes.extend_from_slice(&99_i32.to_le_bytes());
        for value in point {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());

    let object = lines::object_lines_from_ply(&bytes).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_binary_ply_import_accepts_meshlib_float_list_count_on_skipped_element() {
    let mut bytes = b"ply\nformat binary_little_endian 1.0\n\
element vertex 2\nproperty float x\nproperty float y\nproperty float z\n\
element ghost 1\nproperty list float int payload\n\
element edge 1\nproperty int vertex1\nproperty int vertex2\nend_header\n"
        .to_vec();
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&1.9_f32.to_le_bytes());
    bytes.extend_from_slice(&7_i32.to_le_bytes());
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());

    let object = lines::object_lines_from_ply(&bytes).unwrap();

    assert_eq!(object.lines, vec![[0, 1]]);
    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
}

#[test]
fn object_lines_binary_ply_import_skips_edge_elements_without_meshlib_vertex_properties() {
    let mut bytes = b"ply\nformat binary_little_endian 1.0\n\
element vertex 2\nproperty float x\nproperty float y\nproperty float z\n\
element edge 1\nproperty int source\nproperty int target\nend_header\n"
        .to_vec();
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());

    let object = lines::object_lines_from_ply(&bytes).unwrap();

    assert_eq!(object.points, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
    assert!(object.lines.is_empty());
}

#[test]
fn object_lines_binary_big_endian_ply_import_matches_meshlib_vertex_edge_loader() {
    let mut bytes = b"ply\nformat binary_big_endian 1.0\ncomment big endian line fixture\n\
element vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
element edge 2\nproperty int vertex1\nproperty int vertex2\nend_header\n"
        .to_vec();
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    for value in [0_i32, 1, 1, 2] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    let object = lines::object_lines_from_ply(&bytes).unwrap();

    assert_eq!(
        lines::object_lines_to_contours(&object).unwrap(),
        vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]]
    );
}

#[test]
fn distance_map_from_mesh_matches_meshlib_pixel_center_rays() {
    let vertices = vec![
        [0.0, 0.0, 2.0],
        [2.0, 0.0, 2.0],
        [2.0, 2.0, 2.0],
        [0.0, 2.0, 2.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let map = distance_map_from_mesh(
        &vertices,
        &faces,
        2,
        2,
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.0, 0.0, 1.0],
        1e-8,
    )
    .unwrap();

    assert_eq!(map.width, 2);
    assert_eq!(map.height, 2);
    assert_eq!(map.valid_count, 4);
    assert_eq!(map.origin, [0.0, 0.0]);
    assert_eq!(map.pixel_size, [1.0, 1.0]);
    assert_eq!(map.values, vec![2.0, 2.0, 2.0, 2.0]);
    assert_eq!(map.min_value, 2.0);
    assert_eq!(map.max_value, 2.0);
}

#[test]
fn distance_map_from_tiff_matches_meshlib_scalar_float_import() {
    let path = std::env::temp_dir().join(format!(
        "zennah-distance-map-{}-{}.tiff",
        std::process::id(),
        "scalar-float"
    ));
    let values = vec![1.25_f32, -2.5, 3.75, 4.5, 5.25, -6.5];

    {
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = tiff::encoder::TiffEncoder::new(file).unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::Gray32Float>(3, 2, &values)
            .unwrap();
    }

    let map = distance_map_from_tiff(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(map.width, 3);
    assert_eq!(map.height, 2);
    assert_eq!(map.origin, [0.0, 0.0]);
    assert_eq!(map.pixel_size, [1.0, 1.0]);
    assert_eq!(map.valid_count, 6);
    assert_eq!(map.min_value, -6.5);
    assert_eq!(map.max_value, 5.25);
    assert_eq!(map.values, values);
}

#[test]
fn distance_map_from_tiff_converts_rgb_samples_like_meshlib_raw_tiff_reader() {
    let path = std::env::temp_dir().join(format!(
        "zennah-distance-map-{}-{}.tiff",
        std::process::id(),
        "rgb"
    ));
    let samples = vec![10_u8, 20, 30, 100, 50, 0];

    {
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = tiff::encoder::TiffEncoder::new(file).unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::RGB8>(2, 1, &samples)
            .unwrap();
    }

    let map = distance_map_from_tiff(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(map.width, 2);
    assert_eq!(map.height, 1);
    assert!((map.values[0] - 18.15).abs() < 1e-5);
    assert!((map.values[1] - 59.25).abs() < 1e-5);
}

#[test]
fn distance_map_to_tiff_writes_meshlib_scalar_float_tiff_and_nodata_tag() {
    let path = std::env::temp_dir().join(format!(
        "zennah-distance-map-{}-{}.tiff",
        std::process::id(),
        "export"
    ));
    let map = DistanceMapGrid {
        width: 3,
        height: 2,
        origin: [0.0, 0.0],
        pixel_size: [1.0, 1.0],
        model_transform: None,
        values: vec![1.25, f32::MIN, 3.75, 4.5, 5.25, -6.5],
        valid_count: 5,
        min_value: -6.5,
        max_value: 5.25,
    };

    distance_map_to_tiff(&map, &path).unwrap();

    let file = std::fs::File::open(&path).unwrap();
    let mut decoder = tiff::decoder::Decoder::new(file).unwrap();
    assert_eq!(decoder.dimensions().unwrap(), (3, 2));
    assert_eq!(decoder.colortype().unwrap(), tiff::ColorType::Gray(32));
    assert_eq!(
        decoder
            .get_tag_u32(tiff::tags::Tag::PhotometricInterpretation)
            .unwrap(),
        tiff::tags::PhotometricInterpretation::WhiteIsZero.to_u16() as u32
    );
    assert_eq!(
        decoder
            .get_tag_ascii_string(tiff::tags::Tag::GdalNodata)
            .unwrap(),
        format!("{:.16e}", DISTANCE_MAP_NOT_VALID_VALUE)
    );
    drop(decoder);

    let reloaded = distance_map_from_tiff(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(reloaded.values, map.values);
    assert_eq!(reloaded.valid_count, map.valid_count);
}

#[test]
fn distance_map_to_tiff_preserves_meshlib_model_transform_metadata() {
    let path = std::env::temp_dir().join(format!(
        "zennah-distance-map-{}-{}.tiff",
        std::process::id(),
        "transform"
    ));
    let map = DistanceMapGrid {
        width: 2,
        height: 2,
        origin: [10.0, 20.0],
        pixel_size: [2.5, 4.0],
        model_transform: None,
        values: vec![1.0, 2.0, 3.0, 4.0],
        valid_count: 4,
        min_value: 1.0,
        max_value: 4.0,
    };

    distance_map_to_tiff(&map, &path).unwrap();

    let file = std::fs::File::open(&path).unwrap();
    let mut decoder = tiff::decoder::Decoder::new(file).unwrap();
    let matrix = decoder
        .get_tag_f64_vec(tiff::tags::Tag::ModelTransformationTag)
        .unwrap();
    assert_eq!(matrix.len(), 16);
    assert_eq!(
        matrix,
        vec![2.5, 0.0, 0.0, 10.0, 0.0, 4.0, 0.0, 20.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,]
    );
    drop(decoder);

    let reloaded = distance_map_from_tiff(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(reloaded.origin, map.origin);
    assert_eq!(reloaded.pixel_size, map.pixel_size);
    assert_eq!(reloaded.values, map.values);
}

#[test]
fn distance_map_tiff_roundtrip_preserves_arbitrary_meshlib_model_transform_metadata() {
    let path = std::env::temp_dir().join(format!(
        "zennah-distance-map-{}-{}.tiff",
        std::process::id(),
        "rotated-transform-source"
    ));
    let exported_path = std::env::temp_dir().join(format!(
        "zennah-distance-map-{}-{}.tiff",
        std::process::id(),
        "rotated-transform-export"
    ));
    let values = vec![1.0_f32, 2.0, 3.0, 4.0];
    let model_transform = vec![
        0.0, -2.0, 0.0, 10.0, 3.0, 0.0, 0.5, 20.0, 0.0, 0.0, 1.25, 30.0, 0.0, 0.0, 0.0, 1.0,
    ];

    {
        let file = std::fs::File::create(&path).unwrap();
        let mut encoder = tiff::encoder::TiffEncoder::new(file).unwrap();
        let mut image = encoder
            .new_image::<tiff::encoder::colortype::Gray32Float>(2, 2)
            .unwrap();
        image
            .encoder()
            .write_tag(
                tiff::tags::Tag::ModelTransformationTag,
                &model_transform[..],
            )
            .unwrap();
        image.write_data(&values).unwrap();
    }

    let map = distance_map_from_tiff(&path).unwrap();
    distance_map_to_tiff(&map, &exported_path).unwrap();

    let file = std::fs::File::open(&exported_path).unwrap();
    let mut decoder = tiff::decoder::Decoder::new(file).unwrap();
    let exported_transform = decoder
        .get_tag_f64_vec(tiff::tags::Tag::ModelTransformationTag)
        .unwrap();
    drop(decoder);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(exported_path).unwrap();

    assert_eq!(exported_transform, model_transform);
}

#[test]
fn distance_map_to_iso_segments_matches_meshlib_pixel_center_coordinates() {
    let map = DistanceMapGrid {
        width: 2,
        height: 2,
        origin: [0.0, 0.0],
        pixel_size: [1.0, 1.0],
        model_transform: None,
        values: vec![-1.0, 1.0, -1.0, 1.0],
        valid_count: 4,
        min_value: -1.0,
        max_value: 1.0,
    };

    let iso = distance_map_to_iso_segments(&map, 0.0).unwrap();

    assert_eq!(iso.iso_value, 0.0);
    assert_eq!(iso.segments.len(), 1);
    assert_eq!(iso.segments[0][0], [1.0, 1.5]);
    assert_eq!(iso.segments[0][1], [1.0, 0.5]);
}

#[test]
fn distance_map_merge_matches_meshlib_invalid_and_mismatched_extent_contract() {
    let left = DistanceMapGrid {
        width: 3,
        height: 2,
        origin: [10.0, 20.0],
        pixel_size: [2.0, 4.0],
        model_transform: None,
        values: vec![2.0, f32::MIN, -1.0, 4.0, 8.0, 16.0],
        valid_count: 5,
        min_value: -1.0,
        max_value: 16.0,
    };
    let right = DistanceMapGrid {
        width: 2,
        height: 2,
        origin: [10.0, 20.0],
        pixel_size: [2.0, 4.0],
        model_transform: None,
        values: vec![3.0, 5.0, f32::MIN, 6.0],
        valid_count: 3,
        min_value: 3.0,
        max_value: 6.0,
    };

    let merged_min = distance_map_merge(&left, &right, DistanceMapMergeMode::Min).unwrap();
    let merged_max = distance_map_merge(&left, &right, DistanceMapMergeMode::Max).unwrap();
    let subtracted = distance_map_merge(&left, &right, DistanceMapMergeMode::Subtract).unwrap();

    assert_eq!(merged_min.width, 3);
    assert_eq!(merged_min.height, 2);
    assert_eq!(merged_min.origin, left.origin);
    assert_eq!(merged_min.pixel_size, left.pixel_size);
    assert_eq!(merged_min.values, vec![2.0, 5.0, -1.0, 4.0, 6.0, 16.0]);
    assert_eq!(merged_min.valid_count, 6);
    assert_eq!(merged_min.min_value, -1.0);
    assert_eq!(merged_min.max_value, 16.0);

    assert_eq!(merged_max.values, vec![3.0, 5.0, -1.0, 4.0, 8.0, 16.0]);
    assert_eq!(merged_max.valid_count, 6);
    assert_eq!(merged_max.min_value, -1.0);
    assert_eq!(merged_max.max_value, 16.0);

    assert_eq!(
        subtracted.values,
        vec![-1.0, f32::MIN, -1.0, f32::MIN, 2.0, 16.0]
    );
    assert_eq!(subtracted.valid_count, 4);
    assert_eq!(subtracted.min_value, -1.0);
    assert_eq!(subtracted.max_value, 16.0);
}

#[test]
fn distance_map_contour_boolean_matches_meshlib_composition() {
    let contours_a = vec![vec![
        [0.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [0.0, 2.0],
        [0.0, 0.0],
    ]];
    let contours_b = vec![vec![
        [1.0, 0.0],
        [3.0, 0.0],
        [3.0, 2.0],
        [1.0, 2.0],
        [1.0, 0.0],
    ]];

    let union = distance_map_contour_boolean(
        &contours_a,
        &contours_b,
        ContourBooleanMode::Union,
        6,
        5,
        [-1.0, -1.0],
        [1.0, 1.0],
        0.0,
    )
    .unwrap();
    let intersection = distance_map_contour_boolean(
        &contours_a,
        &contours_b,
        ContourBooleanMode::Intersection,
        6,
        5,
        [-1.0, -1.0],
        [1.0, 1.0],
        0.0,
    )
    .unwrap();
    let subtract = distance_map_contour_boolean(
        &contours_a,
        &contours_b,
        ContourBooleanMode::Subtract,
        6,
        5,
        [-1.0, -1.0],
        [1.0, 1.0],
        0.0,
    )
    .unwrap();

    assert_eq!(union.segments.len(), 10);
    assert_eq!(
        intersection.segments,
        vec![
            [[1.5, 0.0], [1.0, 0.5]],
            [[2.0, 0.5], [1.5, 0.0]],
            [[1.0, 0.5], [1.0, 1.5]],
            [[2.0, 1.5], [2.0, 0.5]],
            [[1.0, 1.5], [1.5, 2.0]],
            [[1.5, 2.0], [2.0, 1.5]],
        ]
    );
    assert_eq!(
        subtract.segments,
        vec![
            [[0.5, 0.0], [0.0, 0.5]],
            [[1.0, 0.5], [0.5, 0.0]],
            [[0.0, 0.5], [0.0, 1.5]],
            [[1.0, 1.5], [1.0, 0.5]],
            [[0.0, 1.5], [0.5, 2.0]],
            [[0.5, 2.0], [1.0, 1.5]],
        ]
    );
}

#[test]
fn gcode_linear_paths_match_meshlib_processor_modal_motion() {
    let source = "\n; header retained as a non-empty MeshLib frame\nG90\nG0 X0 Y0 Z1 F3000\nG1 X1 Y0 F1200\nY1 ; modal G1 movement\nG91\nX0.5 (relative modal movement)\nG20\nX1\nG21\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 10);
    assert_eq!(parsed.command_count, 16);
    assert_eq!(parsed.segments.len(), 5);
    assert_eq!(
        parsed
            .segments
            .iter()
            .map(|segment| segment.source_frame_index)
            .collect::<Vec<_>>(),
        vec![2, 3, 4, 6, 8]
    );
    assert!(parsed.segments[0].idle);
    assert!(!parsed.segments[1].idle);
    assert_eq!(parsed.segments[0].start, [0.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].end, [0.0, 0.0, 1.0]);
    assert_eq!(parsed.segments[1].start, [0.0, 0.0, 1.0]);
    assert_eq!(parsed.segments[1].end, [1.0, 0.0, 1.0]);
    assert_eq!(parsed.segments[2].end, [1.0, 1.0, 1.0]);
    assert_eq!(parsed.segments[3].end, [1.5, 1.0, 1.0]);
    assert!((parsed.segments[4].end[0] - 26.9).abs() < 1e-9);
    assert_eq!(parsed.segments[4].end[1], 1.0);
    assert_eq!(parsed.segments[4].end[2], 1.0);
    assert_eq!(parsed.segments[1].feedrate, 1200.0);
    assert_eq!(parsed.segments[4].feedrate, 1200.0);
    assert_eq!(parsed.max_feedrate, 1200.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_command_values_match_meshlib_strtof_narrowing() {
    let parsed =
        gcode::parse_gcode_paths("G90\nG1 X0.123456789 Y0.333333333 Z0.100000001 F1234.56789\n")
            .unwrap();

    assert_eq!(parsed.frame_count, 2);
    assert_eq!(parsed.command_count, 6);
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(
        parsed.segments[0].end,
        [
            f64::from(0.123456789_f32),
            f64::from(0.333333333_f32),
            f64::from(0.100000001_f32),
        ]
    );
    assert_eq!(parsed.segments[0].feedrate, f64::from(1234.56789_f32));
    assert_eq!(parsed.max_feedrate, f64::from(1234.56789_f32));
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_command_values_accept_meshlib_strtof_special_float_tokens() {
    let parsed = gcode::parse_gcode_paths("G90\nG1 Xnan F600\n").unwrap();

    assert_eq!(parsed.frame_count, 2);
    assert_eq!(parsed.command_count, 4);
    assert_eq!(parsed.segments.len(), 1);
    assert!(parsed.segments[0].end[0].is_nan());
    assert_eq!(parsed.segments[0].feedrate, 600.0);
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_command_values_accept_meshlib_strtof_hex_float_tokens() {
    let parsed = gcode::parse_gcode_paths("G90\nG1 X0x1p+2 F600\n").unwrap();

    assert_eq!(parsed.frame_count, 2);
    assert_eq!(parsed.command_count, 4);
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].end, [4.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].feedrate, 600.0);
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_command_values_accept_meshlib_strtof_leading_whitespace() {
    let parsed = gcode::parse_gcode_paths("G90\nG1 X 2 F600\n").unwrap();

    assert_eq!(parsed.frame_count, 2);
    assert_eq!(parsed.command_count, 4);
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].end, [2.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].feedrate, 600.0);
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_arc_paths_match_meshlib_center_offset_sampling() {
    let source = "G90\nG0 X1 Y0 Z0\nG3 X0 Y1 I-1 J0 F600\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 11);
    assert_eq!(parsed.segments.len(), 16);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert!(parsed.segments[0].idle);
    assert!(parsed.segments[1..]
        .iter()
        .all(|segment| segment.source_frame_index == 2 && !segment.idle));
    assert_eq!(parsed.segments[1].start, [1.0, 0.0, 0.0]);
    let first_arc_end = parsed.segments[1].end;
    assert!((first_arc_end[0] - (std::f64::consts::PI / 30.0).cos()).abs() < 1e-9);
    assert!((first_arc_end[1] - (std::f64::consts::PI / 30.0).sin()).abs() < 1e-9);
    assert_eq!(first_arc_end[2], 0.0);
    let last = parsed.segments.last().unwrap();
    assert!(last.end[0].abs() < 1e-9);
    assert!((last.end[1] - 1.0).abs() < 1e-9);
    assert_eq!(last.end[2], 0.0);
    assert!(parsed.segments[1..]
        .iter()
        .all(|segment| segment.feedrate == 600.0));
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_arc_radius_mismatch_warning_matches_meshlib_to_string_float_format() {
    let source = "G90\nG0 X1 Y0 Z0\nG3 X0 Y2 I-1 J0 F600\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 11);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert!(parsed.segments[1..]
        .iter()
        .all(|segment| segment.source_frame_index == 2 && !segment.idle));
    assert_eq!(
        parsed.warnings,
        vec!["frame 2: Begin and end radius are different: diff = 1.732051".to_string()]
    );
}

#[test]
fn gcode_radius_only_arc_matches_meshlib_no_motion_feedrate_contract() {
    let source = "G90\nG0 X1 Y0 Z0\nG2 R1 F600\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 8);
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert!(parsed.segments[0].idle);
    assert_eq!(parsed.segments[0].end, [1.0, 0.0, 0.0]);
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_feedrate_only_frame_updates_meshlib_feedrate_max_without_segments() {
    let source = "G90\nG1 F600\nG0 X1\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 5);
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].source_frame_index, 2);
    assert!(parsed.segments[0].idle);
    assert_eq!(parsed.segments[0].end, [1.0, 0.0, 0.0]);
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_arc_paths_match_meshlib_g18_g19_work_plane_mapping() {
    let zx =
        gcode::parse_gcode_paths("G90\nG18\nG0 X1 Y0 Z0\nG2 X0 Y0 Z1 I-1 J0 K0 F600\n").unwrap();

    assert_eq!(zx.frame_count, 4);
    assert_eq!(zx.command_count, 14);
    assert_eq!(zx.segments.len(), 16);
    assert_eq!(zx.segments[0].source_frame_index, 2);
    assert!(zx.segments[1..]
        .iter()
        .all(|segment| segment.source_frame_index == 3 && !segment.idle));
    assert_eq!(zx.segments[1].start, [1.0, 0.0, 0.0]);
    let zx_first_arc_end = zx.segments[1].end;
    assert!((zx_first_arc_end[0] - (std::f64::consts::PI / 30.0).cos()).abs() < 1e-9);
    assert_eq!(zx_first_arc_end[1], 0.0);
    assert!((zx_first_arc_end[2] - (std::f64::consts::PI / 30.0).sin()).abs() < 1e-9);
    let zx_last = zx.segments.last().unwrap();
    assert!(zx_last.end[0].abs() < 1e-9);
    assert_eq!(zx_last.end[1], 0.0);
    assert!((zx_last.end[2] - 1.0).abs() < 1e-9);
    assert!(zx.warnings.is_empty());

    let yz =
        gcode::parse_gcode_paths("G90\nG19\nG0 X0 Y1 Z0\nG3 X0 Y0 Z1 I0 J-1 K0 F700\n").unwrap();

    assert_eq!(yz.frame_count, 4);
    assert_eq!(yz.command_count, 14);
    assert_eq!(yz.segments.len(), 16);
    assert_eq!(yz.segments[0].source_frame_index, 2);
    assert!(yz.segments[1..]
        .iter()
        .all(|segment| segment.source_frame_index == 3 && !segment.idle));
    assert_eq!(yz.segments[1].start, [0.0, 1.0, 0.0]);
    let yz_first_arc_end = yz.segments[1].end;
    assert_eq!(yz_first_arc_end[0], 0.0);
    assert!((yz_first_arc_end[1] - (std::f64::consts::PI / 30.0).cos()).abs() < 1e-9);
    assert!((yz_first_arc_end[2] - (std::f64::consts::PI / 30.0).sin()).abs() < 1e-9);
    let yz_last = yz.segments.last().unwrap();
    assert_eq!(yz_last.end[0], 0.0);
    assert!(yz_last.end[1].abs() < 1e-9);
    assert!((yz_last.end[2] - 1.0).abs() < 1e-9);
    assert!(yz.segments[1..]
        .iter()
        .all(|segment| segment.feedrate == 700.0));
    assert_eq!(yz.max_feedrate, 700.0);
    assert!(yz.warnings.is_empty());
}

#[test]
fn gcode_scaling_matches_meshlib_g51_g50_contract() {
    let source = "G90\nG51 X2 Y3 Z4\nG0 X1 Y1 Z1\nG51 X0 Y5\nG1 X2 Y2 Z2 F500\nG50\nG1 X2 Y2 Z2\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 7);
    assert_eq!(parsed.command_count, 22);
    assert_eq!(parsed.segments.len(), 3);
    assert_eq!(
        parsed
            .segments
            .iter()
            .map(|segment| segment.source_frame_index)
            .collect::<Vec<_>>(),
        vec![2, 4, 6]
    );
    assert!(parsed.segments[0].idle);
    assert!(!parsed.segments[1].idle);
    assert!(!parsed.segments[2].idle);
    assert_eq!(parsed.segments[0].start, [0.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].end, [2.0, 3.0, 4.0]);
    assert_eq!(parsed.segments[1].start, [2.0, 3.0, 4.0]);
    assert_eq!(parsed.segments[1].end, [4.0, 10.0, 8.0]);
    assert_eq!(parsed.segments[2].start, [4.0, 10.0, 8.0]);
    assert_eq!(parsed.segments[2].end, [2.0, 2.0, 2.0]);
    assert_eq!(parsed.segments[1].feedrate, 500.0);
    assert_eq!(parsed.segments[2].feedrate, 500.0);
    assert_eq!(parsed.max_feedrate, 500.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_rotary_axis_matches_meshlib_default_c_axis_sampling() {
    let source = "G90\nG0 X1 Y0 Z0\nG1 C90 F600\nG1 X2 Y0 C180 F700\n";

    let parsed = gcode::parse_gcode_paths(source).unwrap();

    assert_eq!(parsed.frame_count, 4);
    assert_eq!(parsed.command_count, 13);
    assert_eq!(parsed.segments.len(), 41);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert_eq!(parsed.segments[1].source_frame_index, 2);
    assert_eq!(parsed.segments[20].source_frame_index, 2);
    assert_eq!(parsed.segments[21].source_frame_index, 3);
    assert_eq!(parsed.segments[40].source_frame_index, 3);
    assert!(parsed.segments[0].idle);
    assert!(parsed.segments[1..].iter().all(|segment| !segment.idle));
    assert_eq!(parsed.segments[0].start, [0.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].end, [1.0, 0.0, 0.0]);

    let first_rotation_end = parsed.segments[1].end;
    assert!((first_rotation_end[0] - (4.5_f64.to_radians()).cos()).abs() < 1e-9);
    assert!((first_rotation_end[1] - (4.5_f64.to_radians()).sin()).abs() < 1e-9);
    assert_eq!(first_rotation_end[2], 0.0);
    assert!(parsed.segments[20].end[0].abs() < 1e-9);
    assert!((parsed.segments[20].end[1] - 1.0).abs() < 1e-9);
    assert_eq!(parsed.segments[20].end[2], 0.0);

    assert!(parsed.segments[21].start[0].abs() < 1e-9);
    assert!((parsed.segments[21].start[1] - 1.0).abs() < 1e-9);
    let mid_rotary_line_end = parsed.segments[30].end;
    let mid_expected = 1.5 * 135.0_f64.to_radians().cos();
    assert!((mid_rotary_line_end[0] - mid_expected).abs() < 1e-9);
    assert!((mid_rotary_line_end[1] - 1.5 * 135.0_f64.to_radians().sin()).abs() < 1e-9);
    assert_eq!(mid_rotary_line_end[2], 0.0);
    assert!((parsed.segments[40].end[0] + 2.0).abs() < 1e-9);
    assert!(parsed.segments[40].end[1].abs() < 1e-9);
    assert_eq!(parsed.segments[40].end[2], 0.0);
    assert!(parsed.segments[1..=20]
        .iter()
        .all(|segment| segment.feedrate == 600.0));
    assert!(parsed.segments[21..]
        .iter()
        .all(|segment| segment.feedrate == 700.0));
    assert_eq!(parsed.max_feedrate, 700.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_tool_directions_match_meshlib_default_rotated_plus_z() {
    let parsed = gcode::parse_gcode_paths("G90\nG0 X0 Y0 Z1\nG1 A90 F600\n").unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 8);
    assert_eq!(parsed.segments.len(), 21);
    assert_eq!(parsed.segments[0].tool_direction_start, [0.0, 0.0, 1.0]);
    assert_eq!(parsed.segments[0].tool_direction_end, [0.0, 0.0, 1.0]);
    assert_eq!(parsed.segments[1].tool_direction_start, [0.0, 0.0, 1.0]);
    assert!((parsed.segments[1].tool_direction_end[0]).abs() < 1e-9);
    assert!((parsed.segments[1].tool_direction_end[1] - 4.5_f64.to_radians().sin()).abs() < 1e-9);
    assert!((parsed.segments[1].tool_direction_end[2] - 4.5_f64.to_radians().cos()).abs() < 1e-9);
    assert!((parsed.segments[20].tool_direction_end[0]).abs() < 1e-9);
    assert!((parsed.segments[20].tool_direction_end[1] - 1.0).abs() < 1e-9);
    assert!((parsed.segments[20].tool_direction_end[2]).abs() < 1e-9);
    assert!(parsed.segments[1..]
        .iter()
        .all(|segment| segment.feedrate == 600.0));
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_custom_cnc_home_and_idle_feedrate_match_meshlib_settings() {
    let settings = gcode::GcodeMachineSettings {
        home_position: [2.0, 3.0, 4.0],
        feedrate_idle: 1234.0,
        ..Default::default()
    };

    let parsed =
        gcode::parse_gcode_paths_with_settings("G90\nG0 X1 Y0 Z0\nG28\n", &settings).unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 6);
    assert_eq!(parsed.segments.len(), 2);
    assert!(parsed.segments.iter().all(|segment| segment.idle));
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert_eq!(parsed.segments[1].source_frame_index, 2);
    assert_eq!(parsed.segments[0].start, [2.0, 3.0, 4.0]);
    assert_eq!(parsed.segments[0].end, [1.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[1].start, [1.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[1].end, [2.0, 3.0, 4.0]);
    assert!(parsed
        .segments
        .iter()
        .all(|segment| segment.feedrate == 1234.0));
    assert_eq!(parsed.max_feedrate, 0.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_zero_idle_feedrate_is_rewritten_to_meshlib_final_feedrate_max() {
    let settings = gcode::GcodeMachineSettings {
        feedrate_idle: 0.0,
        ..Default::default()
    };

    let parsed =
        gcode::parse_gcode_paths_with_settings("G90\nG0 X1\nG1 X2 F600\n", &settings).unwrap();

    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 6);
    assert_eq!(parsed.segments.len(), 2);
    assert!(parsed.segments[0].idle);
    assert!(!parsed.segments[1].idle);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert_eq!(parsed.segments[1].source_frame_index, 2);
    assert_eq!(parsed.segments[0].feedrate, 600.0);
    assert_eq!(parsed.segments[1].feedrate, 600.0);
    assert_eq!(parsed.max_feedrate, 600.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_g28_at_home_emits_meshlib_zero_length_idle_action() {
    let settings = gcode::GcodeMachineSettings {
        feedrate_idle: 1234.0,
        ..Default::default()
    };

    let parsed = gcode::parse_gcode_paths_with_settings("G90\nG28\n", &settings).unwrap();

    assert_eq!(parsed.frame_count, 2);
    assert_eq!(parsed.command_count, 2);
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert!(parsed.segments[0].idle);
    assert_eq!(parsed.segments[0].start, [0.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].end, [0.0, 0.0, 0.0]);
    assert_eq!(parsed.segments[0].feedrate, 1234.0);
    assert_eq!(parsed.max_feedrate, 0.0);
    assert!(parsed.warnings.is_empty());
}

#[test]
fn gcode_custom_cnc_rotation_axes_and_order_match_meshlib_settings() {
    let axis_settings = gcode::GcodeMachineSettings {
        rotation_axes: [[0.0, 0.0, 2.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
        rotation_order: vec![0],
        ..Default::default()
    };
    let axis_parsed =
        gcode::parse_gcode_paths_with_settings("G90\nG0 X1 Y0 Z0\nG1 A90 F600\n", &axis_settings)
            .unwrap();

    assert_eq!(axis_parsed.segments.len(), 21);
    assert!((axis_parsed.segments[20].end[0]).abs() < 1e-9);
    assert!((axis_parsed.segments[20].end[1] - 1.0).abs() < 1e-9);
    assert!((axis_parsed.segments[20].end[2]).abs() < 1e-9);
    assert_eq!(axis_parsed.segments[20].tool_direction_end, [0.0, 0.0, 1.0]);
    assert_eq!(axis_parsed.max_feedrate, 600.0);

    let order_settings = gcode::GcodeMachineSettings {
        rotation_order: vec![2, 0, 2],
        ..Default::default()
    };
    let order_parsed = gcode::parse_gcode_paths_with_settings(
        "G90\nG0 X0 Y1 Z0\nG1 A90 C90 F700\n",
        &order_settings,
    )
    .unwrap();

    assert_eq!(order_parsed.segments.len(), 21);
    assert!((order_parsed.segments[20].end[0] + 1.0).abs() < 1e-9);
    assert!((order_parsed.segments[20].end[1]).abs() < 1e-9);
    assert!((order_parsed.segments[20].end[2]).abs() < 1e-9);
    assert_eq!(order_parsed.max_feedrate, 700.0);
    assert!(order_parsed.warnings.is_empty());
}

#[test]
fn gcode_custom_cnc_rotation_limits_match_meshlib_warning_contract() {
    let settings = gcode::GcodeMachineSettings {
        rotation_limits: [Some([-45.0, 45.0]), None, None],
        ..Default::default()
    };

    let parsed =
        gcode::parse_gcode_paths_with_settings("G90\nG0 X0 Y0 Z1\nG1 A90 F600\n", &settings)
            .unwrap();

    assert_eq!(
        parsed.warnings,
        vec!["frame 2: Error input angle: Going beyond the limits.".to_string()]
    );
    assert_eq!(parsed.segments.len(), 21);
    assert_eq!(parsed.max_feedrate, 600.0);

    let ignored_invalid_limits = gcode::GcodeMachineSettings {
        rotation_limits: [Some([45.0, -45.0]), None, None],
        ..Default::default()
    };
    let no_warning = gcode::parse_gcode_paths_with_settings(
        "G90\nG0 X0 Y0 Z1\nG1 A90 F600\n",
        &ignored_invalid_limits,
    )
    .unwrap();
    assert!(no_warning.warnings.is_empty());
}

#[test]
fn gcode_custom_cnc_rotation_limits_are_clamped_like_meshlib_settings() {
    let settings = gcode::GcodeMachineSettings {
        rotation_limits: [Some([-240.0, 240.0]), Some([-240.0, 0.0]), None],
        ..Default::default()
    };

    let inside_clamped_a =
        gcode::parse_gcode_paths_with_settings("G90\nG0 X0 Y0 Z1\nG1 A180 F600\n", &settings)
            .unwrap();
    assert!(inside_clamped_a.warnings.is_empty());

    let outside_clamped_b =
        gcode::parse_gcode_paths_with_settings("G90\nG0 X0 Y0 Z1\nG1 B90 F700\n", &settings)
            .unwrap();
    assert_eq!(
        outside_clamped_b.warnings,
        vec!["frame 2: Error input angle: Going beyond the limits.".to_string()]
    );
}

#[test]
fn gcode_source_file_workflow_matches_meshlib_supported_formats() {
    let path = std::env::temp_dir().join(format!(
        "zennah-gcode-source-{}-{}.NC",
        std::process::id(),
        "import"
    ));
    let source = "\n; retained comment frame\nG90\n\nG0 X0 Y0 Z0 F3000\nG1 X1 Y2 Z3 F600\n";
    std::fs::write(&path, source).unwrap();

    let frames = gcode::load_gcode_source(&path).unwrap();
    let parsed = gcode::parse_gcode_file_paths(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(
        frames,
        vec![
            "; retained comment frame".to_string(),
            "G90".to_string(),
            "G0 X0 Y0 Z0 F3000".to_string(),
            "G1 X1 Y2 Z3 F600".to_string(),
        ]
    );
    assert_eq!(parsed.frame_count, 4);
    assert_eq!(parsed.command_count, 11);
    assert_eq!(parsed.segments.len(), 2);
    assert_eq!(parsed.segments[0].source_frame_index, 2);
    assert!(parsed.segments[0].idle);
    assert_eq!(parsed.segments[1].source_frame_index, 3);
    assert_eq!(parsed.segments[1].feedrate, 600.0);
}

#[test]
fn gcode_source_file_preserves_meshlib_crlf_frame_carriage_returns() {
    let path = std::env::temp_dir().join(format!(
        "zennah-gcode-source-{}-{}.gcode",
        std::process::id(),
        "crlf"
    ));
    let source = "G90\r\nG1 X1 Y2\r\n\nG1 X3\r\n";
    std::fs::write(&path, source).unwrap();

    let frames = gcode::load_gcode_source(&path).unwrap();
    let parsed = gcode::parse_gcode_file_paths(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(
        frames,
        vec![
            "G90\r".to_string(),
            "G1 X1 Y2\r".to_string(),
            "G1 X3\r".to_string(),
        ]
    );
    assert_eq!(parsed.frame_count, 3);
    assert_eq!(parsed.command_count, 6);
    assert_eq!(parsed.segments.len(), 2);
    assert_eq!(parsed.segments[0].source_frame_index, 1);
    assert_eq!(parsed.segments[0].end, [1.0, 2.0, 0.0]);
    assert_eq!(parsed.segments[1].source_frame_index, 2);
    assert_eq!(parsed.segments[1].end, [3.0, 2.0, 0.0]);
}

#[test]
fn gcode_source_file_export_roundtrips_meshlib_object_gcode_source_frames() {
    let path = std::env::temp_dir().join(format!(
        "zennah-gcode-source-{}-{}.gcode",
        std::process::id(),
        "export"
    ));
    let frames = vec![
        "G90".to_string(),
        "G0 X0 Y0 Z0".to_string(),
        "G1 X1 Y0 F500".to_string(),
    ];

    gcode::write_gcode_source(&frames, &path).unwrap();
    let reloaded = gcode::load_gcode_source(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(reloaded, frames);
}

// MeshLib-generated DifferenceAB output for two unit cubes offset by +1 on X.
// This pins the parity target while Rust exact-difference stitching catches up.
fn meshlib_cube_overlap_difference() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, -1.0],
        [1.0, 1e-9, 1.0],
        [0.0, -1.0, 1.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, -1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [4, 5, 3],
        [4, 3, 2],
        [5, 6, 0],
        [5, 0, 3],
        [7, 2, 1],
        [8, 1, 0],
        [8, 0, 6],
        [8, 6, 9],
        [10, 9, 6],
        [11, 10, 6],
        [12, 6, 5],
        [6, 12, 11],
        [12, 5, 4],
        [13, 4, 2],
        [8, 14, 2],
        [8, 2, 7],
        [1, 8, 7],
        [14, 8, 12],
        [12, 9, 11],
        [9, 12, 8],
        [11, 9, 10],
        [14, 4, 13],
        [4, 14, 12],
        [13, 2, 14],
    ];
    (vertices, faces)
}

const MESHLIB_CUBE_OVERLAP_UNION_VERTICES: usize = 18;
const MESHLIB_CUBE_OVERLAP_UNION_FACES: usize = 32;
const MESHLIB_CUBE_OVERLAP_UNION_SELF_INTERSECTIONS: usize = 13;
const MESHLIB_CUBE_OVERLAP_INTERSECTION_VERTICES: usize = 12;
const MESHLIB_CUBE_OVERLAP_INTERSECTION_FACES: usize = 20;
const MESHLIB_CUBE_OVERLAP_INTERSECTION_SELF_INTERSECTIONS: usize = 0;
const MESHLIB_CUBE_OVERLAP_DIFFERENCE_VERTICES: usize = 15;
const MESHLIB_CUBE_OVERLAP_DIFFERENCE_FACES: usize = 26;
const MESHLIB_CUBE_OVERLAP_DIFFERENCE_SELF_INTERSECTIONS: usize = 11;

#[test]
fn cube_stats_match_python_fixture() {
    let (vertices, faces) = cube();
    let stats = mesh_stats(&vertices, &faces).unwrap();

    assert_eq!(stats.vertex_count, 8);
    assert_eq!(stats.face_count, 12);
    assert_eq!(stats.connected_components, 1);
    assert_eq!(stats.boundary_edge_count, 0);
    assert_eq!(stats.bbox_size, [2.0, 2.0, 2.0]);
    assert!((stats.surface_area_mm2 - 24.0).abs() < 1e-9);
    assert!((stats.volume_mm3 - 8.0).abs() < 1e-9);
}

#[test]
fn prune_small_components_removes_area_below_meshlib_threshold() {
    let (mut vertices, mut faces) = cube();
    vertices.extend([[4.0, 0.0, 0.0], [4.1, 0.0, 0.0], [4.0, 0.1, 0.0]]);
    faces.push([8, 9, 10]);

    let pruned = prune_small_components(&vertices, &faces, 0.5).unwrap();

    assert_eq!(pruned.report.input_component_count, 2);
    assert_eq!(pruned.report.output_component_count, 1);
    assert_eq!(pruned.report.removed_component_count, 1);
    assert_eq!(pruned.report.removed_face_count, 1);
    assert_eq!(pruned.report.removed_vertex_count, 3);
    assert_eq!(pruned.vertices.len(), 8);
    assert_eq!(pruned.faces.len(), 12);
    assert_eq!(pruned.faces.iter().flatten().copied().max(), Some(7));
}

#[test]
fn short_edge_diagnostics_matches_meshlib_critical_length_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.05, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 3], [1, 2, 3]];

    let report = short_edge_diagnostics(&vertices, &faces, -0.05).unwrap();

    assert_eq!(report.critical_length_mm, 0.05);
    assert_eq!(report.edge_count, 5);
    assert_eq!(report.short_edge_count, 1);
    assert_eq!(report.edges[0].edge, [0, 1]);
    assert!((report.edges[0].length_mm - 0.05).abs() < 1e-12);
    assert_eq!(report.min_short_edge_length_mm, Some(0.05));
    assert_eq!(report.max_short_edge_length_mm, Some(0.05));
}

#[test]
fn select_short_edges_matches_meshlib_find_short_edges_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.05, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 3], [1, 2, 3]];

    assert_eq!(
        select_short_edges(&vertices, &faces, 0.05).unwrap(),
        vec![[0, 1]]
    );
}

#[test]
fn select_faces_by_area_matches_meshlib_area_threshold_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [3.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [3.0, 2.0, 0.0],
        [7.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [7.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5], [6, 7, 8]];

    assert_eq!(
        select_faces_by_area(&vertices, &faces, 1.0, "absolute", "less").unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_area(&vertices, &faces, 2.5, "absolute", "greater").unwrap(),
        vec![2]
    );
    assert_eq!(
        select_faces_by_area(&vertices, &faces, 10.0, "percentage", "less").unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_area(&vertices, &faces, 50.0, "percentage", "greater").unwrap(),
        vec![2]
    );
}

#[test]
fn degenerate_face_diagnostics_matches_meshlib_aspect_ratio_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 3.0_f64.sqrt() / 2.0, 0.0],
        [10.0, 0.0, 0.0],
        [0.0, 0.1, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 3, 4]];
    let skinny_ratio =
        meshlib_reference_triangle_aspect_ratio(vertices[0], vertices[3], vertices[4]);

    let report = degenerate_face_diagnostics(&vertices, &faces, skinny_ratio).unwrap();

    assert!((report.critical_aspect_ratio - skinny_ratio).abs() < 1e-12);
    assert_eq!(report.face_count, 2);
    assert_eq!(report.degenerate_face_count, 1);
    assert_eq!(report.faces[0].face_index, 1);
    assert_eq!(report.faces[0].face, [0, 3, 4]);
    assert!((report.faces[0].aspect_ratio - skinny_ratio).abs() < 1e-12);
    assert_eq!(report.min_degenerate_aspect_ratio, Some(skinny_ratio));
    assert_eq!(report.max_degenerate_aspect_ratio, Some(skinny_ratio));
}

#[test]
fn select_degenerate_faces_matches_meshlib_find_degenerate_faces_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 0.001, 0.0],
        [0.5, 0.4, 1.0],
        [3.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [3.5, 0.001, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0], [4, 5, 6]];

    assert_eq!(
        select_degenerate_faces(&vertices, &faces, 100.0, false).unwrap(),
        vec![0, 4]
    );
    assert_eq!(
        select_degenerate_faces(&vertices, &faces, 100.0, true).unwrap(),
        vec![4]
    );
}

#[test]
fn multiple_edge_diagnostics_matches_meshlib_vertex_pair_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.5, -0.5, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]];

    let report = multiple_edge_diagnostics(&vertices, &faces).unwrap();

    assert_eq!(report.edge_count, 7);
    assert_eq!(report.multiple_edge_count, 1);
    assert_eq!(report.edges[0].vertex_pair, [0, 1]);
    assert_eq!(report.edges[0].topology_edge_count, 2);
    assert_eq!(report.edges[0].face_edge_occurrences, 3);
    assert_eq!(report.edges[0].forward_occurrences, 2);
    assert_eq!(report.edges[0].reverse_occurrences, 1);
}

#[test]
fn repair_multiple_edges_splits_duplicate_topology_edges_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.5, -0.5, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]];

    let repaired = repair_multiple_edges(&vertices, &faces).unwrap();

    assert_eq!(repaired.report.input_multiple_edge_count, 1);
    assert_eq!(repaired.report.output_multiple_edge_count, 0);
    assert_eq!(repaired.report.split_edge_count, 1);
    assert_eq!(repaired.report.split_face_count, 1);
    assert_eq!(repaired.report.added_vertex_count, 1);
    assert_eq!(repaired.report.input_face_count, 3);
    assert_eq!(repaired.report.output_face_count, 4);
    assert_eq!(repaired.vertices.len(), 6);
    assert_eq!(repaired.faces.len(), 4);
    assert_eq!(repaired.vertices[5], [0.5, 0.0, 0.0]);
    assert!(repaired.faces.contains(&[0, 5, 4]));
    assert!(repaired.faces.contains(&[5, 1, 4]));
    assert_eq!(
        multiple_edge_diagnostics(&repaired.vertices, &repaired.faces)
            .unwrap()
            .multiple_edge_count,
        0
    );
}

#[test]
fn duplicate_multi_hole_vertices_splits_disconnected_boundary_fans_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 3, 4]];

    let repaired = duplicate_multi_hole_vertices(&vertices, &faces).unwrap();

    assert_eq!(repaired.report.input_multi_hole_vertex_count, 1);
    assert_eq!(repaired.report.output_multi_hole_vertex_count, 0);
    assert_eq!(repaired.report.duplicated_vertex_count, 1);
    assert_eq!(repaired.report.input_vertex_count, 5);
    assert_eq!(repaired.report.output_vertex_count, 6);
    assert_eq!(repaired.report.input_face_count, 2);
    assert_eq!(repaired.report.output_face_count, 2);
    assert_eq!(repaired.vertices[5], [0.0, 0.0, 0.0]);
    assert_eq!(repaired.faces, vec![[0, 1, 2], [5, 3, 4]]);
}

#[test]
fn repair_nonmanifold_edges_removes_excess_edge_faces_like_meshlib_builder() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.5, 0.0, 1.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]];

    let repaired = repair_nonmanifold_edges(&vertices, &faces).unwrap();
    let health = mesh_health(&repaired.vertices, &repaired.faces, false, None, 1e-8).unwrap();

    assert_eq!(repaired.report.input_nonmanifold_edge_count, 1);
    assert_eq!(repaired.report.output_nonmanifold_edge_count, 0);
    assert_eq!(repaired.report.removed_face_count, 1);
    assert_eq!(repaired.report.input_vertex_count, 5);
    assert_eq!(repaired.report.output_vertex_count, 5);
    assert_eq!(repaired.report.input_face_count, 3);
    assert_eq!(repaired.report.output_face_count, 2);
    assert_eq!(repaired.faces, vec![[0, 1, 2], [1, 0, 3]]);
    assert_eq!(health.nonmanifold_edge_count, 0);
}

#[test]
fn duplicate_nonmanifold_vertices_splits_disconnected_closed_fans_like_meshlib_builder() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [-1.0, -1.0, 0.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 1],
        [0, 4, 5],
        [0, 5, 6],
        [0, 6, 4],
    ];

    let repaired = duplicate_nonmanifold_vertices(&vertices, &faces).unwrap();

    assert_eq!(repaired.report.input_nonmanifold_vertex_count, 1);
    assert_eq!(repaired.report.output_nonmanifold_vertex_count, 0);
    assert_eq!(repaired.report.duplicated_vertex_count, 1);
    assert_eq!(repaired.report.input_vertex_count, 7);
    assert_eq!(repaired.report.output_vertex_count, 8);
    assert_eq!(repaired.report.input_face_count, 6);
    assert_eq!(repaired.report.output_face_count, 6);
    assert_eq!(repaired.vertices[7], [0.0, 0.0, 0.0]);
    assert_eq!(
        repaired.faces,
        vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [7, 4, 5],
            [7, 5, 6],
            [7, 6, 4],
        ]
    );
}

#[test]
fn duplicate_nonmanifold_vertices_splits_repeated_neighbor_path_like_meshlib_builder() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [-1.0, -1.0, 0.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 1],
        [0, 1, 4],
        [0, 4, 5],
        [0, 5, 1],
    ];

    let repaired = duplicate_nonmanifold_vertices(&vertices, &faces).unwrap();

    assert_eq!(repaired.report.input_nonmanifold_vertex_count, 2);
    assert_eq!(repaired.report.output_nonmanifold_vertex_count, 0);
    assert_eq!(repaired.report.duplicated_vertex_count, 2);
    assert_eq!(repaired.report.input_vertex_count, 6);
    assert_eq!(repaired.report.output_vertex_count, 8);
    assert_eq!(repaired.vertices[6], [0.0, 0.0, 0.0]);
    assert_eq!(repaired.vertices[7], [1.0, 0.0, 0.0]);
    assert_eq!(
        repaired.faces,
        vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [6, 7, 4],
            [6, 4, 5],
            [6, 5, 7],
        ]
    );
}

#[test]
fn duplicate_nonmanifold_vertices_respects_meshlib_face_region_scope() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [-1.0, -1.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
        [2.0, -1.0, 0.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 1],
        [0, 4, 5],
        [0, 5, 6],
        [0, 6, 4],
        [0, 7, 8],
        [0, 8, 9],
        [0, 9, 7],
    ];

    let repaired =
        duplicate_nonmanifold_vertices_in_region(&vertices, &faces, &[3, 4, 5, 6, 7, 8]).unwrap();

    assert_eq!(repaired.report.input_nonmanifold_vertex_count, 1);
    assert_eq!(repaired.report.output_nonmanifold_vertex_count, 0);
    assert_eq!(repaired.report.duplicated_vertex_count, 1);
    assert_eq!(repaired.report.input_vertex_count, 10);
    assert_eq!(repaired.report.output_vertex_count, 11);
    assert_eq!(repaired.vertices[10], [0.0, 0.0, 0.0]);
    assert_eq!(
        repaired.faces,
        vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [0, 4, 5],
            [0, 5, 6],
            [0, 6, 4],
            [10, 7, 8],
            [10, 8, 9],
            [10, 9, 7],
        ]
    );
}

#[test]
fn duplicate_nonmanifold_vertex_ids_uses_meshlib_last_valid_vertex_for_partial_triangulation() {
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 1],
        [0, 4, 5],
        [0, 5, 6],
        [0, 6, 4],
    ];

    let repaired =
        duplicate_nonmanifold_vertex_ids_with_last_valid_vertex(&faces, None, Some(20)).unwrap();

    assert_eq!(repaired.input_nonmanifold_vertex_count, 1);
    assert_eq!(repaired.output_nonmanifold_vertex_count, 0);
    assert_eq!(repaired.duplications.len(), 1);
    assert_eq!(repaired.duplications[0].src_vertex, 0);
    assert_eq!(repaired.duplications[0].dup_vertex, 21);
    assert_eq!(
        repaired.faces,
        vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 1],
            [21, 4, 5],
            [21, 5, 6],
            [21, 6, 4],
        ]
    );
}

#[test]
fn duplicate_nonmanifold_vertex_ids_matches_meshlib_path_orientation_single_pass() {
    let faces = vec![
        [2, 0, 3],
        [1, 2, 0],
        [1, 0, 4],
        [4, 0, 3],
        [0, 3, 1],
        [4, 0, 2],
    ];

    let repaired =
        duplicate_nonmanifold_vertex_ids_with_last_valid_vertex(&faces, None, None).unwrap();

    assert_eq!(
        repaired.duplications,
        vec![
            VertexDuplication {
                src_vertex: 0,
                dup_vertex: 5,
            },
            VertexDuplication {
                src_vertex: 0,
                dup_vertex: 6,
            },
            VertexDuplication {
                src_vertex: 0,
                dup_vertex: 7,
            },
            VertexDuplication {
                src_vertex: 1,
                dup_vertex: 8,
            },
            VertexDuplication {
                src_vertex: 2,
                dup_vertex: 9,
            },
            VertexDuplication {
                src_vertex: 3,
                dup_vertex: 10,
            },
            VertexDuplication {
                src_vertex: 3,
                dup_vertex: 11,
            },
            VertexDuplication {
                src_vertex: 4,
                dup_vertex: 12,
            },
        ]
    );
    assert_eq!(
        repaired.faces,
        vec![
            [2, 5, 3],
            [1, 9, 0],
            [1, 0, 4],
            [12, 7, 11],
            [6, 10, 8],
            [4, 0, 9],
        ]
    );
}

#[test]
fn not_smooth_face_diagnostics_matches_meshlib_neighbor_angle_rule() {
    let vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1],
        [4, 6, 5],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];

    let report = not_smooth_face_diagnostics(&vertices, &faces, 0.3).unwrap();

    assert_eq!(report.face_count, 12);
    assert_eq!(report.not_smooth_face_count, 2);
    assert_eq!(
        report
            .faces
            .iter()
            .map(|face| face.face_index)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert!((report.faces[0].angle_delta_radians - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!((report.faces[1].angle_delta_radians - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert_eq!(
        select_not_smooth_faces(&vertices, &faces, 0.3).unwrap(),
        vec![2, 3]
    );
}

#[test]
fn select_not_smooth_faces_matches_meshlib_neighbor_angle_rule() {
    let vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1],
        [4, 6, 5],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];

    assert_eq!(
        select_not_smooth_faces(&vertices, &faces, 0.3).unwrap(),
        vec![2, 3]
    );
}

#[test]
fn find_disoriented_faces_matches_meshlib_ray_count_contract() {
    let vertices = vec![
        [1.0, 1.0, 1.0],
        [-1.0, -1.0, 1.0],
        [-1.0, 1.0, -1.0],
        [1.0, -1.0, -1.0],
    ];
    let faces = vec![[0, 1, 2], [0, 1, 3], [0, 3, 2], [1, 2, 3]];

    assert_eq!(
        find_disoriented_faces(
            &vertices,
            &faces,
            FindDisorientationRayMode::Shallowest,
            1e-8
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        find_disoriented_faces(&vertices, &faces, FindDisorientationRayMode::Positive, 1e-8)
            .unwrap(),
        vec![0]
    );
    assert_eq!(
        find_disoriented_faces(&vertices, &faces, FindDisorientationRayMode::Both, 1e-8).unwrap(),
        vec![0]
    );
}

#[test]
fn flip_normals_matches_meshlib_full_orientation_flip_contract() {
    let vertices = vec![
        [1.0, 1.0, 1.0],
        [-1.0, -1.0, 1.0],
        [-1.0, 1.0, -1.0],
        [1.0, -1.0, -1.0],
    ];
    let faces = vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];

    let flipped = flip_normals(&vertices, &faces).unwrap();
    let expected = vec![[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
    assert_eq!(flipped, expected);

    let input_faces = crate::mesh::validate_faces(&faces, vertices.len()).unwrap();
    let output_faces = crate::mesh::validate_faces(&flipped, vertices.len()).unwrap();
    assert_eq!(
        crate::mesh::signed_volume(&vertices, &output_faces),
        -crate::mesh::signed_volume(&vertices, &input_faces)
    );
}

#[test]
fn make_delone_edge_flips_matches_meshlib_quadrangle_diagonal_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let result = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();

    assert_eq!(result.flips_done, 1);
    assert_eq!(result.mesh.vertices, vertices);
    assert_eq!(result.mesh.faces, vec![[1, 3, 0], [3, 1, 2]]);
}

#[test]
fn make_delone_edge_flips_honors_meshlib_not_flippable_constraint() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let protected = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: vec![[2, 0]],
            vert_region: None,
        },
    )
    .unwrap();

    assert_eq!(protected.flips_done, 0);
    assert_eq!(protected.mesh.faces, faces);
}

#[test]
fn make_delone_edge_flips_honors_meshlib_vert_region_constraint() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
        [10.0, 10.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let blocked = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: Some(vec![4]),
        },
    )
    .unwrap();
    let allowed_by_new_diagonal_endpoint = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: Some(vec![1]),
        },
    )
    .unwrap();

    assert_eq!(blocked.flips_done, 0);
    assert_eq!(blocked.mesh.faces, faces);
    assert_eq!(allowed_by_new_diagonal_endpoint.flips_done, 1);
    assert_eq!(
        allowed_by_new_diagonal_endpoint.mesh.faces,
        vec![[1, 3, 0], [3, 1, 2]]
    );
}

#[test]
fn make_delone_edge_flips_honors_meshlib_max_deviation_after_flip() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 1.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let unconstrained = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();
    let constrained = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: Some(0.1),
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();

    assert_eq!(unconstrained.flips_done, 1);
    assert_eq!(unconstrained.mesh.faces, vec![[1, 3, 0], [3, 1, 2]]);
    assert_eq!(constrained.flips_done, 0);
    assert_eq!(constrained.mesh.faces, faces);
}

#[test]
fn make_delone_edge_flips_honors_meshlib_max_angle_change() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 1.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let unconstrained = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: None,
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();
    let constrained = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: Some(0.5),
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();

    assert_eq!(unconstrained.flips_done, 1);
    assert_eq!(unconstrained.mesh.faces, vec![[1, 3, 0], [3, 1, 2]]);
    assert_eq!(constrained.flips_done, 0);
    assert_eq!(constrained.mesh.faces, faces);
}

#[test]
fn make_delone_edge_flips_honors_meshlib_critical_tri_aspect_ratio() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 1.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];

    let angle_constrained = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: Some(0.5),
            critical_tri_aspect_ratio: None,
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();
    let aspect_critical = make_delone_edge_flips(
        &vertices,
        &faces,
        MakeDeloneEdgeFlipsOptions {
            num_iters: 1,
            region_faces: None,
            max_deviation_after_flip: None,
            max_angle_change: Some(0.5),
            critical_tri_aspect_ratio: Some(2.0),
            not_flippable_edges: Vec::new(),
            vert_region: None,
        },
    )
    .unwrap();

    assert_eq!(angle_constrained.flips_done, 0);
    assert_eq!(angle_constrained.mesh.faces, faces);
    assert_eq!(aspect_critical.flips_done, 1);
    assert_eq!(aspect_critical.mesh.faces, vec![[1, 3, 0], [3, 1, 2]]);
}

#[test]
fn crease_edge_diagnostics_matches_meshlib_dihedral_cos_contract() {
    let (vertices, faces) = cube();

    let report = crease_edge_diagnostics(&vertices, &faces, 0.3).unwrap();

    assert_eq!(report.edge_count, 18);
    assert_eq!(report.crease_edge_count, 12);
    assert!(report.edges.iter().any(|edge| edge.edge == [0, 1]));
    assert!(!report.edges.iter().any(|edge| edge.edge == [0, 2]));
    assert!(report
        .edges
        .iter()
        .all(|edge| edge.dihedral_cosine.abs() < 1e-12));
}

#[test]
fn crease_edge_diagnostics_filters_short_components_like_meshlib_filter_crease_edges() {
    let (mut vertices, mut faces) = cube();
    let offset = vertices.len() as i64;
    vertices.extend([
        [4.0, 0.0, 0.0],
        [4.1, 0.0, 0.0],
        [4.05, 0.08660254037844387, 0.0],
        [4.05, 0.02886751345948129, 0.08164965809277261],
    ]);
    faces.extend(
        [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]
            .into_iter()
            .map(|face| [face[0] + offset, face[1] + offset, face[2] + offset]),
    );

    let report = crease_edge_diagnostics_with_filter(
        &vertices,
        &faces,
        0.3,
        CreaseEdgeFilterOptions {
            min_component_length_mm: Some(1.0),
            min_branch_length_mm: None,
        },
    )
    .unwrap();

    assert_eq!(report.raw_crease_edge_count, 18);
    assert_eq!(report.crease_edge_count, 12);
    assert!(report
        .edges
        .iter()
        .all(|edge| edge.edge[0] < 8 && edge.edge[1] < 8));
}

#[test]
fn crease_edge_diagnostics_filters_short_branches_like_meshlib_filter_crease_edges() {
    let (mut vertices, mut faces) = cube();
    vertices.extend([[-1.2, -1.0, -1.0], [-1.1, -0.9, -1.0], [-1.1, -1.0, -0.9]]);
    faces.extend([[0, 8, 9], [8, 0, 10]]);

    let unfiltered = crease_edge_diagnostics(&vertices, &faces, 0.3).unwrap();
    assert_eq!(unfiltered.raw_crease_edge_count, 13);
    assert!(unfiltered.edges.iter().any(|edge| edge.edge == [0, 8]));

    let report = crease_edge_diagnostics_with_filter(
        &vertices,
        &faces,
        0.3,
        CreaseEdgeFilterOptions {
            min_component_length_mm: None,
            min_branch_length_mm: Some(0.5),
        },
    )
    .unwrap();

    assert_eq!(report.raw_crease_edge_count, 13);
    assert_eq!(report.crease_edge_count, 12);
    assert!(!report.edges.iter().any(|edge| edge.edge == [0, 8]));
    assert!(report.edges.iter().any(|edge| edge.edge == [0, 1]));
}

#[test]
fn crease_repair_plan_diagnostics_selects_meshlib_fix_faces_around_inverted_patch() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 0, 3]];

    let report = crease_repair_plan_diagnostics(
        &vertices,
        &faces,
        std::f64::consts::PI * 175.0 / 180.0,
        1e3,
    )
    .unwrap();

    assert_eq!(report.crease_edge_count, 1);
    assert_eq!(report.planned_region_count, 1);
    assert_eq!(report.planned_face_count, 2);
    assert_eq!(report.regions[0].crease_edge, [0, 1]);
    assert_eq!(report.regions[0].selected_face_indices, vec![0, 1]);
}

#[test]
fn fix_mesh_creases_retriangulates_flipped_cube_patch_like_meshlib() {
    let vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 3, 2],
        [0, 2, 1],
        [4, 6, 5],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];

    let repaired = fix_mesh_creases(
        &vertices,
        &faces,
        std::f64::consts::PI * 175.0 / 180.0,
        1e3,
        10,
    )
    .unwrap();

    assert_eq!(repaired.report.input_crease_edge_count, 1);
    assert_eq!(repaired.report.output_crease_edge_count, 0);
    assert_eq!(repaired.report.repaired_region_count, 1);
    assert_eq!(repaired.report.removed_face_count, 1);
    assert_eq!(repaired.report.added_face_count, 1);
    assert_eq!(repaired.report.input_face_count, 12);
    assert_eq!(repaired.report.output_face_count, 12);
    assert_eq!(repaired.vertices, vertices);
    assert_eq!(
        crease_edge_diagnostics(
            &repaired.vertices,
            &repaired.faces,
            std::f64::consts::PI * 175.0 / 180.0,
        )
        .unwrap()
        .crease_edge_count,
        0
    );
    assert!(
        mesh_health(&repaired.vertices, &repaired.faces, false, None, 1e-8)
            .unwrap()
            .is_closed
    );
}

#[test]
fn meshlib_cube_overlap_difference_fixture_matches_reference_envelope() {
    let (vertices, faces) = meshlib_cube_overlap_difference();
    let stats = mesh_stats(&vertices, &faces).unwrap();
    let health = mesh_health(&vertices, &faces, true, None, 1e-8).unwrap();

    assert_eq!(stats.vertex_count, MESHLIB_CUBE_OVERLAP_DIFFERENCE_VERTICES);
    assert_eq!(stats.face_count, MESHLIB_CUBE_OVERLAP_DIFFERENCE_FACES);
    assert_eq!(stats.connected_components, 1);
    assert_eq!(stats.boundary_edge_count, 0);
    assert_eq!(stats.bbox_min, [-1.0, -1.0, -1.0]);
    assert_eq!(stats.bbox_max, [1.0, 1.0, 1.0]);
    assert!((stats.surface_area_mm2 - 24.0).abs() < 1e-9);
    assert!((stats.volume_mm3 - 4.0).abs() < 1e-9);
    assert!(health.is_closed);
    assert_eq!(health.holes_count, 0);
    assert_eq!(health.boundary_edge_count, 0);
    assert_eq!(health.nonmanifold_edge_count, 0);
    assert_eq!(
        health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_DIFFERENCE_SELF_INTERSECTIONS)
    );
}

#[test]
fn exact_boolean_cube_overlap_promotes_paired_coplanar_candidate_to_meshlib_envelope() {
    let (source_vertices, source_faces) = cube();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
        .collect::<Vec<_>>();

    let result = exact_boolean_from_meshes(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &source_faces,
        ExactBooleanOperation::Union,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.parity_ready);
    assert!(result.diagnostics.stitch_compatible);
    assert_eq!(result.diagnostics.stitch_unmatched_first_edges, 0);
    assert_eq!(result.diagnostics.stitch_unmatched_second_edges, 0);
    assert_eq!(result.diagnostics.stitch_cut_path_length_mismatches, 0);
    assert!(result.diagnostics.first_prepare_part_dividable);
    assert!(result.diagnostics.second_prepare_part_dividable);
    assert_eq!(result.diagnostics.first_cut_path_overlap_components, 0);
    assert_eq!(result.diagnostics.second_cut_path_overlap_components, 0);
    assert!(result.diagnostics.first_skipped_source_faces.is_empty());
    assert!(result.diagnostics.second_skipped_source_faces.is_empty());
    assert!(result.diagnostics.requires_topology_splice);
    assert!(result.diagnostics.coplanar_overlap_pairs > 0);
    assert!(result.diagnostics.coplanar_overlap_region_edges > 0);
    assert!(result.diagnostics.coplanar_overlap_area > 0.0);
    assert_eq!(
        result.diagnostics.coplanar_overlap_contours,
        result.diagnostics.coplanar_overlap_pairs
    );
    assert_eq!(
        result.diagnostics.coplanar_overlap_contour_edges,
        result.diagnostics.coplanar_overlap_region_edges
    );
    assert!(result.diagnostics.coplanar_cut_trial_contours > 0);
    assert!(result.diagnostics.coplanar_cut_trial_contour_edges > 0);
    assert!(result.diagnostics.coplanar_cut_trial_first_cut_edges > 0);
    assert!(result.diagnostics.coplanar_cut_trial_second_cut_edges > 0);
    assert_eq!(result.diagnostics.paired_coplanar_cut_trial_contours, 2);
    assert_eq!(
        result.diagnostics.paired_coplanar_cut_trial_contour_edges,
        16
    );
    assert_eq!(
        result.diagnostics.paired_coplanar_cut_trial_first_cut_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_cut_trial_second_cut_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_stitch_cut_path_length_mismatches,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_stitch_unmatched_first_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_stitch_unmatched_second_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_duplicate_first_path_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_duplicate_second_path_edges,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_stitch_compatible
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_prepare_part_dividable
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_prepare_part_dividable
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_cut_path_side_components,
        [1, 1]
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_cut_path_side_components,
        [1, 1]
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_cut_path_overlap_components,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_cut_path_overlap_components,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_result_cut_paths_complete
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_prepare_cut_complete
    );
    assert!(result.diagnostics.paired_coplanar_candidate_output_faces > 0);
    assert!(result.diagnostics.paired_coplanar_candidate_output_volume > 0.0);
    assert_eq!(
        result.diagnostics.paired_coplanar_candidate_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_nonmanifold_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_duplicate_output_faces,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_preserves_active_volume
    );
    assert!((result.diagnostics.paired_coplanar_candidate_output_volume - 12.0).abs() < 1e-6);
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_active_volume_delta
            .abs()
            < 1e-6
    );
    assert!(!result.diagnostics.coplanar_cut_trial_accepted);
    assert!(result
        .diagnostics
        .coplanar_cut_trial_first_skipped_faces
        .is_empty());
    assert!(result
        .diagnostics
        .coplanar_cut_trial_second_skipped_faces
        .is_empty());
    assert_eq!(result.diagnostics.stitched_output_edges, 16);
    assert_eq!(result.diagnostics.stitched_output_edges_with_two_faces, 16);
    assert_eq!(result.diagnostics.stitched_output_edges_needing_splice, 0);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_mapped_contour_edges, 8);
    assert_eq!(
        [
            result.diagnostics.meshlib_topology_base_faces,
            result.diagnostics.meshlib_topology_incoming_faces,
            result.assembly.prepare_first_faces.len(),
            result.assembly.prepare_second_faces.len(),
            result.diagnostics.meshlib_topology_selected_first_faces,
            result.diagnostics.meshlib_topology_selected_second_faces,
            result.diagnostics.meshlib_topology_first_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_second_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_duplicate_first_source_faces,
            result
                .diagnostics
                .meshlib_topology_duplicate_second_source_faces,
        ],
        [20, 20, 20, 20, 30, 14, 8, 4, 20, 4]
    );
    assert_eq!(
        (
            result.diagnostics.meshlib_topology_raw_selected_faces,
            result
                .diagnostics
                .meshlib_topology_same_oriented_overlap_faces,
            result.diagnostics.meshlib_topology_boundary_misses,
            result
                .diagnostics
                .meshlib_topology_coplanar_selection_delta_faces,
        ),
        ([20, 20], [16, 16], [[0, 0], [8, 8]], [10, -6])
    );
    assert_eq!(result.diagnostics.meshlib_topology_missing_base_edges, 0);
    assert_eq!(
        result.diagnostics.meshlib_topology_missing_incoming_edges,
        8
    );
    assert_eq!(result.diagnostics.meshlib_topology_direction_mismatches, 0);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_mapped_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_missing_stitch_contour_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_synthetic_stitch_contour_edges,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_direction_mismatches,
        0
    );
    assert!(!result.diagnostics.meshlib_topology_stitch_metadata_ready);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_unmaterialized_stitch_contour_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_synthetic_stitch_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_direction_mismatches,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_ready
    );
    assert_eq!(
        result.diagnostics.meshlib_topology_record_rewrite_commands,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_blocked_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_synthetic_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_direction_mismatches,
        0
    );
    assert!(result.diagnostics.meshlib_topology_record_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_open_stitch_paths, 0);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_updates,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_blocked_updates,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_ready
    );
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_near_stitch_update_commands,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_applied,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_start,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_end,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_previous_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_next_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_origin_mismatches,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_previous_left_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_next_right_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_other,
        ],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    let near_stitch_failed_details = &result
        .diagnostics
        .meshlib_topology_near_stitch_failed_details;
    assert_eq!(near_stitch_failed_details.len(), 0);
    assert_eq!(
        near_stitch_failed_details.len(),
        result
            .diagnostics
            .meshlib_topology_near_stitch_updates_failed
    );
    assert!(near_stitch_failed_details
        .iter()
        .all(|detail| detail.endpoint.is_some()
            && detail.candidate_diagnostics.is_some()
            && !detail.error.is_empty()));
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_record_rewrite_applied_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_closed_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_sources,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_other_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_prepared_synthetic_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_translated_face_records,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_synthetic_sides,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_exported_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_failed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_non_triangular_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_left_ring_not_closed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_missing_origin_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_other_failed_faces,
        ],
        [16, 0, 0, 0, 0, 0, 8, 5, 8, 44, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        (
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_changed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_ready,
        ),
        (true, true)
    );
    let prepared_base_rewrite = result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    assert_eq!(
        [
            prepared_base_rewrite.prepared_faces,
            prepared_base_rewrite.prepared_vertices,
            prepared_base_rewrite.virtual_vertices,
            prepared_base_rewrite.prepared_face_sources,
            prepared_base_rewrite.applied_commands,
            prepared_base_rewrite.failed_commands,
            prepared_base_rewrite.near_stitch_updates_applied,
            prepared_base_rewrite.near_stitch_updates_failed,
            prepared_base_rewrite.exported_faces,
            prepared_base_rewrite.export_failed_faces,
        ],
        [20, 20, 0, 20, 16, 0, 0, 0, 40, 0]
    );
    assert!(prepared_base_rewrite.ready_for_export);
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_near_stitch_target_left_closures,
            prepared_base_rewrite.record_rewrite_near_stitch_target_right_closures,
        ),
        (1, 0)
    );
    assert_eq!(
        [
            prepared_base_rewrite.record_failed_missing_targets,
            prepared_base_rewrite.record_failed_closed_targets,
            prepared_base_rewrite.record_failed_missing_sources,
            prepared_base_rewrite.record_failed_other_commands,
            prepared_base_rewrite.translated_copied_edge_records,
            prepared_base_rewrite.translated_copied_face_records,
            prepared_base_rewrite.failed_copied_edge_records,
            prepared_base_rewrite.refreshed_face_records,
            prepared_base_rewrite.near_stitch_failed_start,
            prepared_base_rewrite.near_stitch_failed_end,
            prepared_base_rewrite.near_stitch_missing_previous_edges,
            prepared_base_rewrite.near_stitch_missing_next_edges,
            prepared_base_rewrite.near_stitch_origin_mismatches,
            prepared_base_rewrite.near_stitch_previous_left_faces,
            prepared_base_rewrite.near_stitch_next_right_faces,
            prepared_base_rewrite.near_stitch_failed_other,
            prepared_base_rewrite.export_non_triangular_faces,
            prepared_base_rewrite.export_left_ring_not_closed_faces,
            prepared_base_rewrite.export_missing_origin_faces,
            prepared_base_rewrite.export_face_record_left_mismatch_faces,
            prepared_base_rewrite.export_face_left_ring_mismatch_faces,
            prepared_base_rewrite.export_other_failed_faces,
        ],
        [0, 0, 0, 0, 76, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        prepared_base_rewrite.near_stitch_failed_details.len(),
        prepared_base_rewrite.near_stitch_updates_failed
    );
    assert!(prepared_base_rewrite.near_stitch_failed_details.is_empty());
    let rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_stats
        .as_ref()
        .expect("rewrite export stats");
    assert_eq!(rewrite_stats.vertex_count, 24);
    assert_eq!(rewrite_stats.face_count, 44);
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_health
        .is_some());
    let packed_rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_stats
        .as_ref()
        .expect("packed rewrite export stats");
    assert_eq!(packed_rewrite_stats.vertex_count, 24);
    assert_eq!(packed_rewrite_stats.face_count, 44);
    assert_ne!(
        packed_rewrite_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_UNION_VERTICES
    );
    assert_ne!(
        packed_rewrite_stats.face_count,
        MESHLIB_CUBE_OVERLAP_UNION_FACES
    );
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_health
        .is_some());
    assert!(result.diagnostics.topology_splice_ready);
    assert_eq!(result.diagnostics.topology_splice_non_manifold_edges, 0);
    assert!(result.diagnostics.topology_splice_apply_ready);
    assert_eq!(
        result.diagnostics.topology_splice_verified_boundary_edges,
        0
    );
    assert_eq!(result.diagnostics.topology_splice_blocked_edges, 0);
    assert_eq!(result.diagnostics.topology_splice_failed_edges, 0);
    assert!(!result.assembly.stitched_edge_paths.is_empty());
    assert!(result.topology_splice_apply_plan.stitched_paths > 0);
    assert_eq!(result.topology_splice_apply_plan.verified_boundary_paths, 2);
    assert_eq!(result.topology_splice_apply_plan.blocked_paths, 0);
    assert_eq!(result.topology_splice_apply_plan.failed_paths, 0);
    assert_eq!(result.diagnostics.topology_splice_synthetic_side_edges, 0);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_materialized_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_materialization_failed_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_duplicate_output_face_groups,
        0
    );
    assert_eq!(result.diagnostics.topology_splice_duplicate_output_faces, 0);
    assert!(result.diagnostics.first_cut_edges >= 3);
    assert!(result.diagnostics.second_cut_edges >= 3);
    assert!(result.diagnostics.result_cut_paths_complete);
    assert_eq!(
        result.diagnostics.result_cut_mapped_paths,
        result.diagnostics.result_cut_paths
    );
    assert_eq!(
        result.diagnostics.result_cut_mapped_path_edges,
        result.diagnostics.result_cut_path_edges
    );
    assert_eq!(
        result.diagnostics.result_cut_mapped_closed_paths,
        result.diagnostics.result_cut_closed_paths
    );
    assert!(result.diagnostics.output_mesh_health.is_closed);
    assert_eq!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(0)
    );
    assert_ne!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_UNION_SELF_INTERSECTIONS),
        "the promoted union candidate is envelope-ready but not MeshLib topology-parity-ready"
    );
    assert!(
        result
            .diagnostics
            .output_mesh_health
            .self_intersections_available
    );
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 24);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 44);
    assert_ne!(
        result.diagnostics.output_mesh_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_UNION_VERTICES
    );
    assert_ne!(
        result.diagnostics.output_mesh_stats.face_count,
        MESHLIB_CUBE_OVERLAP_UNION_FACES
    );
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 1);
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 0);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        0
    );
    assert_eq!(result.topology_splice_apply_plan.exported_boundary_edges, 0);
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.topology_splice_exported_faces, 44);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_before_materialization,
        66
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_after_materialization,
        66
    );
    assert_eq!(
        result.diagnostics.topology_splice_deleted_synthetic_edges,
        0
    );
    assert!(
        (result.diagnostics.output_mesh_stats.volume_mm3 - 12.0).abs() < 1e-3,
        "the stored MeshLib cube-overlap union volume is 12 mm3"
    );
    assert!(
        (result.diagnostics.output_mesh_stats.surface_area_mm2 - 32.0).abs() < 1e-6,
        "the MeshLib-style cube-overlap union envelope is a 3x2x2 box"
    );
}

#[test]
fn exact_boolean_cube_overlap_intersection_promotes_paired_coplanar_candidate() {
    let (source_vertices, source_faces) = cube();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
        .collect::<Vec<_>>();

    let result = exact_boolean_from_meshes(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &source_faces,
        ExactBooleanOperation::Intersection,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.parity_ready);
    assert!(result.diagnostics.stitch_compatible);
    assert_eq!(result.diagnostics.stitch_unmatched_first_edges, 0);
    assert_eq!(result.diagnostics.stitch_unmatched_second_edges, 0);
    assert!(result.diagnostics.first_prepare_part_dividable);
    assert!(result.diagnostics.second_prepare_part_dividable);
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_preserves_active_volume
    );
    assert_eq!(
        result.diagnostics.paired_coplanar_candidate_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_nonmanifold_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_duplicate_output_faces,
        0
    );
    assert!(result.diagnostics.output_mesh_health.is_closed);
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 0);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        0
    );
    assert_eq!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(0)
    );
    assert_eq!(
        result.diagnostics.output_mesh_health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_INTERSECTION_SELF_INTERSECTIONS),
        "the promoted intersection candidate now matches the MeshLib self-intersection count"
    );
    assert!(
        result
            .diagnostics
            .output_mesh_health
            .self_intersections_available
    );
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 1);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_mapped_contour_edges, 8);
    assert_eq!(result.diagnostics.meshlib_topology_missing_base_edges, 8);
    assert_eq!(
        result.diagnostics.meshlib_topology_missing_incoming_edges,
        0
    );
    assert_eq!(result.diagnostics.meshlib_topology_direction_mismatches, 0);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_mapped_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_missing_stitch_contour_edges,
        0
    );
    assert_eq!(
        [
            result.diagnostics.meshlib_topology_base_faces,
            result.diagnostics.meshlib_topology_incoming_faces,
            result.assembly.prepare_first_faces.len(),
            result.assembly.prepare_second_faces.len(),
            result.diagnostics.meshlib_topology_selected_first_faces,
            result.diagnostics.meshlib_topology_selected_second_faces,
            result.diagnostics.meshlib_topology_first_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_second_source_face_groups,
            result
                .diagnostics
                .meshlib_topology_duplicate_first_source_faces,
            result
                .diagnostics
                .meshlib_topology_duplicate_second_source_faces,
        ],
        [16, 16, 16, 16, 22, 6, 6, 2, 12, 4]
    );
    assert_eq!(
        (
            result.diagnostics.meshlib_topology_raw_selected_faces,
            result
                .diagnostics
                .meshlib_topology_same_oriented_overlap_faces,
            result.diagnostics.meshlib_topology_boundary_misses,
            result
                .diagnostics
                .meshlib_topology_coplanar_selection_delta_faces,
        ),
        ([16, 16], [16, 16], [[0, 0], [8, 8]], [6, -10])
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_synthetic_stitch_contour_edges,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_direction_mismatches,
        0
    );
    assert!(!result.diagnostics.meshlib_topology_stitch_metadata_ready);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_stitch_contour_edges,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_unmaterialized_stitch_contour_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_materialized_synthetic_stitch_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_direction_mismatches,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_stitch_materialization_ready
    );
    assert_eq!(
        result.diagnostics.meshlib_topology_record_rewrite_commands,
        16
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_blocked_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_synthetic_sides,
        8
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_record_rewrite_direction_mismatches,
        0
    );
    assert!(result.diagnostics.meshlib_topology_record_rewrite_ready);
    assert_eq!(result.diagnostics.meshlib_topology_open_stitch_paths, 0);
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_updates,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_blocked_updates,
        0
    );
    assert!(
        result
            .diagnostics
            .meshlib_topology_open_stitch_near_edge_ready
    );
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_near_stitch_update_commands,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_applied,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_start,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_end,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_previous_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_missing_next_edges,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_origin_mismatches,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_previous_left_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_next_right_faces,
            result
                .diagnostics
                .meshlib_topology_near_stitch_updates_failed_other,
        ],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    let near_stitch_failed_details = &result
        .diagnostics
        .meshlib_topology_near_stitch_failed_details;
    assert_eq!(near_stitch_failed_details.len(), 0);
    assert_eq!(
        near_stitch_failed_details.len(),
        result
            .diagnostics
            .meshlib_topology_near_stitch_updates_failed
    );
    assert!(near_stitch_failed_details
        .iter()
        .all(|detail| detail.endpoint.is_some()
            && detail.candidate_diagnostics.is_some()
            && !detail.error.is_empty()));
    assert_eq!(
        [
            result
                .diagnostics
                .meshlib_topology_record_rewrite_applied_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_closed_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_missing_sources,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_failed_other_commands,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_prepared_synthetic_targets,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_translated_face_records,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_synthetic_sides,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_exported_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_failed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_non_triangular_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_left_ring_not_closed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_missing_origin_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_other_failed_faces,
        ],
        [16, 0, 0, 0, 0, 0, 0, 11, 8, 28, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        (
            result
                .diagnostics
                .meshlib_topology_record_rewrite_export_changed_faces,
            result
                .diagnostics
                .meshlib_topology_record_rewrite_apply_ready,
        ),
        (true, true)
    );
    let prepared_base_rewrite = result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    assert_eq!(
        [
            prepared_base_rewrite.prepared_faces,
            prepared_base_rewrite.prepared_vertices,
            prepared_base_rewrite.virtual_vertices,
            prepared_base_rewrite.prepared_face_sources,
            prepared_base_rewrite.applied_commands,
            prepared_base_rewrite.failed_commands,
            prepared_base_rewrite.near_stitch_updates_applied,
            prepared_base_rewrite.near_stitch_updates_failed,
            prepared_base_rewrite.exported_faces,
            prepared_base_rewrite.export_failed_faces,
        ],
        [16, 16, 0, 16, 16, 0, 0, 0, 32, 0]
    );
    assert!(prepared_base_rewrite.ready_for_export);
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_near_stitch_target_left_closures,
            prepared_base_rewrite.record_rewrite_near_stitch_target_right_closures,
        ),
        (5, 0)
    );
    assert_eq!(
        [
            prepared_base_rewrite.record_failed_missing_targets,
            prepared_base_rewrite.record_failed_closed_targets,
            prepared_base_rewrite.record_failed_missing_sources,
            prepared_base_rewrite.record_failed_other_commands,
            prepared_base_rewrite.translated_copied_edge_records,
            prepared_base_rewrite.translated_copied_face_records,
            prepared_base_rewrite.failed_copied_edge_records,
            prepared_base_rewrite.refreshed_face_records,
            prepared_base_rewrite.near_stitch_failed_start,
            prepared_base_rewrite.near_stitch_failed_end,
            prepared_base_rewrite.near_stitch_missing_previous_edges,
            prepared_base_rewrite.near_stitch_missing_next_edges,
            prepared_base_rewrite.near_stitch_origin_mismatches,
            prepared_base_rewrite.near_stitch_previous_left_faces,
            prepared_base_rewrite.near_stitch_next_right_faces,
            prepared_base_rewrite.near_stitch_failed_other,
            prepared_base_rewrite.export_non_triangular_faces,
            prepared_base_rewrite.export_left_ring_not_closed_faces,
            prepared_base_rewrite.export_missing_origin_faces,
            prepared_base_rewrite.export_face_record_left_mismatch_faces,
            prepared_base_rewrite.export_face_left_ring_mismatch_faces,
            prepared_base_rewrite.export_other_failed_faces,
        ],
        [0, 0, 0, 0, 64, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        prepared_base_rewrite.near_stitch_failed_details.len(),
        prepared_base_rewrite.near_stitch_updates_failed
    );
    assert!(prepared_base_rewrite.near_stitch_failed_details.is_empty());
    let rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_stats
        .as_ref()
        .expect("rewrite export stats");
    assert_eq!(rewrite_stats.vertex_count, 16);
    assert_eq!(rewrite_stats.face_count, 28);
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_exported_mesh_health
        .is_some());
    let packed_rewrite_stats = result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_stats
        .as_ref()
        .expect("packed rewrite export stats");
    assert_eq!(packed_rewrite_stats.vertex_count, 16);
    assert_eq!(packed_rewrite_stats.face_count, 28);
    assert_ne!(
        packed_rewrite_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_VERTICES
    );
    assert_ne!(
        packed_rewrite_stats.face_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_FACES
    );
    assert!(result
        .diagnostics
        .meshlib_topology_record_rewrite_packed_mesh_health
        .is_some());
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 16);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 28);
    assert_ne!(
        result.diagnostics.output_mesh_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_VERTICES
    );
    assert_ne!(
        result.diagnostics.output_mesh_stats.face_count,
        MESHLIB_CUBE_OVERLAP_INTERSECTION_FACES
    );
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert!(!result.diagnostics.meshlib_topology_rewrite_ready);
    assert_eq!(result.diagnostics.topology_splice_exported_faces, 28);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_before_materialization,
        42
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_after_materialization,
        42
    );
    assert_eq!(
        result.diagnostics.topology_splice_deleted_synthetic_edges,
        0
    );
    assert!(
        (result.diagnostics.output_mesh_stats.volume_mm3 - 4.0).abs() < 1e-6,
        "the stored MeshLib cube-overlap intersection volume is 4 mm3"
    );
    assert!(
        (result.diagnostics.output_mesh_stats.surface_area_mm2 - 16.0).abs() < 1e-6,
        "the MeshLib-style cube-overlap intersection envelope is a 1x2x2 box"
    );
}

#[test]
fn exact_boolean_cube_overlap_difference_matches_meshlib_envelope() {
    let (source_vertices, source_faces) = cube();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 1.0, vertex[1], vertex[2]])
        .collect::<Vec<_>>();
    let (reference_vertices, reference_faces) = meshlib_cube_overlap_difference();
    let reference_stats = mesh_stats(&reference_vertices, &reference_faces).unwrap();
    let reference_health =
        mesh_health(&reference_vertices, &reference_faces, true, None, 1e-8).unwrap();

    let result = exact_boolean_from_meshes(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &source_faces,
        ExactBooleanOperation::DifferenceAB,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.diagnostics.parity_ready);
    assert!(result.diagnostics.stitch_compatible);
    assert_eq!(result.diagnostics.stitch_unmatched_first_edges, 0);
    assert_eq!(result.diagnostics.stitch_unmatched_second_edges, 0);
    assert_eq!(result.diagnostics.stitch_cut_path_length_mismatches, 0);
    assert!(result.diagnostics.meshlib_topology_rewrite_ready);
    assert!(result.diagnostics.first_prepare_part_dividable);
    assert!(result.diagnostics.second_prepare_part_dividable);
    assert!(result.diagnostics.result_cut_paths_complete);
    assert_eq!(
        [
            result.diagnostics.meshlib_topology_base_faces,
            result.diagnostics.meshlib_topology_incoming_faces,
            result.assembly.prepare_first_faces.len(),
            result.assembly.prepare_second_faces.len(),
            result.assembly.selected_first_faces.len(),
            result.assembly.selected_second_faces.len(),
        ],
        [16, 10, 16, 10, 16, 10]
    );
    let prepared_base_rewrite = result
        .diagnostics
        .meshlib_topology_prepared_base_record_rewrite;
    assert_eq!(
        [
            prepared_base_rewrite.prepared_faces,
            prepared_base_rewrite.prepared_vertices,
            prepared_base_rewrite.virtual_vertices,
            prepared_base_rewrite.prepared_face_sources,
            prepared_base_rewrite.applied_commands,
            prepared_base_rewrite.failed_commands,
            prepared_base_rewrite.near_stitch_updates_applied,
            prepared_base_rewrite.near_stitch_updates_failed,
            prepared_base_rewrite.exported_faces,
            prepared_base_rewrite.export_failed_faces,
        ],
        [16, 14, 0, 16, 10, 0, 0, 0, 26, 0]
    );
    assert!(prepared_base_rewrite.ready_for_export);
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_near_stitch_target_left_closures,
            prepared_base_rewrite.record_rewrite_near_stitch_target_right_closures,
        ),
        (3, 0)
    );
    assert_eq!(
        (
            prepared_base_rewrite.copied_prev_next_edge_update_attempts,
            prepared_base_rewrite.copied_prev_next_edge_updates_applied,
            prepared_base_rewrite.copied_prev_next_edge_updates_skipped,
            prepared_base_rewrite
                .copied_prev_next_edge_update_details
                .len(),
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(
        (
            prepared_base_rewrite.mapped_source_record_replays,
            prepared_base_rewrite.mapped_source_record_replays_on_near_stitch_targets,
            prepared_base_rewrite.mapped_source_record_replay_attempts,
            prepared_base_rewrite.mapped_source_record_replay_attempts_on_near_stitch_targets,
            prepared_base_rewrite.skipped_mapped_source_record_replays,
        ),
        (20, 0, 20, 0, 0)
    );
    assert_eq!(
        [
            prepared_base_rewrite.record_failed_missing_targets,
            prepared_base_rewrite.record_failed_closed_targets,
            prepared_base_rewrite.record_failed_missing_sources,
            prepared_base_rewrite.record_failed_other_commands,
            prepared_base_rewrite.translated_copied_edge_records,
            prepared_base_rewrite.translated_copied_face_records,
            prepared_base_rewrite.failed_copied_edge_records,
            prepared_base_rewrite.refreshed_face_records,
            prepared_base_rewrite.near_stitch_failed_start,
            prepared_base_rewrite.near_stitch_failed_end,
            prepared_base_rewrite.near_stitch_missing_previous_edges,
            prepared_base_rewrite.near_stitch_missing_next_edges,
            prepared_base_rewrite.near_stitch_origin_mismatches,
            prepared_base_rewrite.near_stitch_previous_left_faces,
            prepared_base_rewrite.near_stitch_next_right_faces,
            prepared_base_rewrite.near_stitch_failed_other,
            prepared_base_rewrite.export_non_triangular_faces,
            prepared_base_rewrite.export_left_ring_not_closed_faces,
            prepared_base_rewrite.export_missing_origin_faces,
            prepared_base_rewrite.export_face_record_left_mismatch_faces,
            prepared_base_rewrite.export_face_left_ring_mismatch_faces,
            prepared_base_rewrite.export_other_failed_faces,
        ],
        [0, 0, 0, 0, 40, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        (
            prepared_base_rewrite.record_rewrite_target_details.len(),
            prepared_base_rewrite
                .mapped_source_record_replay_details
                .len(),
            prepared_base_rewrite
                .copied_prev_next_edge_update_details
                .len(),
            prepared_base_rewrite.copied_face_record_details.len(),
            prepared_base_rewrite.export_failed_face_indices.len(),
            prepared_base_rewrite.export_failed_face_details.len(),
            prepared_base_rewrite.near_stitch_failed_details.len(),
        ),
        (10, 20, 0, 10, 0, 0, 0)
    );
    assert!(prepared_base_rewrite.exported_mesh_stats.is_some());
    assert!(prepared_base_rewrite.exported_mesh_health.is_some());
    assert!(prepared_base_rewrite.packed_mesh_stats.is_some());
    assert!(prepared_base_rewrite.packed_mesh_health.is_some());
    assert_eq!(
        (
            prepared_base_rewrite.near_stitch_skipped_previous_left_source_edges,
            prepared_base_rewrite.near_stitch_skipped_next_right_source_edges,
        ),
        (0, 0)
    );
    assert_eq!(
        (
            prepared_base_rewrite.near_stitch_previous_left_copied_source_edges,
            prepared_base_rewrite.near_stitch_next_right_copied_source_edges,
        ),
        (0, 0)
    );
    assert_eq!(
        (
            result.diagnostics.meshlib_topology_raw_selected_faces,
            result
                .diagnostics
                .meshlib_topology_same_oriented_overlap_faces,
            result.diagnostics.meshlib_topology_boundary_misses,
            result
                .diagnostics
                .meshlib_topology_coplanar_selection_delta_faces,
        ),
        ([16, 10], [7, 10], [[0, 0], [0, 0]], [0, 0])
    );
    assert_eq!(result.output.source, ExactBooleanOutputMeshSource::Assembly);
    assert_eq!(result.output.vertices.len(), 15);
    assert_eq!(result.output.faces.len(), 26);
    assert_eq!(result.diagnostics.output_mesh_stats.connected_components, 1);
    assert_eq!(result.diagnostics.output_mesh_stats.vertex_count, 15);
    assert_eq!(result.diagnostics.output_mesh_stats.face_count, 26);
    assert_eq!(
        result.diagnostics.output_mesh_stats.vertex_count,
        MESHLIB_CUBE_OVERLAP_DIFFERENCE_VERTICES
    );
    assert_eq!(
        result.diagnostics.output_mesh_stats.face_count,
        MESHLIB_CUBE_OVERLAP_DIFFERENCE_FACES
    );
    assert_eq!(result.diagnostics.output_mesh_health.boundary_edge_count, 0);
    assert_eq!(
        result.diagnostics.output_mesh_health.nonmanifold_edge_count,
        0
    );
    assert!(result.diagnostics.output_mesh_health.is_closed);
    assert!(
        result
            .diagnostics
            .output_mesh_health
            .self_intersections_available
    );
    assert!(result
        .diagnostics
        .output_mesh_health
        .self_intersections
        .is_some());
    assert!(reference_health.is_closed);
    assert_eq!(reference_health.boundary_edge_count, 0);
    assert_eq!(reference_health.nonmanifold_edge_count, 0);
    assert_eq!(
        reference_health.self_intersections,
        Some(MESHLIB_CUBE_OVERLAP_DIFFERENCE_SELF_INTERSECTIONS)
    );
    assert!((reference_stats.volume_mm3 - 4.0).abs() < 1e-6);
    assert!(
        (result.diagnostics.output_mesh_stats.volume_mm3 - reference_stats.volume_mm3).abs() < 1e-6
    );
    assert!(
        (result.diagnostics.output_mesh_stats.surface_area_mm2 - reference_stats.surface_area_mm2)
            .abs()
            < 1e-6
    );
    for axis in 0..3 {
        assert!(
            (result.diagnostics.output_mesh_stats.bbox_min[axis] - reference_stats.bbox_min[axis])
                .abs()
                < 1e-6
        );
        assert!(
            (result.diagnostics.output_mesh_stats.bbox_max[axis] - reference_stats.bbox_max[axis])
                .abs()
                < 1e-6
        );
    }
    assert_eq!(
        result.assembly.faces.len(),
        reference_faces.len(),
        "the active difference assembly should match the MeshLib fixture face count"
    );
    assert!(!result.diagnostics.topology_splice_export_changed_faces);
    assert_eq!(result.diagnostics.topology_splice_exported_faces, 26);
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_before_materialization,
        39
    );
    assert_eq!(
        result
            .diagnostics
            .topology_splice_edges_after_materialization,
        39
    );
    assert_eq!(
        result.diagnostics.topology_splice_deleted_synthetic_edges,
        0
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_stitch_compatible
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_first_prepare_part_dividable
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_second_prepare_part_dividable
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_preserves_active_volume
    );
    assert_eq!(
        result.diagnostics.paired_coplanar_candidate_boundary_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_nonmanifold_edges,
        0
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_duplicate_output_faces,
        0
    );
    assert!(
        !result
            .diagnostics
            .paired_coplanar_candidate_result_cut_paths_complete
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_prepare_cut_complete
    );
    assert!(
        result
            .diagnostics
            .paired_coplanar_candidate_self_intersections_available
    );
    assert_eq!(
        result
            .diagnostics
            .paired_coplanar_candidate_self_intersections,
        Some(0)
    );
    assert_ne!(
        result
            .diagnostics
            .paired_coplanar_candidate_self_intersections,
        reference_health.self_intersections
    );
    assert!((result.diagnostics.paired_coplanar_candidate_output_volume - 4.0).abs() < 1e-6);
    assert!(
        (result.diagnostics.paired_coplanar_candidate_output_area - 16.0).abs() < 1e-6,
        "the closed paired candidate is the mathematical slab, not MeshLib's coplanar envelope"
    );
}

#[test]
fn exact_boolean_reported_thin_cube_overlap_outputs_closed_meshes() {
    let (unit_vertices, faces) = cube();
    let source_vertices = unit_vertices
        .iter()
        .map(|vertex| [vertex[0] * 6.0, vertex[1] * 6.0, vertex[2] * 6.0])
        .collect::<Vec<_>>();
    let target_vertices = source_vertices
        .iter()
        .map(|vertex| [vertex[0] + 10.5, vertex[1], vertex[2]])
        .collect::<Vec<_>>();

    for (operation, expected_volume) in [
        (ExactBooleanOperation::Union, 3240.0),
        (ExactBooleanOperation::DifferenceAB, 1512.0),
        (ExactBooleanOperation::Intersection, 216.0),
    ] {
        let result = exact_boolean_from_meshes(
            &source_vertices,
            &faces,
            &target_vertices,
            &faces,
            operation,
            8,
            1e-9,
        )
        .unwrap();

        assert!(
            result.diagnostics.parity_ready,
            "{operation:?} should be MeshLib-parity ready"
        );
        assert_eq!(
            result.diagnostics.output_mesh_health.boundary_edge_count, 0,
            "{operation:?} should not leave boundary edges"
        );
        assert_eq!(
            result.diagnostics.output_mesh_health.nonmanifold_edge_count, 0,
            "{operation:?} should not leave non-manifold edges"
        );
        assert!(result.diagnostics.output_mesh_health.is_closed);
        assert!(
            (result.diagnostics.output_mesh_stats.volume_mm3 - expected_volume).abs() < 1e-6,
            "{operation:?} volume {} != {expected_volume}",
            result.diagnostics.output_mesh_stats.volume_mm3
        );
    }
}

#[test]
fn core_mesh_helpers_match_python_contract() {
    let (vertices, faces) = cube();

    let (bbox_min, bbox_max) = mesh_bounds(&vertices);
    assert_eq!(bbox_min, [-1.0, -1.0, -1.0]);
    assert_eq!(bbox_max, [1.0, 1.0, 1.0]);
    assert_eq!(
        safe_normalize_vectors(&[[3.0, 0.0, 0.0], [0.0, 0.0, 0.0]]),
        vec![[1.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
    );
    assert_eq!(
        normalize_axis_vector([0.0, 2.0, 0.0]).unwrap(),
        [0.0, 1.0, 0.0]
    );
    assert!(normalize_axis_vector([0.0, 0.0, 0.0]).is_err());

    assert_eq!(
        face_normals_for_mesh(&vertices, &faces).unwrap().len(),
        faces.len()
    );
    assert_eq!(
        vertex_normals_for_mesh(&vertices, &faces).unwrap().len(),
        vertices.len()
    );
    assert!((mesh_surface_area(&vertices, &faces).unwrap() - 24.0).abs() < 1e-9);
    assert!((mesh_signed_volume(&vertices, &faces).unwrap() - 8.0).abs() < 1e-9);
    assert!((mesh_volume(&vertices, &faces).unwrap() - 8.0).abs() < 1e-9);
    assert_eq!(boundary_edges_for_mesh(&vertices, &faces).unwrap().len(), 0);
    assert_eq!(
        face_adjacency_for_mesh(&vertices, &faces).unwrap().len(),
        faces.len()
    );
    let mut components = connected_face_components_for_mesh(&vertices, &faces).unwrap();
    assert_eq!(components.len(), 1);
    components[0].sort_unstable();
    assert_eq!(components[0], (0..faces.len() as i64).collect::<Vec<_>>());
    assert_eq!(
        vertex_neighbors_for_mesh(&vertices, &faces).unwrap().len(),
        vertices.len()
    );
}

#[test]
fn open_cube_boundary_loops_match_python_fixture() {
    let (vertices, mut faces) = cube();
    faces.truncate(10);

    let loops = boundary_loops(&vertices, &faces).unwrap();
    let health = mesh_health(&vertices, &faces, true, Some(50_000), 1e-8).unwrap();

    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].len(), 4);
    assert!(!health.is_closed);
    assert_eq!(health.holes_count, 1);
    assert_eq!(health.boundary_edge_count, 4);
    assert_eq!(health.nonmanifold_edge_count, 0);
    assert_eq!(health.self_intersections, Some(0));
    assert!(health.self_intersections_available);
}

#[test]
fn hole_fill_plan_diagnostics_reports_representative_edges_and_triangle_counts() {
    let (vertices, mut faces) = cube();
    faces.truncate(10);

    let report = hole_fill_plan_diagnostics(&vertices, &faces, Some(3)).unwrap();

    assert_eq!(report.input_holes, 1);
    assert_eq!(report.planned_holes, 0);
    assert_eq!(report.skipped_holes, 1);
    assert_eq!(report.total_boundary_edges, 4);
    assert_eq!(report.total_planned_triangles, 0);
    assert_eq!(report.plans[0].representative_edge, [0, 3]);
    assert_eq!(report.plans[0].boundary_vertex_indices, vec![0, 3, 7, 4]);
    assert_eq!(report.plans[0].planned_triangles, 0);
    assert_eq!(
        report.plans[0].skip_reason.as_deref(),
        Some("max_edges_exceeded")
    );

    let unrestricted = hole_fill_plan_diagnostics(&vertices, &faces, None).unwrap();
    assert_eq!(unrestricted.planned_holes, 1);
    assert_eq!(unrestricted.total_planned_triangles, 2);
    assert_eq!(unrestricted.plans[0].planned_triangles, 2);
}

#[test]
fn repeated_hole_boundary_vertices_detects_same_hole_ring_revisits_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 3], [1, 2, 4], [1, 3, 5]];

    let loops = ordered_boundary_loops(&vertices, &faces).unwrap();
    let report = repeated_hole_boundary_vertices_diagnostics(&vertices, &faces).unwrap();

    assert_eq!(loops, vec![vec![0, 1, 2, 4, 1, 5, 3]]);
    assert_eq!(report.input_holes, 1);
    assert_eq!(report.repeated_vertex_count, 1);
    assert_eq!(report.vertices[0].vertex_index, 1);
    assert_eq!(report.vertices[0].hole_indices, vec![0]);
    assert_eq!(report.vertices[0].occurrences, 2);

    let (open_vertices, mut open_faces) = cube();
    open_faces.truncate(10);
    let open_report =
        repeated_hole_boundary_vertices_diagnostics(&open_vertices, &open_faces).unwrap();
    assert_eq!(open_report.input_holes, 1);
    assert_eq!(open_report.repeated_vertex_count, 0);
    assert!(open_report.vertices.is_empty());
}

#[test]
fn hole_complicating_faces_reports_smaller_wedge_faces_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 4], [1, 2, 5], [1, 3, 4]];

    let loops = ordered_boundary_loops(&vertices, &faces).unwrap();
    let report = hole_complicating_faces_diagnostics(&vertices, &faces).unwrap();

    assert_eq!(loops, vec![vec![0, 1, 2, 5, 1, 3, 4]]);
    assert_eq!(report.input_repeated_vertex_count, 1);
    assert_eq!(report.complicating_face_count, 1);
    assert_eq!(report.faces[0].repeated_vertex_index, 1);
    assert_eq!(report.faces[0].face_index, 1);
}

#[test]
fn remove_hole_complicating_faces_deletes_meshlib_reported_faces() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 4], [1, 2, 5], [1, 3, 4]];

    let repaired = remove_hole_complicating_faces(&vertices, &faces).unwrap();

    assert_eq!(repaired.report.input_face_count, 3);
    assert_eq!(repaired.report.output_face_count, 2);
    assert_eq!(repaired.report.removed_face_count, 1);
    assert_eq!(repaired.report.input_repeated_vertex_count, 1);
    assert_eq!(repaired.report.output_repeated_vertex_count, 0);
    assert_eq!(repaired.faces, vec![[0, 1, 4], [1, 3, 4]]);
}

#[test]
fn service_fill_holes_uses_triangulated_patch_like_meshlib_fillhole() {
    let (vertices, mut faces) = cube();
    faces.truncate(10);

    let repaired = service_fill_holes(&vertices, &faces, None).unwrap();
    let health = mesh_health(
        &repaired.vertices,
        &repaired.faces,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();

    assert_eq!(repaired.vertices.len(), vertices.len());
    assert_eq!(repaired.report.input_holes, 1);
    assert_eq!(repaired.report.filled_holes, 1);
    assert_eq!(repaired.report.added_vertices, 0);
    assert_eq!(repaired.report.added_faces, 2);
    assert!(health.is_closed);
}

#[test]
fn strong_hole_fill_avoids_existing_nonboundary_diagonal_like_meshlib() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.1, 0.0],
        [1.5, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.5, 1.0],
    ];
    let boundary_loop = vec![0, 1, 2, 3, 4];
    let existing_faces = vec![[1, 3, 5]];

    let weak_patch = triangulate_hole_loop(&vertices, &boundary_loop);
    assert!(triangulation_uses_edge(&weak_patch, 1, 3));

    let strong_patch = crate::repair::fill::triangulate_hole_loop_strong(
        &vertices,
        &existing_faces,
        &boundary_loop,
    );

    assert_eq!(strong_patch.len(), 3);
    assert!(!triangulation_uses_edge(&strong_patch, 1, 3));
}

fn triangulation_uses_edge(faces: &[[i64; 3]], a: i64, b: i64) -> bool {
    faces.iter().any(|face| {
        [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
            .into_iter()
            .any(|(u, v)| (u == a && v == b) || (u == b && v == a))
    })
}

#[test]
fn health_can_skip_self_intersection_budget() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let health = mesh_health(&vertices, &faces, true, Some(1), 1e-8).unwrap();

    assert_eq!(health.self_intersections, None);
    assert!(!health.self_intersections_available);
}

#[test]
fn service_mesh_health_matches_current_meshlib_payload_contract() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let health = service_mesh_health(&vertices, &faces, 1, 1e-8).unwrap();

    assert!(!health.is_closed);
    assert_eq!(health.self_intersections, 2);
    assert_eq!(health.self_intersection_faces, vec![0]);
    assert_eq!(health.holes_count, 2);
    assert_eq!(health.degenerate_faces, 0);
    assert_eq!(health.health_score, 56);
}

#[test]
fn summarize_thickness_matches_python_behavior() {
    let values = vec![2.0_f32, f32::NAN, -1.0, 0.25, f32::INFINITY, 0.75];

    let summary = summarize_thickness(&values, 0.6);

    assert_eq!(summary.min_mm, Some(0.25));
    assert!((summary.avg_mm.unwrap() - 1.0).abs() < 1e-9);
    assert_eq!(summary.max_mm, Some(2.0));
    assert_eq!(summary.valid_vertex_count, 3);
    assert_eq!(summary.violation_count, 1);
}

#[test]
fn summarize_thickness_handles_no_valid_values() {
    let values = vec![f32::NAN, 0.0, -1.0];

    let summary = summarize_thickness(&values, 0.6);

    assert_eq!(summary.min_mm, None);
    assert_eq!(summary.avg_mm, None);
    assert_eq!(summary.max_mm, None);
    assert_eq!(summary.valid_vertex_count, 0);
    assert_eq!(summary.violation_count, 0);
}

#[test]
fn material_weight_conversions_match_python_contract() {
    assert_eq!(material_density_g_cm3("gold_18k"), 15.58);
    assert_eq!(material_density_g_cm3("unknown"), 15.58);
    assert!((mm3_to_grams(1000.0, "gold_18k") - 15.58).abs() < 1e-12);
    assert!((grams_to_mm3(15.58, "gold_18k") - 1000.0).abs() < 1e-12);

    let table = material_weight_table(1000.0);
    assert_eq!(table.len(), 7);
    assert_eq!(table[0].0, "gold_24k");
    assert_eq!(table[2].0, "gold_18k");
    assert_eq!(table[2].1.volume_mm3, 1000.0);
    assert_eq!(table[2].1.weight_g, 15.58);
    assert!(table[6].1.weight_g > table[5].1.weight_g);
}

#[test]
fn sdf_value_transforms_match_voxel_ops_contract() {
    let values = vec![-2.0_f32, -0.25, 0.0, 1.5];
    let offset = sdf_offset_values(&values, 0.5).unwrap();
    assert_eq!(offset, vec![-2.5, -0.75, -0.5, 1.0]);

    let shell = sdf_shell_values(&values, 1.0).unwrap();
    assert_eq!(shell, vec![1.0, -0.25, 0.0, 1.5]);
    assert!(sdf_offset_values(&values, f64::NAN).is_err());
    assert!(sdf_shell_values(&values, 0.0).is_err());
}

#[test]
fn occupied_sdf_surface_extraction_matches_python_contract() {
    let values = vec![
        -1.0_f32, -1.0, //
        -1.0, -1.0, //
        -1.0, -1.0, //
        -1.0, -1.0,
    ];

    let mesh =
        extract_surface_mesh_from_sdf_cells(&values, [0.0, 0.0, 0.0], [2, 2, 2], 1.0, 0.0).unwrap();

    assert_eq!(mesh.vertices.len(), 8);
    assert_eq!(mesh.faces.len(), 12);
    assert_eq!(mesh.vertices[0], [0.0, 0.0, 0.0]);
    assert_eq!(mesh.faces[0], [0, 1, 2]);
    assert_eq!(mesh.faces[1], [0, 2, 3]);

    let empty =
        extract_surface_mesh_from_sdf_cells(&[1.0; 8], [0.0, 0.0, 0.0], [2, 2, 2], 1.0, 0.0)
            .unwrap();
    assert!(empty.vertices.is_empty());
    assert!(empty.faces.is_empty());
}

#[test]
fn ring_size_helpers_match_python_module_contract() {
    assert_eq!(ring_diameter_for_size(5.0), 15.67);
    assert!(
        (ring_diameter_for_size(5.25) - ((40.0 + 5.25 * 2.55) / std::f64::consts::PI)).abs()
            < 1e-12
    );
    assert_eq!(closest_ring_size(Some(15.6)), Some(5.0));
    assert_eq!(closest_ring_size(None), None);
}

#[test]
fn empty_ring_measurement_matches_python_module_contract() {
    let measurement = measure_ring(&[], None).unwrap();

    assert_eq!(measurement.ring_axis, [0.0, 1.0, 0.0]);
    assert_eq!(measurement.ring_axis_confidence, 0.0);
    assert_eq!(measurement.inner_diameter_mm, None);
    assert_eq!(measurement.bbox_mm, [0.0, 0.0, 0.0]);
    assert!(measurement.needs_axis_confirmation);
}

#[test]
fn nearest_distances_to_indices_matches_vertex_targets() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 3.0, 0.0],
        [4.0, 0.0, 0.0],
    ];

    let distances = nearest_distances_to_indices(&vertices, &[0, 2]).unwrap();

    assert_eq!(distances, vec![0.0, 2.0, 0.0, 4.0]);
}

#[test]
fn protected_hollow_scale_field_preserves_selected_regions() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [16.0, 0.0, 0.0],
    ];
    let regions = vec!["head".to_string(), "outer_band".to_string()];
    let scales = protected_hollow_scale_field(
        &vertices,
        &regions,
        &[0, 1, 3],
        &[1, 2, 3],
        &["head".to_string()],
        1.0,
    )
    .unwrap();

    assert_eq!(scales.len(), vertices.len());
    assert!((scales[1] - 0.18).abs() < 1e-6);
    assert!(scales[0] < 1.0);
    assert_eq!(scales[3], 1.0);
}

#[test]
fn hollow_preview_offsets_vertices_inward() {
    let (vertices, faces) = cube();
    let regions = vec!["head".to_string()];

    let displaced = weighted_inner_offset_vertices(
        &vertices,
        &faces,
        &regions,
        &[0, 1],
        &[0],
        &["head".to_string()],
        0.5,
    )
    .unwrap();

    assert_eq!(displaced.len(), vertices.len());
    assert_ne!(displaced[0], vertices[0]);
}

#[test]
fn adaptive_hollow_to_weight_hits_midpoint_target() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let midpoint_shell = voxel_shell_mesh(&vertices, &faces, 0.8, options).unwrap();
    let target_weight_g = mm3_to_grams(
        mesh_volume(&midpoint_shell.vertices, &midpoint_shell.faces).unwrap(),
        "silver_925",
    );

    let result = adaptive_hollow_to_weight(
        &vertices,
        &faces,
        target_weight_g,
        "silver_925",
        0.01,
        0.4,
        1.2,
        1,
        options,
    )
    .unwrap();

    assert_eq!(result.iterations, 1);
    assert_eq!(result.wall_thickness_mm, Some(0.8));
    assert!(result.warning.is_none());
    assert!((result.achieved_weight_g - target_weight_g).abs() < 0.01);
    assert!(!result.faces.is_empty());
}

#[test]
fn adaptive_hollow_to_weight_reuses_mesh_sdf_for_weight_search() {
    let (vertices, faces) = smooth_torus_for_hollow(32, 12, 9.0, 2.0);
    let options = VoxelMeshOptions {
        voxel_size: 0.6,
        padding_mm: None,
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let target_shell = voxel_shell_mesh(&vertices, &faces, 0.7, options).unwrap();
    let target_weight_g = mm3_to_grams(
        mesh_volume(&target_shell.vertices, &target_shell.faces).unwrap(),
        "silver_925",
    );

    let result = adaptive_hollow_to_weight(
        &vertices,
        &faces,
        target_weight_g,
        "silver_925",
        0.1,
        0.4,
        2.0,
        8,
        options,
    )
    .unwrap();

    assert!(
        result.iterations <= 3,
        "adaptive hollow weight search should start from the cached-field estimate before bisection; iterations {}",
        result.iterations
    );
    assert!(result.warning.is_none(), "{:?}", result.warning);
    assert!((result.achieved_weight_g - target_weight_g).abs() < 0.1);
    assert!(
        mesh_health(&result.vertices, &result.faces, true, Some(50_000), 1e-8)
            .unwrap()
            .is_closed
    );
}

fn smooth_torus_for_hollow(
    radial_segments: usize,
    tube_segments: usize,
    major_radius: f64,
    minor_radius: f64,
) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(radial_segments * tube_segments);
    for radial in 0..radial_segments {
        let theta = 2.0 * std::f64::consts::PI * radial as f64 / radial_segments as f64;
        let (sin_theta, cos_theta) = theta.sin_cos();
        for tube in 0..tube_segments {
            let phi = 2.0 * std::f64::consts::PI * tube as f64 / tube_segments as f64;
            let (sin_phi, cos_phi) = phi.sin_cos();
            let radius = major_radius + minor_radius * cos_phi;
            vertices.push([
                radius * cos_theta,
                radius * sin_theta,
                minor_radius * sin_phi,
            ]);
        }
    }

    let mut faces = Vec::with_capacity(radial_segments * tube_segments * 2);
    for radial in 0..radial_segments {
        let next_radial = (radial + 1) % radial_segments;
        for tube in 0..tube_segments {
            let next_tube = (tube + 1) % tube_segments;
            let a = (radial * tube_segments + tube) as i64;
            let b = (next_radial * tube_segments + tube) as i64;
            let c = (radial * tube_segments + next_tube) as i64;
            let d = (next_radial * tube_segments + next_tube) as i64;
            faces.push([a, b, c]);
            faces.push([c, b, d]);
        }
    }

    (vertices, faces)
}

#[test]
fn protected_hollow_mesh_builds_closed_shell() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };

    let shell = protected_hollow_mesh(
        &vertices,
        &faces,
        &["head".to_string()],
        &[0, 2],
        &[0, 1],
        &["head".to_string()],
        0.8,
        options,
    )
    .unwrap();
    let health = mesh_health(&shell.vertices, &shell.faces, true, Some(50_000), 1e-8).unwrap();

    assert!(!shell.faces.is_empty());
    assert!(health.is_closed);
    assert!(
        mesh_volume(&shell.vertices, &shell.faces).unwrap()
            < mesh_volume(&vertices, &faces).unwrap()
    );
}

#[test]
fn global_thicken_mesh_uses_service_offset_contract() {
    let (vertices, faces) = cube();
    let thickened = global_thicken_mesh(&vertices, &faces, 1.0).unwrap();
    let reference = voxel_offset_mesh(
        &vertices,
        &faces,
        0.5,
        VoxelMeshOptions {
            voxel_size: 0.25,
            padding_mm: None,
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
    .unwrap();

    assert_eq!(thickened.vertices, reference.vertices);
    assert_eq!(thickened.faces, reference.faces);
    assert!(mesh_volume(&thickened.vertices, &thickened.faces).unwrap() > 8.0);
}

#[test]
fn voxel_offset_mesh_accepts_negative_inward_offset() {
    let (vertices, faces) = cube();
    let shrunk = voxel_offset_mesh(
        &vertices,
        &faces,
        -0.25,
        VoxelMeshOptions {
            voxel_size: 0.25,
            padding_mm: Some(1.0),
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
    .unwrap();
    let health = mesh_health(&shrunk.vertices, &shrunk.faces, true, Some(50_000), 1e-8).unwrap();
    let volume = mesh_volume(&shrunk.vertices, &shrunk.faces).unwrap();

    assert!(health.is_closed);
    assert!(volume > 0.0);
    assert!(volume < mesh_volume(&vertices, &faces).unwrap());
}

#[test]
fn voxel_thicken_mesh_keeps_original_and_offset_layers() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.25,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let offset = voxel_offset_mesh(&vertices, &faces, 0.25, options).unwrap();
    let thickened = voxel_thicken_mesh(&vertices, &faces, 0.25, options).unwrap();
    let hollowed = voxel_thicken_mesh(&vertices, &faces, -0.25, options).unwrap();

    assert_eq!(
        thickened.vertices.len(),
        offset.vertices.len() + vertices.len()
    );
    assert_eq!(thickened.faces.len(), offset.faces.len() + faces.len());
    assert!(
        mesh_health(
            &thickened.vertices,
            &thickened.faces,
            true,
            Some(50_000),
            1e-8
        )
        .unwrap()
        .is_closed
    );
    assert!(
        mesh_health(
            &hollowed.vertices,
            &hollowed.faces,
            true,
            Some(50_000),
            1e-8
        )
        .unwrap()
        .is_closed
    );
    assert!(mesh_volume(&thickened.vertices, &thickened.faces).unwrap() > 0.0);
    assert!(mesh_volume(&hollowed.vertices, &hollowed.faces).unwrap() > 0.0);
}

#[test]
fn voxel_weighted_shell_mesh_applies_region_additive_weight() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.25,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let constant_offset = voxel_offset_mesh(&vertices, &faces, 0.2, options).unwrap();
    let weighted_shell = voxel_weighted_shell_mesh(
        &vertices,
        &faces,
        &["corner".to_string()],
        &[0, 1],
        &[6],
        &["corner".to_string()],
        &[0.45],
        0.2,
        1.75,
        options,
    )
    .unwrap();

    assert!(
        mesh_health(
            &weighted_shell.vertices,
            &weighted_shell.faces,
            true,
            Some(50_000),
            1e-8
        )
        .unwrap()
        .is_closed
    );
    assert!(
        mesh_volume(&weighted_shell.vertices, &weighted_shell.faces).unwrap()
            > mesh_volume(&constant_offset.vertices, &constant_offset.faces).unwrap()
    );
}

#[test]
fn voxel_partial_offset_mesh_expands_selected_region_less_than_global_offset() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let partial = voxel_partial_offset_mesh(
        &vertices,
        &faces,
        &["top".to_string()],
        &[0, 4],
        &[4, 5, 6, 7],
        &["top".to_string()],
        0.4,
        options,
    )
    .unwrap();
    let global = voxel_offset_mesh(&vertices, &faces, 0.4, options).unwrap();
    let source_volume = mesh_volume(&vertices, &faces).unwrap();
    let partial_volume = mesh_volume(&partial.vertices, &partial.faces).unwrap();
    let global_volume = mesh_volume(&global.vertices, &global.faces).unwrap();

    assert!(
        mesh_health(&partial.vertices, &partial.faces, true, Some(50_000), 1e-8)
            .unwrap()
            .is_closed
    );
    assert!(partial_volume > source_volume);
    assert!(partial_volume < global_volume);
}

#[test]
fn service_hollow_mesh_uses_meshlib_service_shell_contract() {
    let (vertices, faces) = cube();
    let shell = service_hollow_mesh(&vertices, &faces, 1.0).unwrap();
    let reference = voxel_shell_mesh(
        &vertices,
        &faces,
        1.0,
        VoxelMeshOptions {
            voxel_size: 0.25,
            padding_mm: None,
            extractor: VoxelMeshExtractor::Marching,
            refine: false,
        },
    )
    .unwrap();

    assert_eq!(service_hollow_voxel_size(&vertices, 1.0).unwrap(), 0.25);
    assert_eq!(shell.vertices, reference.vertices);
    assert_eq!(shell.faces, reference.faces);
    assert!(mesh_volume(&shell.vertices, &shell.faces).unwrap() < 8.0);
}

#[test]
fn adaptive_protected_hollow_to_weight_hits_midpoint_target() {
    let (vertices, faces) = cube();
    let options = VoxelMeshOptions {
        voxel_size: 0.5,
        padding_mm: Some(1.0),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let region_ids = vec!["head".to_string()];
    let vertex_offsets = vec![0, 2];
    let vertex_indices = vec![0, 1];
    let protect_region_ids = vec!["head".to_string()];
    let midpoint_shell = protected_hollow_mesh(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &protect_region_ids,
        0.8,
        options,
    )
    .unwrap();
    let target_weight_g = mm3_to_grams(
        mesh_volume(&midpoint_shell.vertices, &midpoint_shell.faces).unwrap(),
        "silver_925",
    );

    let result = adaptive_protected_hollow_to_weight(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &protect_region_ids,
        target_weight_g,
        "silver_925",
        0.01,
        0.4,
        1.2,
        1,
        options,
    )
    .unwrap();

    assert_eq!(result.iterations, 1);
    assert_eq!(result.wall_thickness_mm, Some(0.8));
    assert!(result.warning.is_none());
    assert!((result.achieved_weight_g - target_weight_g).abs() < 0.01);
    assert!(!result.faces.is_empty());
}

#[test]
fn adaptive_protected_hollow_to_weight_keeps_closed_ring_target() {
    let (vertices, faces) = smooth_torus_for_hollow(24, 8, 8.0, 1.8);
    let options = VoxelMeshOptions {
        voxel_size: 0.8,
        padding_mm: Some(2.4),
        extractor: VoxelMeshExtractor::Marching,
        refine: false,
    };
    let region_ids = vec!["head".to_string()];
    let vertex_offsets = vec![0, 4];
    let vertex_indices = vec![0, 1, 2, 3];
    let protect_region_ids = vec!["head".to_string()];
    let target_shell = protected_hollow_mesh(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &protect_region_ids,
        0.7,
        options,
    )
    .unwrap();
    let target_weight_g = mm3_to_grams(
        mesh_volume(&target_shell.vertices, &target_shell.faces).unwrap(),
        "silver_925",
    );

    let result = adaptive_protected_hollow_to_weight(
        &vertices,
        &faces,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &protect_region_ids,
        target_weight_g,
        "silver_925",
        0.1,
        0.4,
        2.0,
        8,
        options,
    )
    .unwrap();

    assert!(result.warning.is_none(), "{:?}", result.warning);
    assert!((result.achieved_weight_g - target_weight_g).abs() < 0.1);
    assert!(
        mesh_health(&result.vertices, &result.faces, true, Some(50_000), 1e-8)
            .unwrap()
            .is_closed
    );
}

#[test]
fn drain_hole_planning_returns_opposing_plans() {
    let vertices = vec![
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
    ];
    let plans = plan_drain_holes(
        &vertices,
        &["inner_band".to_string()],
        &[0, 4],
        &[0, 1, 2, 3],
        [0.0, 1.0, 0.0],
        0.8,
        1.0,
    )
    .unwrap();

    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].radius_mm, 0.5);
    assert_eq!(plans[0].length_mm, 4.0);
    assert!(dot(plans[0].direction, plans[1].direction) < -0.95);
}

#[test]
fn drain_hole_cutter_mesh_counts_match_python_contract() {
    let cutter = drain_hole_cutter_mesh(
        DrainHolePlan {
            center_mm: [0.0, 0.0, 0.0],
            direction: [1.0, 0.0, 0.0],
            radius_mm: 0.5,
            length_mm: 4.0,
        },
        16,
    )
    .unwrap();

    assert_eq!(cutter.vertices.len(), 34);
    assert_eq!(cutter.faces.len(), 64);

    let cutters = drain_hole_cutters_mesh(
        &[
            DrainHolePlan {
                center_mm: [0.0, 0.0, 0.0],
                direction: [1.0, 0.0, 0.0],
                radius_mm: 0.5,
                length_mm: 4.0,
            },
            DrainHolePlan {
                center_mm: [0.0, 0.0, 0.0],
                direction: [-1.0, 0.0, 0.0],
                radius_mm: 0.5,
                length_mm: 4.0,
            },
        ],
        12,
    )
    .unwrap();

    assert_eq!(cutters.vertices.len(), 52);
    assert_eq!(cutters.faces.len(), 96);
}

#[test]
fn compare_summary_matches_cube_surface_distances() {
    let source_vertices = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let (target_vertices, target_faces) = cube();
    let target_vertices: Vec<[f64; 3]> = target_vertices
        .into_iter()
        .map(|vertex| scale(vertex, 2.0))
        .collect();

    let distances =
        nearest_surface_distances(&source_vertices, &target_vertices, &target_faces).unwrap();
    let summary = compare_summary(&source_vertices, &target_vertices, &target_faces).unwrap();

    assert!(distances
        .iter()
        .all(|distance| (*distance - 1.0).abs() < 1e-6));
    assert_eq!(summary.min_mm, Some(1.0));
    assert_eq!(summary.max_mm, Some(1.0));
    assert_eq!(summary.mean_mm, Some(1.0));
}

#[test]
fn nearest_vertex_distances_match_python_behavior() {
    let source_vertices = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
    let target_vertices = vec![[0.0, 1.0, 0.0], [5.0, 0.0, 0.0]];

    let distances = nearest_vertex_distances(&source_vertices, &target_vertices);

    assert_eq!(distances.len(), 2);
    assert!((distances[0] - 1.0).abs() < 1e-6);
    assert!((distances[1] - 2.236_068).abs() < 1e-5);
}

#[test]
fn signed_compare_summary_uses_unsigned_fallback_for_open_target() {
    let source_vertices = vec![[0.0, 0.0, 0.0]];
    let (target_vertices, mut target_faces) = cube();
    target_faces.truncate(10);

    let distances = signed_surface_distances(
        &source_vertices,
        &target_vertices,
        &target_faces,
        0.5,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();
    let summary = signed_compare_summary(
        &source_vertices,
        &target_vertices,
        &target_faces,
        0.5,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();

    assert_eq!(distances.len(), 1);
    assert!(distances[0] >= 0.0);
    assert_eq!(summary.min_mm, Some(f64::from(distances[0])));
}

#[test]
fn version_compare_summary_matches_service_contract_shape() {
    let (source_vertices, source_faces) = cube();
    let target_vertices: Vec<[f64; 3]> = source_vertices
        .iter()
        .map(|vertex| scale(*vertex, 2.0))
        .collect();
    let target_faces = source_faces.clone();

    let summary = version_compare_summary(
        &source_vertices,
        &source_faces,
        &target_vertices,
        &target_faces,
        SignedCompareOptions {
            winding_threshold: 0.5,
            reject_self_intersections: true,
            max_self_intersection_faces: Some(50_000),
            epsilon: 1e-8,
        },
    )
    .unwrap();

    assert!((summary.volume_delta_mm3 + 56.0).abs() < 1e-12);
    assert_eq!(summary.bbox_delta_mm, [-2.0, -2.0, -2.0]);
    assert_eq!(summary.min_signed_distance_mm, Some(-1.0));
    assert_eq!(summary.max_signed_distance_mm, Some(-1.0));
    assert_eq!(summary.mean_signed_distance_mm, Some(-1.0));
}

#[test]
fn version_compare_distances_filter_service_outliers() {
    let (source_vertices, _) = cube();
    let (target_vertices, target_faces) = cube();
    let far_target_vertices: Vec<[f64; 3]> = target_vertices
        .iter()
        .map(|vertex| add(*vertex, [100.0, 0.0, 0.0]))
        .collect();

    let distances = version_compare_distances(
        &source_vertices,
        &far_target_vertices,
        &target_faces,
        SignedCompareOptions {
            winding_threshold: 0.5,
            reject_self_intersections: true,
            max_self_intersection_faces: Some(50_000),
            epsilon: 1e-8,
        },
    )
    .unwrap();

    assert_eq!(distances.len(), source_vertices.len());
    assert!(distances.iter().all(|distance| distance.is_nan()));
}

#[test]
fn service_compare_distances_follow_meshlib_reference_mesh_direction() {
    let (source_vertices, source_faces) = cube();
    let other_vertices = vec![
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.0, 0.5, -0.5],
        [0.0, 0.0, 0.75],
    ];
    let other_faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
    let options = SignedCompareOptions {
        winding_threshold: 0.5,
        reject_self_intersections: true,
        max_self_intersection_faces: Some(50_000),
        epsilon: 1e-8,
    };

    let service_distances =
        service_compare_distances(&source_vertices, &source_faces, &other_vertices, options)
            .unwrap();
    let expected =
        version_compare_distances(&other_vertices, &source_vertices, &source_faces, options)
            .unwrap();
    let summary = service_compare_summary(
        &source_vertices,
        &source_faces,
        &other_vertices,
        &other_faces,
        options,
    )
    .unwrap();

    assert_eq!(service_distances.len(), other_vertices.len());
    assert_eq!(service_distances, expected);
    assert!(summary.volume_delta_mm3 > 0.0);
    assert_eq!(summary.bbox_delta_mm, [1.0, 1.0, 0.75]);
    assert_eq!(
        summary.min_signed_distance_mm,
        summarize_distances(&service_distances, true).min_mm
    );
}

#[test]
fn cube_has_no_self_intersections() {
    let (vertices, faces) = cube();
    let intersections = self_intersecting_faces(&vertices, &faces, 1e-8).unwrap();

    assert!(intersections.is_empty());
}

#[test]
fn crossing_triangles_report_both_faces() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let intersections = self_intersecting_faces(&vertices, &faces, 1e-8).unwrap();

    assert_eq!(intersections, vec![0, 1]);
}

#[test]
fn fix_self_intersections_relax_matches_meshlib_relax_region_without_subdivision() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5], [0, 2, 3], [2, 3, 5]];

    let result = fix_self_intersections_relax(
        &vertices,
        &faces,
        FixSelfIntersectionsRelaxOptions {
            relax_iterations: 1,
            max_expand: 3,
            touch_is_intersection: true,
            force: 0.5,
            epsilon: 1e-8,
        },
    )
    .unwrap();

    assert_eq!(result.faces, faces);
    assert_eq!(result.report.input_self_intersections, 2);
    assert_eq!(result.report.output_self_intersections, 0);
    assert_eq!(result.report.relaxed_face_count, 4);
    assert_eq!(result.report.moved_vertex_count, 6);
    assert!(!result.report.topology_changed);

    let expected = [
        [-1.0 / 3.0, 1.0 / 12.0, -1.0 / 6.0],
        [0.25, 0.25, 0.0],
        [0.0, 0.52, -0.2],
        [-0.1, 0.02, -0.4],
        [0.0, -0.075, 0.25],
        [0.0, 0.6, 0.0],
    ];
    for (actual, expected) in result.vertices.iter().zip(expected) {
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() < 1e-12);
        }
    }
}

#[test]
fn fix_self_intersections_relax_ignores_cross_component_intersections_like_meshlib_fix() {
    let vertices = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.5, -1.0],
        [0.0, -0.5, 1.0],
        [0.0, 1.2, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];

    let result = fix_self_intersections_relax(
        &vertices,
        &faces,
        FixSelfIntersectionsRelaxOptions {
            relax_iterations: 1,
            max_expand: 3,
            touch_is_intersection: true,
            force: 0.5,
            epsilon: 1e-8,
        },
    )
    .unwrap();

    assert_eq!(result.vertices, vertices);
    assert_eq!(result.faces, faces);
    assert_eq!(result.report.input_self_intersections, 0);
    assert_eq!(result.report.relaxed_face_count, 0);
    assert_eq!(result.report.moved_vertex_count, 0);
}

#[test]
fn torus_with_self_intersections_matches_meshlib_get_faces_count() {
    let (vertices, faces) = torus_with_self_intersections(1.0, 0.2, 32, 16);

    let intersections = self_intersecting_faces_with_touch(&vertices, &faces, 1e-8, false).unwrap();

    assert_eq!(faces.len(), 1024);
    assert_eq!(intersections.len(), 128);
}

#[test]
fn point_mesh_distances_match_cube_fixture() {
    let (vertices, faces) = cube();
    let points = vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0]];

    let distances = point_mesh_distances(&points, &vertices, &faces).unwrap();

    assert_eq!(distances.len(), 2);
    assert!((distances[0] - 1.0).abs() < 1e-9);
    assert!((distances[1] - 1.0).abs() < 1e-9);
}

fn torus_with_self_intersections(
    primary_radius: f64,
    secondary_radius: f64,
    primary_resolution: usize,
    secondary_resolution: usize,
) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let mut vertices = Vec::with_capacity(primary_resolution * secondary_resolution);
    for i in 0..secondary_resolution {
        let a = 2.0 * std::f64::consts::PI * i as f64 / secondary_resolution as f64;
        for j in 0..primary_resolution {
            let b = 2.0 * std::f64::consts::PI * j as f64 / primary_resolution as f64;
            vertices.push([
                (primary_radius - secondary_radius * a.cos()) * b.cos(),
                (primary_radius - secondary_radius * a.cos()) * b.sin(),
                secondary_radius * (2.0 * a).sin(),
            ]);
        }
    }

    let mut faces = Vec::with_capacity(2 * vertices.len());
    for i in 0..secondary_resolution {
        for j in 0..primary_resolution {
            faces.push([
                (i * primary_resolution + j) as i64,
                (((i + 1) % secondary_resolution) * primary_resolution + j) as i64,
                (i * primary_resolution + (j + 1) % primary_resolution) as i64,
            ]);
            faces.push([
                (i * primary_resolution + (j + 1) % primary_resolution) as i64,
                (((i + 1) % secondary_resolution) * primary_resolution + j) as i64,
                (((i + 1) % secondary_resolution) * primary_resolution
                    + (j + 1) % primary_resolution) as i64,
            ]);
        }
    }

    (vertices, faces)
}

#[test]
fn closest_points_report_face_ids_for_multiple_queries() {
    let (vertices, faces) = cube();
    let points = vec![[2.0, 0.0, 0.0], [0.0, 0.0, 2.0], [0.25, 0.25, 0.25]];

    let hits = closest_points_on_mesh(&points, &vertices, &faces).unwrap();

    assert_eq!(hits.closest_points.len(), 3);
    assert_eq!(hits.distances.len(), 3);
    assert_eq!(hits.face_indices.len(), 3);
    assert!(hits.face_indices.iter().all(|face_id| *face_id >= 0));
    assert!((hits.distances[0] - 1.0).abs() < 1e-9);
    assert!((hits.distances[1] - 1.0).abs() < 1e-9);
    assert!((hits.distances[2] - 0.75).abs() < 1e-9);
}

#[test]
fn ray_hits_cube_front_face() {
    let (vertices, faces) = cube();

    let hit = first_ray_hit(
        &vertices,
        &faces,
        [0.0, 0.0, 3.0],
        [0.0, 0.0, -1.0],
        1e-8,
        &[],
    )
    .unwrap()
    .unwrap();

    assert!((hit.distance - 2.0).abs() < 1e-9);
    assert_eq!(hit.point, [0.0, 0.0, 1.0]);
}

#[test]
fn ray_hit_skips_ignored_nearest_faces() {
    let (vertices, faces) = cube();

    let hit = first_ray_hit(
        &vertices,
        &faces,
        [0.0, 0.0, 3.0],
        [0.0, 0.0, -1.0],
        1e-8,
        &[2, 3],
    )
    .unwrap()
    .unwrap();

    assert!((hit.distance - 4.0).abs() < 1e-9);
    assert_eq!(hit.point, [0.0, 0.0, -1.0]);
}

#[test]
fn batched_ray_hits_reuse_tree_and_report_misses() {
    let (vertices, faces) = cube();
    let origins = vec![[0.0, 0.0, 3.0], [4.0, 4.0, 4.0]];
    let directions = vec![[0.0, 0.0, -1.0], [1.0, 0.0, 0.0]];

    let hits = first_ray_hits(&vertices, &faces, &origins, &directions, 1e-8, &[]).unwrap();

    assert_eq!(hits.face_indices.len(), 2);
    assert!(hits.face_indices[0] >= 0);
    assert!((hits.distances[0] - 2.0).abs() < 1e-9);
    assert_eq!(hits.points[0], [0.0, 0.0, 1.0]);
    assert_eq!(hits.face_indices[1], -1);
    assert!(hits.distances[1].is_infinite());
    assert!(hits.points[1].iter().all(|value| value.is_nan()));
}

#[test]
fn winding_numbers_classify_cube_points() {
    let (vertices, faces) = cube();
    let points = vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0]];

    let values = winding_numbers(&points, &vertices, &faces).unwrap();

    assert!((values[0].abs() - 1.0).abs() < 1e-9);
    assert!(values[1].abs() < 1e-9);
}

#[test]
fn signed_point_mesh_distances_classify_cube_points() {
    let (vertices, faces) = cube();
    let points = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]];

    let distances = signed_point_mesh_distances(&points, &vertices, &faces, 0.5).unwrap();

    assert!((distances[0] + 1.0).abs() < 1e-9);
    assert!((distances[1] - 1.0).abs() < 1e-9);
}

#[test]
fn ray_thickness_matches_cube_fixture_shape() {
    let (vertices, faces) = cube();

    let thickness = ray_thickness_at_vertices(&vertices, &faces, 1e-5).unwrap();

    assert_eq!(thickness.len(), vertices.len());
    assert!(thickness.iter().all(|value| value.is_finite()));
    assert!(thickness.iter().all(|value| *value > 0.0));
}

#[test]
fn service_thickness_combines_insphere_and_ray_like_meshlib_service() {
    let (vertices, faces) = cube();
    let options = InSphereThicknessOptions {
        max_radius: 0.5,
        ..InSphereThicknessOptions::default()
    };

    let ray = ray_thickness_at_vertices(&vertices, &faces, options.epsilon).unwrap();
    let insphere = insphere_thickness_at_vertices(&vertices, &faces, options).unwrap();
    let combined = service_thickness_at_vertices(&vertices, &faces, options).unwrap();

    assert_eq!(combined.len(), vertices.len());
    for ((combined_value, insphere_value), ray_value) in combined.iter().zip(&insphere).zip(&ray) {
        assert!(combined_value.is_finite());
        assert!(*combined_value > 0.0);
        assert!(*combined_value <= *insphere_value + 1e-6);
        assert!(*combined_value as f64 <= *ray_value + 1e-6);
        assert!(*combined_value <= 1.0 + 1e-6);
    }
}

#[test]
fn sdf_grid_values_classify_cube_center() {
    let (vertices, faces) = cube();

    let values =
        sdf_grid_values(&vertices, &faces, [-2.0, -2.0, -2.0], [5, 5, 5], 1.0, 0.5).unwrap();

    assert_eq!(values.len(), 125);
    assert!(values[2 * 25 + 2 * 5 + 2] < 0.0);
    assert!(values[0] > 0.0);
}

#[test]
fn sample_sdf_grid_in_bounds_uses_meshlib_style_ceil_lattice() {
    let (vertices, faces) = cube();

    let sample = sample_sdf_grid_in_bounds(
        &vertices,
        &faces,
        [-1.0, 0.1, 2.0],
        [1.1, 3.2, 2.4],
        0.75,
        0.5,
        [0.25, -0.5, 1.0],
        0.5,
    )
    .unwrap();

    assert_eq!(sample.origin, [-1.3125, -0.775, 2.25]);
    assert_eq!(sample.shape, [6, 7, 3]);
    assert_eq!(sample.values.len(), 6 * 7 * 3);
}

#[test]
fn sdf_grid_helpers_match_python_contract() {
    let values = vec![-1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    let cells = sdf_cell_values(&values, [2, 2, 2]).unwrap();
    let occupancy = sdf_occupancy(&values, [2, 2, 2], 0.0).unwrap();
    let volume = estimate_sdf_volume(&values, [2, 2, 2], 0.5, 0.0).unwrap();
    let samples = sample_sdf_values_batch(
        &values,
        [0.0, 0.0, 0.0],
        [2, 2, 2],
        1.0,
        &[[0.0, 0.0, 0.0], [0.5, 0.5, 0.5]],
    )
    .unwrap();
    let gradients =
        sample_sdf_gradients_batch(&values, [0.0, 0.0, 0.0], [2, 2, 2], 1.0, &[[0.5; 3]]).unwrap();

    assert_eq!(cells, vec![0.75]);
    assert_eq!(occupancy, vec![0]);
    assert_eq!(volume, 0.0);
    assert_eq!(samples[0], -1.0);
    assert!((samples[1] - 0.75).abs() < 1e-6);
    assert_eq!(gradients.len(), 1);
    assert!(gradients[0].iter().all(|value| value.is_finite()));
}

#[test]
fn sdf_grid_coordinate_helpers_match_meshlib_dense_grid_order() {
    let points = sdf_grid_points([-1.0, 2.0, 4.0], [2, 2, 2], 0.5).unwrap();
    assert_eq!(
        points,
        vec![
            [-1.0, 2.0, 4.0],
            [-1.0, 2.0, 4.5],
            [-1.0, 2.5, 4.0],
            [-1.0, 2.5, 4.5],
            [-0.5, 2.0, 4.0],
            [-0.5, 2.0, 4.5],
            [-0.5, 2.5, 4.0],
            [-0.5, 2.5, 4.5],
        ]
    );

    let grid_points = sdf_points_to_grid(
        [-1.0, 2.0, 4.0],
        0.5,
        &[[-1.0, 2.0, 4.0], [-0.25, 3.0, 5.5]],
    )
    .unwrap();
    assert_eq!(grid_points, vec![[0.0, 0.0, 0.0], [1.5, 2.0, 3.0]]);
}

#[test]
fn sdf_boolean_values_match_field_operations() {
    let left = vec![-1.0, 0.25, 2.0];
    let right = vec![0.5, -0.75, 1.0];

    let union = sdf_boolean_values(&left, &right, SdfBooleanOperation::Union).unwrap();
    let intersection =
        sdf_boolean_values(&left, &right, SdfBooleanOperation::Intersection).unwrap();
    let difference = sdf_boolean_values(&left, &right, SdfBooleanOperation::Difference).unwrap();

    assert_eq!(union, vec![-1.0, -0.75, 1.0]);
    assert_eq!(intersection, vec![0.5, 0.25, 2.0]);
    assert_eq!(difference, vec![-0.5, 0.75, 2.0]);
}

#[test]
fn voxel_binary_values_match_meshlib_binary_operations_plugin_scalar_contract() {
    let left = vec![1.0, 0.25, -2.0];
    let right = vec![0.5, -0.75, 4.0];

    let max_values = voxel_binary_values(&left, &right, VoxelBinaryOperation::Max).unwrap();
    let min_values = voxel_binary_values(&left, &right, VoxelBinaryOperation::Min).unwrap();
    let sum_values = voxel_binary_values(&left, &right, VoxelBinaryOperation::Sum).unwrap();
    let multiply_values =
        voxel_binary_values(&left, &right, VoxelBinaryOperation::Multiply).unwrap();
    let divide_values = voxel_binary_values(&left, &right, VoxelBinaryOperation::Divide).unwrap();

    assert_eq!(max_values, vec![1.0, 0.25, 4.0]);
    assert_eq!(min_values, vec![0.5, -0.75, -2.0]);
    assert_eq!(sum_values, vec![1.5, -0.5, 2.0]);
    assert_eq!(multiply_values, vec![0.5, -0.1875, -8.0]);
    assert_eq!(divide_values, vec![2.0, -0.33333334, -0.5]);
}

#[test]
fn voxel_binary_iso_value_matches_meshlib_binary_operations_plugin() {
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Union),
        1.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Intersection),
        1.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Difference),
        1.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Max),
        2.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Min),
        1.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Sum),
        3.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Multiply),
        2.0
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 2.0, VoxelBinaryOperation::Divide),
        0.5
    );
    assert_eq!(
        voxel_binary_iso_value(1.0, 0.0, VoxelBinaryOperation::Divide),
        1.0
    );
}

#[test]
fn voxel_value_range_matches_meshlib_voxel_payload_contract() {
    let (min_value, max_value) = voxel_value_range(&[-3.0, 2.5, 0.0, 9.0]).unwrap();

    assert_eq!(min_value, -3.0);
    assert_eq!(max_value, 9.0);
}

#[test]
fn raw_voxels_from_file_matches_meshlib_uint8_normalization_contract() {
    let path =
        std::env::temp_dir().join(format!("zennah_raw_voxels_u8_{}.raw", std::process::id()));
    std::fs::write(&path, [0_u8, 128, 255, 64]).unwrap();

    let volume = load_raw_voxels(
        &path,
        RawVoxelParameters {
            dimensions: [2, 2, 1],
            voxel_size: [0.5, 1.0, 2.0],
            grid_level_set: false,
            scalar_type: RawVoxelScalarType::UInt8,
        },
    )
    .unwrap();

    assert_eq!(volume.dimensions, [2, 2, 1]);
    assert_eq!(volume.voxel_size, [0.5, 1.0, 2.0]);
    assert_eq!(volume.scalar_type, RawVoxelScalarType::UInt8);
    assert_eq!(volume.values[0], 0.0);
    assert!((volume.values[1] - 128.0 / 255.0).abs() < 1e-6);
    assert_eq!(volume.values[2], 1.0);
    assert!((volume.values[3] - 64.0 / 255.0).abs() < 1e-6);
    assert_eq!(volume.min, 0.0);
    assert_eq!(volume.max, 1.0);

    let _ = std::fs::remove_file(path);
}

#[test]
fn raw_voxels_from_file_matches_meshlib_float32_and_float32_4_contracts() {
    let float_path =
        std::env::temp_dir().join(format!("zennah_raw_voxels_f32_{}.raw", std::process::id()));
    let mut bytes = Vec::new();
    for value in [-2.5_f32, 0.25, 9.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(&float_path, bytes).unwrap();

    let volume = load_raw_voxels(
        &float_path,
        RawVoxelParameters {
            dimensions: [3, 1, 1],
            voxel_size: [1.0, 1.0, 1.0],
            grid_level_set: false,
            scalar_type: RawVoxelScalarType::Float32,
        },
    )
    .unwrap();

    assert_eq!(volume.values, vec![-2.5, 0.25, 9.0]);
    assert_eq!(volume.min, -2.5);
    assert_eq!(volume.max, 9.0);

    let float4_path = std::env::temp_dir().join(format!(
        "zennah_raw_voxels_f32x4_{}.raw",
        std::process::id()
    ));
    let mut bytes = Vec::new();
    for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(&float4_path, bytes).unwrap();

    let volume = load_raw_voxels(
        &float4_path,
        RawVoxelParameters {
            dimensions: [2, 1, 1],
            voxel_size: [1.0, 1.0, 1.0],
            grid_level_set: false,
            scalar_type: RawVoxelScalarType::Float32_4,
        },
    )
    .unwrap();

    // Float32_4 takes the 4th channel of each 16-byte voxel, unnormalized (like
    // Float32). voxel0 = (1,2,3,4) -> 4.0, voxel1 = (5,6,7,8) -> 8.0. (A stray
    // `/ 0.0` previously made every value +Infinity.)
    assert_eq!(volume.values, vec![4.0, 8.0]);
    assert_eq!(volume.min, 4.0);
    assert_eq!(volume.max, 8.0);

    let _ = std::fs::remove_file(float_path);
    let _ = std::fs::remove_file(float4_path);
}

#[test]
fn raw_voxels_auto_parameters_match_meshlib_filename_parser_contract() {
    let path = std::env::temp_dir().join(format!("w2_h1_s1_x500_F_{}.raw", std::process::id()));
    let mut bytes = Vec::new();
    for value in [0.0_f32, 1.25] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(&path, bytes).unwrap();

    let volume = load_raw_voxels_auto(&path).unwrap();

    assert_eq!(volume.dimensions, [2, 1, 1]);
    assert_eq!(volume.voxel_size, [0.5, 0.5, 0.5]);
    assert_eq!(volume.scalar_type, RawVoxelScalarType::Float32);
    assert!(!volume.grid_level_set);
    assert_eq!(volume.values, vec![0.0, 1.25]);

    let _ = std::fs::remove_file(path);
}

#[test]
fn tiff_voxels_dir_matches_meshlib_sorted_slice_stack_contract() {
    let dir =
        std::env::temp_dir().join(format!("zennah_tiff_voxels_sorted_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let later_path = dir.join("slice_10.tiff");
    let earlier_path = dir.join("slice_02.tiff");
    {
        let file = std::fs::File::create(&later_path).unwrap();
        let mut encoder = tiff::encoder::TiffEncoder::new(file).unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::Gray32Float>(2, 1, &[10.0_f32, 11.0])
            .unwrap();
    }
    {
        let file = std::fs::File::create(&earlier_path).unwrap();
        let mut encoder = tiff::encoder::TiffEncoder::new(file).unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::Gray32Float>(2, 1, &[2.0_f32, 3.0])
            .unwrap();
    }

    let volume = load_tiff_voxels_dir(&dir, [0.5, 0.25, 2.0], false).unwrap();

    assert_eq!(volume.dimensions, [2, 1, 2]);
    assert_eq!(volume.voxel_size, [0.5, 0.25, 2.0]);
    assert_eq!(volume.values, vec![2.0, 3.0, 10.0, 11.0]);
    assert_eq!(volume.min, 2.0);
    assert_eq!(volume.max, 11.0);
    assert!(!volume.grid_level_set);
    assert!(volume.source_files[0].ends_with("slice_02.tiff"));
    assert!(volume.source_files[1].ends_with("slice_10.tiff"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn tiff_voxels_dir_converts_rgb_like_meshlib_and_preserves_level_set_flag() {
    let dir = std::env::temp_dir().join(format!("zennah_tiff_voxels_rgb_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    for (name, samples) in [
        ("slice_01.tiff", [100_u8, 0, 0]),
        ("slice_02.tiff", [0_u8, 100, 0]),
    ] {
        let file = std::fs::File::create(dir.join(name)).unwrap();
        let mut encoder = tiff::encoder::TiffEncoder::new(file).unwrap();
        encoder
            .write_image::<tiff::encoder::colortype::RGB8>(1, 1, &samples)
            .unwrap();
    }

    let volume = load_tiff_voxels_dir(&dir, [1.0, 1.0, 1.0], true).unwrap();

    assert_eq!(volume.dimensions, [1, 1, 2]);
    assert_eq!(volume.values.len(), 2);
    assert!((volume.values[0] - 29.9).abs() < 1e-5);
    assert!((volume.values[1] - 58.7).abs() < 1e-5);
    assert!(volume.grid_level_set);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn sdf_boolean_values_reject_mismatched_lengths() {
    let error = sdf_boolean_values(&[0.0], &[0.0, 1.0], SdfBooleanOperation::Union).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::MismatchedSdfValueCount { left: 1, right: 2 }
    ));
}

#[test]
fn sdf_boolean_marching_tetrahedra_matches_staged_field_extraction() {
    let (vertices, faces) = cube();
    let left = sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let right: Vec<f32> = left.iter().map(|value| *value - 0.25).collect();
    let staged_values = sdf_boolean_values(&left, &right, SdfBooleanOperation::Union).unwrap();
    let staged =
        marching_tetrahedra(&staged_values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();

    let resident = sdf_boolean_marching_tetrahedra(
        &left,
        &right,
        SdfBooleanOperation::Union,
        [-1.5, -1.5, -1.5],
        [7, 7, 7],
        0.5,
        0.0,
    )
    .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn sdf_offset_marching_tetrahedra_matches_staged_field_extraction() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let staged_values: Vec<f32> = values.iter().map(|value| *value - 0.25).collect();
    let staged =
        marching_tetrahedra(&staged_values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();

    let resident =
        sdf_offset_marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.25, 0.0)
            .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn sdf_shell_marching_tetrahedra_matches_staged_field_extraction() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-2.0, -2.0, -2.0], [9, 9, 9], 0.5, 0.5).unwrap();
    let staged_values: Vec<f32> = values
        .iter()
        .map(|value| (*value as f64).max(-(*value as f64 + 0.75)) as f32)
        .collect();
    let staged =
        marching_tetrahedra(&staged_values, [-2.0, -2.0, -2.0], [9, 9, 9], 0.5, 0.0).unwrap();

    let resident =
        sdf_shell_marching_tetrahedra(&values, [-2.0, -2.0, -2.0], [9, 9, 9], 0.5, 0.75, 0.0)
            .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn project_vertices_to_sdf_moves_points_toward_cube_surface() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let query = vec![[1.25, 0.0, 0.0], [0.0, 0.0, 1.25]];

    let projected =
        project_vertices_to_sdf(&query, &values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0, 3)
            .unwrap();

    assert!((projected[0][0] - 1.0).abs() < 1e-5);
    assert!(projected[0][1].abs() < 1e-5);
    assert!(projected[0][2].abs() < 1e-5);
    assert!((projected[1][2] - 1.0).abs() < 1e-5);
}

#[test]
fn project_vertices_to_sdf_rejects_mismatched_grid_values() {
    let error = project_vertices_to_sdf(&[], &[0.0], [0.0; 3], [2, 2, 2], 1.0, 0.0, 1).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::SdfValueCountDoesNotMatchShape {
            values: 1,
            shape: [2, 2, 2]
        }
    ));
}

#[test]
fn refine_vertices_with_sdf_matches_staged_smoothing_and_projection() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();
    let moved: Vec<[f64; 3]> = vertices
        .iter()
        .map(|vertex| [vertex[0] * 0.92, vertex[1] * 0.92, vertex[2] * 0.92])
        .collect();
    let smoothed = laplacian_smooth_vertices(&moved, &faces, 1, 0.2).unwrap();
    let staged = project_vertices_to_sdf(
        &smoothed,
        &values,
        [-1.5, -1.5, -1.5],
        [7, 7, 7],
        0.5,
        0.0,
        3,
    )
    .unwrap();

    let resident = refine_vertices_with_sdf(
        &moved,
        &faces,
        &values,
        [-1.5, -1.5, -1.5],
        [7, 7, 7],
        0.5,
        0.0,
        1,
        0.2,
        3,
    )
    .unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn laplacian_smooth_vertices_matches_one_ring_average() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 3, 2]];

    let smoothed = laplacian_smooth_vertices(&vertices, &faces, 1, 0.5).unwrap();

    assert_eq!(smoothed.len(), vertices.len());
    assert_eq!(smoothed[0], [0.5, 0.5, 0.0]);
    assert_eq!(smoothed[3], [1.5, 1.5, 0.0]);
}

#[test]
fn laplacian_smooth_vertices_zero_iterations_is_noop() {
    let (vertices, faces) = cube();

    let smoothed = laplacian_smooth_vertices(&vertices, &faces, 0, 1.0).unwrap();

    assert_eq!(smoothed, vertices);
}

#[test]
fn weighted_laplacian_smooth_vertices_scales_by_weight() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 3, 2]];
    let weights = vec![1.0, 0.0, 0.5, 1.0];

    let smoothed =
        weighted_laplacian_smooth_vertices(&vertices, &faces, &weights, 1, 0.5, 0.02).unwrap();

    assert_eq!(smoothed[0], [0.5, 0.5, 0.0]);
    assert_eq!(smoothed[1], vertices[1]);
    assert!((smoothed[2][0] - 1.0 / 3.0).abs() < 1e-12);
    assert!((smoothed[2][1] - 5.0 / 3.0).abs() < 1e-12);
    assert_eq!(smoothed[2][2], 0.0);
}

#[test]
fn weighted_laplacian_smooth_vertices_rejects_mismatched_weights() {
    let (vertices, faces) = cube();

    let error =
        weighted_laplacian_smooth_vertices(&vertices, &faces, &[1.0], 1, 0.5, 0.02).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::WeightCountDoesNotMatchVertices {
            weights: 1,
            vertices: 8
        }
    ));
}

#[test]
fn taubin_smooth_vertices_alternates_laplacian_passes() {
    let (vertices, faces) = cube();

    let smoothed = taubin_smooth_vertices(&vertices, &faces, 2, 0.25, -0.5).unwrap();
    let laplacian_only = laplacian_smooth_vertices(&vertices, &faces, 2, 0.25).unwrap();

    assert_eq!(smoothed.len(), vertices.len());
    assert_ne!(smoothed, vertices);
    assert_ne!(smoothed, laplacian_only);
}

#[test]
fn falloff_weights_match_gaussian_seed_distances() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
    ];

    let weights = falloff_weights(&vertices, &[0], 1.0, 3.0).unwrap();

    assert_eq!(weights[0], 1.0);
    assert!((weights[1] - (-0.5_f32).exp()).abs() < 1e-6);
    assert!((weights[2] - (-2.0_f32).exp()).abs() < 1e-6);
    assert_eq!(weights[3], 0.0);
}

#[test]
fn falloff_weights_reject_empty_seeds() {
    let (vertices, _) = cube();

    let error = falloff_weights(&vertices, &[], 1.0, 3.0).unwrap_err();

    assert!(matches!(error, GeometryError::EmptySeedIndices));
}

#[test]
fn smooth_vertices_with_falloff_matches_weighted_pipeline() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [1, 3, 2]];
    let weights = falloff_weights(&vertices, &[0], 2.0, 3.0).unwrap();

    let resident = smooth_vertices_with_falloff(
        &vertices,
        &faces,
        &[0],
        SmoothFalloffOptions {
            falloff_mm: 2.0,
            iterations: 2,
            strength: 0.35,
            active_threshold: 0.02,
            cutoff_multiplier: 3.0,
        },
    )
    .unwrap();
    let staged =
        weighted_laplacian_smooth_vertices(&vertices, &faces, &weights, 2, 0.35, 0.02).unwrap();

    assert_eq!(resident, staged);
}

#[test]
fn outward_directions_flips_center_facing_normals() {
    let vertices = vec![[0.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 1.0, -1.0]];
    let faces = vec![[0, 1, 2]];

    let directions = outward_directions(&vertices, &faces).unwrap();

    assert_eq!(directions, vec![[0.0, 0.0, -1.0]; 3]);
}

#[test]
fn local_offset_vertices_matches_staged_displacement() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];
    let directions = outward_directions(&vertices, &faces).unwrap();
    let weights = falloff_weights(&vertices, &[0], 2.0, 3.0).unwrap();

    let displaced = local_offset_vertices(&vertices, &faces, &[0], 2.0, 0.25, 3.0).unwrap();

    for (index, vertex) in vertices.iter().enumerate() {
        let expected = add(
            *vertex,
            scale(directions[index], 0.25 * weights[index] as f64),
        );
        assert_eq!(displaced[index], expected);
    }
}

#[test]
fn local_thicken_to_minimum_vertices_uses_deficit_field() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];
    let thickness = vec![0.25, 0.75, 1.5, f32::NAN];
    let min_target = 1.0;
    let deficit_scale = 0.75;
    let directions = outward_directions(&vertices, &faces).unwrap();
    let weights = falloff_weights(&vertices, &[0], 2.0, 3.0).unwrap();

    let displaced = local_thicken_to_minimum_vertices(
        &vertices,
        &faces,
        &[0],
        &thickness,
        min_target,
        2.0,
        deficit_scale,
    )
    .unwrap();

    for (index, vertex) in vertices.iter().enumerate() {
        let safe_thickness = if thickness[index].is_finite() {
            thickness[index].max(0.0) as f64
        } else {
            0.0
        };
        let deficit = (min_target - safe_thickness).clamp(0.0, min_target);
        let expected = add(
            *vertex,
            scale(
                directions[index],
                deficit * weights[index] as f64 * deficit_scale,
            ),
        );
        assert_eq!(displaced[index], expected);
    }
}

#[test]
fn apply_brush_strokes_matches_sequential_pipeline() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];
    let first = local_offset_vertices(&vertices, &faces, &[0], 2.0, 0.25, 3.0).unwrap();
    let second = local_offset_vertices(&first, &faces, &[3], 1.5, -0.1, 3.0).unwrap();
    let expected = smooth_vertices_with_falloff(
        &second,
        &faces,
        &[0, 3],
        SmoothFalloffOptions {
            falloff_mm: 2.0,
            iterations: 1,
            strength: 0.25,
            active_threshold: 0.02,
            cutoff_multiplier: 3.0,
        },
    )
    .unwrap();

    let composed = apply_brush_strokes(
        &vertices,
        &faces,
        &[0, 1, 2],
        &[0, 1, 2, 4],
        &[0, 3, 0, 3],
        &[0, 0, 0],
        &[0, 0, 0, 0],
        &[],
        &[0, 0, 0, 0],
        &[],
        &[0.25, 0.1, 0.0],
        &[2.0, 1.5, 2.0],
        &[1, 1, 1],
        &[0.5, 0.5, 0.25],
        3.0,
    )
    .unwrap();

    assert_eq!(composed, expected);
}

#[test]
fn apply_brush_strokes_respects_masks_and_protected_vertices() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let faces = vec![[0, 2, 1], [1, 2, 3]];

    let composed = apply_brush_strokes(
        &vertices,
        &faces,
        &[0],
        &[0, 1],
        &[0],
        &[1],
        &[0, 2],
        &[0, 2],
        &[0, 1],
        &[2],
        &[0.25],
        &[2.0],
        &[1],
        &[0.5],
        3.0,
    )
    .unwrap();

    assert_ne!(composed[0], vertices[0]);
    assert_eq!(composed[1], vertices[1]);
    assert_eq!(composed[2], vertices[2]);
    assert_eq!(composed[3], vertices[3]);
}

#[test]
fn region_brush_masks_respects_allowed_operations_and_overrides() {
    let region_ids = vec![
        "inner_band".to_string(),
        "outer_band".to_string(),
        "head".to_string(),
    ];
    let vertex_offsets = vec![0, 3, 6, 9];
    let vertex_indices = vec![4, 1, 2, 8, 7, 6, 11, 10, 9];
    let allowed_offsets = vec![0, 1, 2, 4];
    let allowed_operations = vec![1, 0, 0, 2];

    let (editable, protected) = region_brush_masks(
        1,
        &region_ids,
        &vertex_offsets,
        &vertex_indices,
        &allowed_offsets,
        &allowed_operations,
        &[],
        &["head".to_string()],
        false,
        true,
        true,
    )
    .unwrap();

    assert_eq!(editable, vec![1, 2, 4]);
    assert_eq!(protected, vec![6, 7, 8, 9, 10, 11]);
}

#[test]
fn region_brush_masks_rejects_unknown_region_id() {
    let region_ids = vec!["inner_band".to_string()];
    let error = region_brush_masks(
        0,
        &region_ids,
        &[0, 1],
        &[0],
        &[0, 1],
        &[0],
        &["missing".to_string()],
        &[],
        true,
        false,
        true,
    )
    .unwrap_err();

    assert!(matches!(error, GeometryError::UnknownRegionIds { ids } if ids == vec!["missing"]));
}

#[test]
fn apply_brush_strokes_rejects_bad_seed_offsets() {
    let (vertices, faces) = cube();
    let error = apply_brush_strokes(
        &vertices,
        &faces,
        &[0],
        &[0],
        &[0],
        &[0],
        &[0, 0],
        &[],
        &[0, 0],
        &[],
        &[0.1],
        &[1.0],
        &[1],
        &[0.5],
        3.0,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GeometryError::BrushSeedOffsetCountMismatch {
            offsets: 1,
            operations: 1
        }
    ));
}

#[test]
fn marching_tetrahedra_extracts_cube_surface() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();

    let mesh = marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();

    assert!(!mesh.vertices.is_empty());
    assert!(!mesh.faces.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|point| point.iter().all(|value| value.is_finite())));
}

#[test]
fn finalized_marching_tetrahedra_repairs_and_orients_cube_surface() {
    let (vertices, faces) = cube();
    let values =
        sdf_grid_values(&vertices, &faces, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.5).unwrap();

    let raw = marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();
    let finalized =
        finalized_marching_tetrahedra(&values, [-1.5, -1.5, -1.5], [7, 7, 7], 0.5, 0.0).unwrap();
    let stats = mesh_stats(&finalized.vertices, &finalized.faces).unwrap();
    let health = mesh_health(
        &finalized.vertices,
        &finalized.faces,
        true,
        Some(50_000),
        1e-8,
    )
    .unwrap();

    assert!(finalized.vertices.len() < raw.vertices.len());
    assert_eq!(finalized.faces.len(), raw.faces.len());
    assert_eq!(stats.boundary_edge_count, 0);
    assert!(health.is_closed);
    assert!(stats.volume_mm3 > 0.0);
}

fn shifted_scaled_tetrahedron(offset: [f64; 3], scale: f64) -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    let vertices = vec![
        [offset[0], offset[1], offset[2]],
        [scale + offset[0], offset[1], offset[2]],
        [offset[0], scale + offset[1], offset[2]],
        [offset[0], offset[1], scale + offset[2]],
    ];
    let faces = vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
    (vertices, faces)
}

#[test]
fn exact_intersection_pipeline_reports_mixed_tetra_surface_contacts() {
    let (first_vertices, first_faces) = shifted_scaled_tetrahedron([0.0, 0.0, 0.0], 2.0);
    let (second_vertices, second_faces) = shifted_scaled_tetrahedron([0.5, 0.5, 0.5], 2.0);

    let intersections = exact_mesh_intersections(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!intersections.is_empty());

    let contours = exact_intersection_contours(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!contours.is_empty());

    let one_mesh_contours = exact_one_mesh_intersection_contours(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!one_mesh_contours.first.is_empty());
    assert_eq!(
        one_mesh_contours.first.len(),
        one_mesh_contours.second.len()
    );

    let cut_meshes = exact_mesh_pair_cut_meshes(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();
    assert!(!cut_meshes.first.cut_edges.is_empty());
    assert!(!cut_meshes.second.cut_edges.is_empty());
}

#[test]
fn marching_tetrahedra_rejects_mismatched_grid_values() {
    let error = marching_tetrahedra(&[1.0], [0.0; 3], [2, 2, 2], 1.0, 0.0).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::SdfValueCountDoesNotMatchShape {
            values: 1,
            shape: [2, 2, 2]
        }
    ));
}

#[test]
fn orient_faces_consistently_flips_shared_same_direction_edges() {
    let faces = vec![[0, 1, 2], [1, 2, 3], [4, 5, 6]];

    let result = orient_faces_consistently(&faces).unwrap();

    assert_eq!(result.faces, vec![[0, 1, 2], [1, 3, 2], [4, 5, 6]]);
    assert_eq!(result.component_offsets, vec![0, 2, 3]);
    assert_eq!(result.component_faces, vec![0, 1, 2]);
}

#[test]
fn orient_faces_consistently_rejects_negative_indices() {
    let error = orient_faces_consistently(&[[0, -1, 2]]).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::NegativeFaceIndex {
            face: 0,
            vertex: -1
        }
    ));
}

#[test]
fn expand_face_selection_to_components_matches_meshlib_per_edge_components() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [4.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [2, 1, 3], [4, 5, 6]];

    let expanded = expand_face_selection_to_components(&vertices, &faces, &[0]).unwrap();

    assert_eq!(expanded, vec![0, 1]);
    assert_eq!(
        expand_face_selection_to_components(&vertices, &faces, &[2]).unwrap(),
        vec![2]
    );
}

#[test]
fn select_largest_component_matches_meshlib_surface_area_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [4.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [2, 1, 3], [4, 5, 6]];

    assert_eq!(
        select_largest_component_faces(&vertices, &faces, 0.0).unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        select_largest_component_faces(&vertices, &faces, 1.1).unwrap(),
        Vec::<i64>::new()
    );
}

#[test]
fn expand_face_selection_to_components_rejects_out_of_range_seed_faces() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let faces = vec![[0, 1, 2]];

    let error = expand_face_selection_to_components(&vertices, &faces, &[3]).unwrap_err();

    assert!(matches!(
        error,
        GeometryError::FaceRegionIndexOutOfBounds {
            index: 3,
            face_count: 1
        }
    ));
}

#[test]
fn select_boundary_faces_matches_meshlib_find_bd_faces_contract() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [5.0, 0.0, 0.0],
        [4.5, 1.0, 0.0],
        [4.5, 0.5, 1.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [2, 1, 3],
        [4, 6, 5],
        [4, 5, 7],
        [5, 6, 7],
        [6, 4, 7],
    ];

    assert_eq!(
        select_boundary_faces(&vertices, &faces).unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        select_boundary_edges(&vertices, &faces).unwrap(),
        vec![[0, 1], [0, 2], [1, 3], [2, 3]]
    );
}

#[test]
fn select_faces_by_screen_polygon_matches_meshlib_lasso_contract() {
    let vertices = vec![
        [-0.8, -0.8, 0.0],
        [-0.2, -0.8, 0.0],
        [-0.8, 0.8, 0.0],
        [0.2, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];
    let view_projection = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];
    let polygon = vec![[-1.0, -1.0], [-0.05, -1.0], [-0.05, 1.0], [-1.0, 1.0]];

    assert_eq!(
        select_faces_by_screen_polygon(&vertices, &faces, &view_projection, &polygon, true, false)
            .unwrap(),
        vec![0]
    );

    let backface = vec![[0, 2, 1]];
    assert_eq!(
        select_faces_by_screen_polygon(
            &vertices[..3],
            &backface,
            &view_projection,
            &polygon,
            true,
            false,
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_screen_polygon(
            &vertices[..3],
            &backface,
            &view_projection,
            &polygon,
            false,
            false,
        )
        .unwrap(),
        Vec::<i64>::new()
    );
}

#[test]
fn select_faces_by_screen_rect_matches_meshlib_rect_contract() {
    let vertices = vec![
        [-0.8, -0.8, 0.0],
        [-0.2, -0.8, 0.0],
        [-0.8, 0.8, 0.0],
        [0.2, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];
    let view_projection = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    assert_eq!(
        select_faces_by_screen_rect(
            &vertices,
            &faces,
            &view_projection,
            [-1.0, -1.0],
            [-0.05, 1.0],
            true,
            false,
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_screen_rect(
            &vertices,
            &faces,
            &view_projection,
            [-0.05, -1.0],
            [1.0, 1.0],
            true,
            false,
        )
        .unwrap(),
        vec![1]
    );

    let backface = vec![[0, 2, 1]];
    assert_eq!(
        select_faces_by_screen_rect(
            &vertices[..3],
            &backface,
            &view_projection,
            [-1.0, -1.0],
            [-0.05, 1.0],
            true,
            false,
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_screen_rect(
            &vertices[..3],
            &backface,
            &view_projection,
            [-1.0, -1.0],
            [-0.05, 1.0],
            false,
            false,
        )
        .unwrap(),
        Vec::<i64>::new()
    );
}

#[test]
fn select_faces_by_screen_brush_matches_meshlib_near_polygon_contract() {
    let vertices = vec![
        [-0.8, -0.8, 0.0],
        [-0.2, -0.8, 0.0],
        [-0.8, 0.8, 0.0],
        [0.2, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
    ];
    let faces = vec![[0, 1, 2], [3, 4, 5]];
    let view_projection = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];
    let brush_path = vec![[-0.9, -0.7], [-0.9, 0.7]];

    assert_eq!(
        select_faces_by_screen_brush(
            &vertices,
            &faces,
            &view_projection,
            &brush_path,
            0.12,
            true,
            false,
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_screen_brush(
            &vertices,
            &faces,
            &view_projection,
            &brush_path,
            0.05,
            true,
            false,
        )
        .unwrap(),
        Vec::<i64>::new()
    );

    let backface = vec![[0, 2, 1]];
    assert_eq!(
        select_faces_by_screen_brush(
            &vertices[..3],
            &backface,
            &view_projection,
            &brush_path,
            0.12,
            true,
            false,
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        select_faces_by_screen_brush(
            &vertices[..3],
            &backface,
            &view_projection,
            &brush_path,
            0.12,
            false,
            false,
        )
        .unwrap(),
        Vec::<i64>::new()
    );
}

#[test]
fn select_point_cloud_points_by_screen_modes_match_meshlib_viewport_area_contract() {
    let points = vec![
        [-0.8, -0.2, 0.0],
        [-0.4, 0.4, 0.0],
        [0.3, 0.0, 0.0],
        [1.4, 0.0, 0.0],
    ];
    let view_projection = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];
    let polygon = vec![[-0.95, -0.35], [-0.25, -0.35], [-0.25, 0.55], [-0.95, 0.55]];

    assert_eq!(
        select_point_cloud_points_by_screen_polygon(
            &points,
            None,
            &view_projection,
            &polygon,
            true,
            false
        )
        .unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        select_point_cloud_points_by_screen_rect(
            &points,
            None,
            &view_projection,
            [-1.0, -0.5],
            [-0.5, 0.1],
            true,
            false
        )
        .unwrap(),
        vec![0]
    );

    let brush_path = vec![[-0.9, -0.4], [-0.9, 0.0]];
    assert_eq!(
        select_point_cloud_points_by_screen_brush(
            &points,
            None,
            &view_projection,
            &brush_path,
            0.12,
            true,
            false
        )
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        select_point_cloud_points_by_screen_brush(
            &points,
            None,
            &view_projection,
            &brush_path,
            0.05,
            true,
            false
        )
        .unwrap(),
        Vec::<i64>::new()
    );

    let backface_normals = vec![
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];
    assert_eq!(
        select_point_cloud_points_by_screen_polygon(
            &points,
            Some(&backface_normals),
            &view_projection,
            &polygon,
            true,
            false,
        )
        .unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        select_point_cloud_points_by_screen_polygon(
            &points,
            Some(&backface_normals),
            &view_projection,
            &polygon,
            false,
            false,
        )
        .unwrap(),
        Vec::<i64>::new()
    );
    assert_eq!(
        select_point_cloud_points_by_screen_polygon(
            &points,
            None,
            &view_projection,
            &polygon,
            false,
            false
        )
        .unwrap(),
        vec![0, 1]
    );
}

#[test]
fn point_cloud_pick_by_ray_matches_meshlib_frontmost_point_pick_contract() {
    let points = vec![
        [0.04, 0.0, 1.0],
        [0.02, 0.0, 2.0],
        [0.2, 0.0, 0.5],
        [0.0, 0.0, -1.0],
    ];

    let selected = point_cloud_pick_by_ray(
        &points,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        0.05,
        10.0,
        None,
        true,
    )
    .unwrap();

    assert_eq!(selected, vec![0]);
    assert_eq!(
        point_cloud_pick_by_ray(
            &points,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            0.01,
            10.0,
            None,
            true,
        )
        .unwrap(),
        Vec::<i64>::new()
    );

    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    assert_eq!(
        point_cloud_pick_by_ray(
            &points,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            0.05,
            10.0,
            Some(&normals),
            false,
        )
        .unwrap(),
        vec![1]
    );

    let error = point_cloud_pick_by_ray(
        &points,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        0.05,
        10.0,
        None,
        true,
    )
    .unwrap_err();
    assert_eq!(error, "ray_direction must have non-zero length");
}

#[test]
fn select_inside_part_faces_matches_meshlib_winding_self_intersection_contract() {
    let outer_vertices = vec![
        [-2.0, -2.0, -2.0],
        [2.0, -2.0, -2.0],
        [2.0, 2.0, -2.0],
        [-2.0, 2.0, -2.0],
        [-2.0, -2.0, 2.0],
        [2.0, -2.0, 2.0],
        [2.0, 2.0, 2.0],
        [-2.0, 2.0, 2.0],
    ];
    let inner_vertices = vec![
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let cube_faces = [
        [0, 3, 2],
        [0, 2, 1],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let mut vertices = outer_vertices;
    vertices.extend(inner_vertices);
    let mut faces = cube_faces.to_vec();
    faces.extend(
        cube_faces
            .iter()
            .map(|face| [face[0] + 8, face[1] + 8, face[2] + 8]),
    );

    assert_eq!(
        select_inside_part_faces(&vertices, &faces).unwrap(),
        (12_i64..24_i64).collect::<Vec<_>>()
    );
}

#[test]
fn select_camera_facing_faces_matches_meshinspector_view_direction_contract() {
    let vertices = vec![
        [-1.0, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [1.0, 1.0, 0.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, -1.0, -1.0],
        [0.0, 1.0, -1.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![
        [0, 1, 2], // normal +Z, facing a camera looking down -Z
        [3, 5, 4], // normal -Z, facing a camera looking up +Z
        [6, 7, 8], // normal +X, tangent to a Z camera direction
    ];

    assert_eq!(
        select_camera_facing_faces(&vertices, &faces, [0.0, 0.0, -1.0], 0.0).unwrap(),
        vec![0],
    );
    assert_eq!(
        select_camera_facing_faces(&vertices, &faces, [0.0, 0.0, 1.0], 0.0).unwrap(),
        vec![1],
    );
    assert_eq!(
        select_camera_facing_faces(&vertices, &faces, [0.0, 0.0, -1.0], 0.5).unwrap(),
        vec![0],
    );
    assert!(select_camera_facing_faces(&vertices, &faces, [0.0, 0.0, 0.0], 0.0).is_err());
}

#[test]
fn select_faces_by_screen_polygon_samples_large_triangles_like_meshlib() {
    let vertices = vec![[-0.9, -0.9, 0.0], [0.9, -0.9, 0.0], [0.0, 0.9, 0.0]];
    let faces = vec![[0, 1, 2]];
    let view_projection = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];
    let polygon = vec![[-0.1, -0.1], [0.1, -0.1], [0.1, 0.1], [-0.1, 0.1]];

    assert_eq!(
        select_faces_by_screen_polygon(&vertices, &faces, &view_projection, &polygon, true, false)
            .unwrap(),
        vec![0]
    );
}

fn exact_cut_triangle() -> (Vec<[f64; 3]>, Vec<[i64; 3]>) {
    (
        vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
        vec![[0, 1, 2]],
    )
}

#[test]
fn exact_cut_mesh_splits_vertex_to_opposite_edge_triangle() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [0.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 4);
    assert_eq!(result.faces.len(), 2);
    assert_eq!(result.cut_edges, vec![[0, 3]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_edge_to_edge_triangle() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([2, 0]),
                coordinate: [0.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 3);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_interior_face_point_to_edge_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.4, 0.4, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 4);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_edge_point_to_interior_face_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.4, 0.4, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 4);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_interior_face_point_to_interior_face_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.4, 0.4, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.8, 0.6, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 5);
    assert_eq!(result.faces.len(), 5);
    assert_eq!(result.cut_edges, vec![[3, 4]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_cut_mesh_splits_two_boundary_spokes_to_shared_interior_point() {
    let (vertices, faces) = exact_cut_triangle();
    let contours = vec![ExactOneMeshContour {
        intersections: vec![
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([0, 1]),
                coordinate: [1.0, 0.0, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Face(0),
                coordinate: [0.5, 0.5, 0.0],
            },
            ExactOneMeshIntersection {
                primitive: ExactOneMeshPrimitive::Edge([1, 2]),
                coordinate: [1.0, 1.0, 0.0],
            },
        ],
        closed: false,
    }];

    let result = exact_cut_mesh_by_contours(&vertices, &faces, &contours, 1e-9).unwrap();

    assert_eq!(result.vertices.len(), 6);
    assert_eq!(result.faces.len(), 5);
    assert_eq!(result.cut_edges, vec![[3, 4], [4, 5]]);
    assert_eq!(result.source_face_for_faces, vec![0, 0, 0, 0, 0]);
    assert!(result.skipped_source_faces.is_empty());
}

#[test]
fn exact_mesh_pair_cut_meshes_return_operand_results() {
    let first_vertices = vec![[2.0, 1.0, 0.0], [-2.0, 1.0, 0.0], [0.0, -2.0, 0.0]];
    let first_faces = vec![[0, 1, 2]];
    let second_vertices = vec![[0.0, 0.0, -1.0], [0.0, 0.0, 1.0], [3.0, 0.0, 0.0]];
    let second_faces = vec![[0, 1, 2]];

    let result = exact_mesh_pair_cut_meshes(
        &first_vertices,
        &first_faces,
        &second_vertices,
        &second_faces,
        8,
        1e-9,
    )
    .unwrap();

    assert!(result.first.vertices.len() >= first_vertices.len());
    assert!(result.second.vertices.len() >= second_vertices.len());
    assert!(!result.first.cut_edges.is_empty() || !result.first.skipped_source_faces.is_empty());
    assert!(!result.second.cut_edges.is_empty() || !result.second.skipped_source_faces.is_empty());
}

#[test]
fn exact_planar_hole_fill_plan_triangulates_loop_and_preserves_source_face() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 2.0, 0.0],
    ];

    let plan = exact_planar_hole_fill_plan(&vertices, &[0, 1, 2, 3], 1e-9).unwrap();
    let execution = execute_exact_planar_hole_fill_plan(&plan, 42);

    assert_eq!(plan.boundary_loop, vec![0, 1, 2, 3]);
    assert_eq!(plan.num_tris, 2);
    assert_eq!(execution.faces.len(), 2);
    assert_eq!(execution.source_face_for_faces, vec![42, 42]);
}

#[test]
fn exact_planar_hole_fill_plan_rejects_degenerate_loop() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];

    assert!(exact_planar_hole_fill_plan(&vertices, &[0, 1, 2], 1e-9).is_none());
}

fn exact_cut_mesh_with_square_cut_hole() -> ExactCutMeshResult {
    ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [2.0, -1.0, 0.0],
            [2.0, 2.0, 0.0],
            [-1.0, 2.0, 0.0],
        ],
        faces: vec![[4, 0, 1], [5, 1, 2], [6, 2, 3], [7, 3, 0]],
        cut_edges: vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        cut_edge_paths: vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
        cut_edge_path_closed: vec![true],
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![100, 101, 102, 103],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    }
}

fn exact_cut_mesh_with_two_sided_closed_square_cut() -> ExactCutMeshResult {
    ExactCutMeshResult {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0],
            [2.0, -1.0, 0.0],
            [2.0, 2.0, 0.0],
            [-1.0, 2.0, 0.0],
            [0.5, -0.5, 0.0],
            [1.5, 0.5, 0.0],
            [0.5, 1.5, 0.0],
            [-0.5, 0.5, 0.0],
        ],
        faces: vec![
            [4, 0, 1],
            [8, 1, 0],
            [5, 1, 2],
            [9, 2, 1],
            [6, 2, 3],
            [10, 3, 2],
            [7, 3, 0],
            [11, 0, 3],
        ],
        cut_edges: vec![[0, 1], [1, 2], [2, 3], [0, 3]],
        cut_edge_paths: vec![vec![[0, 1], [1, 2], [2, 3], [3, 0]]],
        cut_edge_path_closed: vec![true],
        cut_edge_path_source_faces: Vec::new(),
        collapsed_cut_segment_paths: Vec::new(),
        collapsed_cut_segment_path_source_faces: Vec::new(),
        source_face_for_faces: vec![100, 200, 101, 201, 102, 202, 103, 203],
        cut_face_source_events: Vec::new(),
        skipped_source_faces: Vec::new(),
    }
}

#[test]
fn exact_cut_hole_fill_plans_discover_cut_boundary_loop() {
    let cut_mesh = exact_cut_mesh_with_square_cut_hole();

    let plans = exact_cut_hole_fill_plans(&cut_mesh, 1e-9).unwrap();

    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].representative_edge, [1, 0]);
    assert_eq!(plans[0].boundary_loop, vec![1, 0, 3, 2]);
    assert_eq!(
        plans[0].boundary_edges,
        vec![[1, 0], [0, 3], [3, 2], [2, 1]]
    );
    assert_eq!(plans[0].source_face, 100);
    assert_eq!(plans[0].fill_plan.num_tris, 2);
}

#[test]
fn exact_cut_hole_fill_plans_replace_two_sided_closed_cut_loop() {
    let cut_mesh = exact_cut_mesh_with_two_sided_closed_square_cut();

    assert!(exact_cut_hole_fill_plans(&cut_mesh, 1e-9)
        .unwrap()
        .is_empty());

    let plans = exact_cut_hole_fill_plans_with_replacements(&cut_mesh, 1e-9).unwrap();

    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].representative_edge, [0, 1]);
    assert_eq!(plans[0].boundary_loop, vec![0, 1, 2, 3]);
    assert_eq!(
        plans[0].boundary_edges,
        vec![[0, 1], [1, 2], [2, 3], [3, 0]]
    );
    assert_eq!(plans[0].source_face, 100);
    assert_eq!(plans[0].fill_plan.num_tris, 2);
}

#[test]
fn exact_fill_cut_holes_appends_plan_faces_with_source_mapping() {
    let cut_mesh = exact_cut_mesh_with_square_cut_hole();

    let result = exact_fill_cut_holes(&cut_mesh, 1e-9).unwrap();

    assert_eq!(result.fill_plans.len(), 1);
    assert_eq!(result.added_face_ranges, vec![[cut_mesh.faces.len(), 6]]);
    assert_eq!(result.mesh.vertices, cut_mesh.vertices);
    assert_eq!(result.mesh.faces.len(), cut_mesh.faces.len() + 2);
    assert_eq!(
        &result.mesh.source_face_for_faces[cut_mesh.source_face_for_faces.len()..],
        &[100, 100]
    );
    assert_eq!(result.mesh.cut_edges, cut_mesh.cut_edges);
}

#[test]
fn subdivide_mesh_matches_meshlib_square_region_split_counts() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let first = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.3,
            max_edge_splits: 1000,
            region_faces: Some(vec![0]),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(first.splits_done, 22);
    assert_eq!(first.mesh.vertices.len(), 26);
    assert_eq!(first.mesh.faces.len(), 40);
    assert_eq!(first.region_face_count, 32);

    let limited = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.3,
            max_edge_splits: 10,
            region_faces: Some(vec![0]),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(limited.splits_done, 10);
    assert_eq!(limited.mesh.vertices.len(), 14);
    assert_eq!(limited.mesh.faces.len(), 18);
    assert_eq!(limited.region_face_count, 14);
}

#[test]
fn subdivide_full_region_fast_path_matches_linear_path() {
    // On a closed mesh there are no border edges, so `subdivide_border` has no
    // effect on which edges are candidates. The full-region path with
    // subdivide_border=true takes the EdgeState-scan fast path; with
    // subdivide_border=false it takes the original edge_incident_faces linear
    // path. On this closed octahedron both must therefore produce byte-identical
    // output — proving the fast path is bit-for-bit equivalent.
    let vertices = vec![
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    let faces = vec![
        [0_i64, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [2, 0, 5],
        [1, 2, 5],
        [3, 1, 5],
        [0, 3, 5],
    ];
    let opts = |subdivide_border| SubdivideMeshOptions {
        max_edge_len: 0.5,
        max_edge_splits: 2000,
        subdivide_border,
        ..SubdivideMeshOptions::default()
    };
    let fast = subdivide_mesh(&vertices, &faces, opts(true)).unwrap();
    let linear = subdivide_mesh(&vertices, &faces, opts(false)).unwrap();

    assert!(
        fast.splits_done > 30,
        "expected meaningful subdivision, got {}",
        fast.splits_done
    );
    assert_eq!(fast.splits_done, linear.splits_done);
    assert_eq!(fast.mesh.faces, linear.mesh.faces);
    assert_eq!(fast.mesh.vertices, linear.mesh.vertices);
}

#[test]
fn subdivide_mesh_projects_new_vertices_to_unit_sphere_like_meshlib_make_sphere_callback() {
    let vertices = vec![
        normalize([-0.5, -0.5, -0.5]).unwrap(),
        normalize([-0.5, 0.5, -0.5]).unwrap(),
        normalize([0.5, 0.5, -0.5]).unwrap(),
        normalize([0.5, -0.5, -0.5]).unwrap(),
        normalize([-0.5, -0.5, 0.5]).unwrap(),
        normalize([-0.5, 0.5, 0.5]).unwrap(),
        normalize([0.5, 0.5, 0.5]).unwrap(),
        normalize([0.5, -0.5, 0.5]).unwrap(),
    ];
    let faces = vec![
        [0_i64, 1, 2],
        [2, 3, 0],
        [0, 4, 5],
        [5, 1, 0],
        [0, 3, 7],
        [7, 4, 0],
        [6, 5, 4],
        [4, 7, 6],
        [1, 5, 6],
        [6, 2, 1],
        [6, 7, 3],
        [3, 2, 6],
    ];

    let result = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 6,
            project_new_vertices_to_unit_sphere: true,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.splits_done, 6);
    assert_eq!(result.mesh.vertices.len(), 14);
    for vertex in &result.mesh.vertices[8..] {
        let radius = (vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2]).sqrt();
        assert!((radius - 1.0).abs() < 1e-12);
    }
}

fn subdivide_delone_guard_vertices() -> Vec<[f64; 3]> {
    vec![
        [2.6276049261498553, 2.9361648936968914, 0.7212656061740566],
        [2.0369637564197727, 0.16430872643309868, 1.5154317688237702],
        [1.6171991057049149, 0.5114846825888412, 1.9134006098472023],
        [0.7822080654816956, 1.7910118323907225, 0.21890750193281283],
    ]
}

fn mesh_faces_have_edge_i64(faces: &[[i64; 3]], edge: [i64; 2]) -> bool {
    let ordered = if edge[0] <= edge[1] {
        edge
    } else {
        [edge[1], edge[0]]
    };
    faces.iter().any(|face| {
        [[face[0], face[1]], [face[1], face[2]], [face[2], face[0]]]
            .into_iter()
            .map(|face_edge| {
                if face_edge[0] <= face_edge[1] {
                    face_edge
                } else {
                    [face_edge[1], face_edge[0]]
                }
            })
            .any(|face_edge| face_edge == ordered)
    })
}

#[test]
fn subdivide_mesh_honors_meshlib_not_flippable_delone_guard() {
    let vertices = subdivide_delone_guard_vertices();
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let unprotected = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.01,
            max_edge_splits: 1,
            region_faces: Some(vec![0, 1]),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();
    let protected = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.01,
            max_edge_splits: 1,
            region_faces: Some(vec![0, 1]),
            not_flippable_edges: vec![[0, 2]],
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(unprotected.splits_done, 1);
    assert_eq!(protected.splits_done, 1);
    assert!(!mesh_faces_have_edge_i64(&unprotected.mesh.faces, [0, 2]));
    assert!(mesh_faces_have_edge_i64(&protected.mesh.faces, [0, 2]));
}

#[test]
fn subdivide_mesh_honors_meshlib_max_deviation_after_flip() {
    let vertices = subdivide_delone_guard_vertices();
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let unconstrained = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.01,
            max_edge_splits: 1,
            region_faces: Some(vec![0, 1]),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();
    let constrained = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.01,
            max_edge_splits: 1,
            region_faces: Some(vec![0, 1]),
            max_deviation_after_flip: Some(0.01),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(unconstrained.splits_done, 1);
    assert_eq!(constrained.splits_done, 1);
    assert!(!mesh_faces_have_edge_i64(&unconstrained.mesh.faces, [0, 2]));
    assert!(mesh_faces_have_edge_i64(&constrained.mesh.faces, [0, 2]));
}

#[test]
fn subdivide_mesh_honors_meshlib_max_angle_change_and_critical_aspect_flip() {
    let vertices = subdivide_delone_guard_vertices();
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let angle_constrained = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.01,
            max_edge_splits: 1,
            region_faces: Some(vec![0, 1]),
            max_angle_change_after_flip: Some(0.01),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();
    let aspect_critical = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.01,
            max_edge_splits: 1,
            region_faces: Some(vec![0, 1]),
            max_angle_change_after_flip: Some(0.01),
            critical_tri_aspect_ratio_flip: Some(1.0),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(angle_constrained.splits_done, 1);
    assert_eq!(aspect_critical.splits_done, 1);
    assert!(mesh_faces_have_edge_i64(
        &angle_constrained.mesh.faces,
        [0, 2]
    ));
    assert!(!mesh_faces_have_edge_i64(
        &aspect_critical.mesh.faces,
        [0, 2]
    ));
}

#[test]
fn offset_verts_mesh_shifts_vertices_along_meshlib_pseudonormals() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let offsets = vec![0.10_f32, 0.20, 0.00, -0.05];

    let result = offset_verts_mesh(&vertices, &faces, &offsets).unwrap();

    assert_eq!(result.faces, faces);
    for (index, offset) in offsets.iter().enumerate() {
        assert!((result.vertices[index][0] - vertices[index][0]).abs() <= 1e-12);
        assert!((result.vertices[index][1] - vertices[index][1]).abs() <= 1e-12);
        assert!((result.vertices[index][2] - *offset as f64).abs() <= 1e-12);
    }
}

#[test]
fn decimate_mesh_default_settings_match_meshlib_half_face_guard() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let result = decimate_mesh(&vertices, &faces, DecimateMeshOptions::default()).unwrap();

    assert_eq!(result.faces_deleted, faces.len() / 2);
    assert_eq!(result.mesh.faces.len(), faces.len() - faces.len() / 2);
}

#[test]
fn decimate_mesh_target_face_count_maps_to_deleted_face_limit() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            target_face_count: Some(3),
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.faces_deleted, 0);
    assert_eq!(result.mesh.faces.len(), faces.len());
}

#[test]
fn decimate_mesh_target_face_ratio_maps_to_deleted_face_limit() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            target_face_ratio: Some(0.5),
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.faces_deleted, 2);
    assert_eq!(result.mesh.faces.len(), 2);
}

#[test]
fn decimate_mesh_subdivide_parts_can_preserve_part_boundaries() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            max_deleted_faces: 2,
            subdivide_parts: 2,
            decimate_between_parts: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.faces_deleted, 0);
    assert_eq!(result.mesh.faces.len(), faces.len());
}

#[test]
fn decimate_mesh_subdivide_parts_final_between_parts_pass_decimates_boundary() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            max_deleted_faces: 2,
            subdivide_parts: 2,
            decimate_between_parts: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.faces_deleted, 2);
    assert_eq!(result.mesh.faces.len(), 2);
}

#[test]
fn decimate_mesh_shortest_edge_first_stops_when_shortest_edge_exceeds_max_error() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.1,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 0);
    assert_eq!(result.faces_deleted, 0);
    assert_eq!(result.mesh.vertices, vertices);
    assert_eq!(result.mesh.faces, faces);
    assert!(!result.cancelled);
}

#[test]
fn decimate_mesh_shortest_edge_first_collapses_and_packs_short_edge() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            max_deleted_vertices: 1,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert!(!result.cancelled);
    assert_eq!(result.mesh.vertices.len(), 3);
    assert_eq!(result.mesh.faces, vec![[0, 1, 2]]);
    assert_eq!(result.mesh.vertices[0], [0.05, 0.0, 0.0]);
    assert_eq!(result.mesh.vertices[1], [1.0, 0.0, 0.0]);
    assert_eq!(result.mesh.vertices[2], [0.0, 1.0, 0.0]);
}

#[test]
fn decimate_mesh_minimize_error_uses_qem_deviation_instead_of_edge_length() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::MinimizeError,
            max_error: 0.9,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert!(result.error_introduced <= 0.9);
    assert!(!result.cancelled);
    assert!(result.mesh.faces.len() <= 1);
}

fn assert_point_close(actual: [f64; 3], expected: [f64; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 1e-5,
            "expected {actual:?} to be within 1e-5 of {expected:?}",
        );
    }
}

#[test]
fn decimate_mesh_honors_meshlib_angle_weighted_face_plane_qem() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.705_869, 0.340_508, 0.212_305],
        [-0.187_224, 1.464_488, 0.373_414],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 1, 3], [1, 4, 2], [0, 3, 4]];

    let unweighted = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::MinimizeError,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            angle_weighted_dist_to_plane: false,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let weighted = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::MinimizeError,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            angle_weighted_dist_to_plane: true,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(unweighted.verts_deleted, 1);
    assert_eq!(weighted.verts_deleted, 1);
    assert_ne!(unweighted.mesh.faces, weighted.mesh.faces);
    assert_point_close(
        unweighted.mesh.vertices[2],
        [-0.073_394, 1.267_286, 0.219_714],
    );
    assert_point_close(weighted.mesh.vertices[0], [-0.021_75, 0.950_335, 0.066_332]);
}

#[test]
fn decimate_mesh_honors_meshlib_qem_stabilizer() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-0.355_003, 0.768_589, -0.355_005],
        [0.584_114, -0.325_361, -0.291_144],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 1, 3], [1, 4, 2], [0, 3, 4]];

    let default_stabilizer = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::MinimizeError,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            stabilizer: 0.001,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let strong_stabilizer = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::MinimizeError,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            stabilizer: 1.0,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(default_stabilizer.verts_deleted, 1);
    assert_eq!(strong_stabilizer.verts_deleted, 1);
    assert_ne!(default_stabilizer.mesh.faces, strong_stabilizer.mesh.faces);
    assert_point_close(
        default_stabilizer.mesh.vertices[0],
        [0.096_007, 0.129_607, -0.067_989],
    );
    assert_point_close(
        strong_stabilizer.mesh.vertices[0],
        [-0.086_09, 0.337_451, -0.137_335],
    );
}

#[test]
fn decimate_mesh_honors_meshlib_max_triangle_aspect_ratio_guard() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.090_871_211_463_571_5, 0.0],
        [2.0, 0.0, 0.0],
        [1.5, 0.866_025_403_784_438_6, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [1, 3, 4]];

    let blocked = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 1.1,
            max_triangle_aspect_ratio: 1.05,
            max_deleted_vertices: 1,
            region_faces: Some(vec![0]),
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let allowed = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 1.1,
            max_triangle_aspect_ratio: 2.0,
            max_deleted_vertices: 1,
            region_faces: Some(vec![0]),
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(blocked.verts_deleted, 0);
    assert_eq!(blocked.faces_deleted, 0);
    assert_eq!(allowed.verts_deleted, 1);
    assert_eq!(allowed.faces_deleted, 1);
    assert_eq!(allowed.mesh.vertices.len(), 3);
    assert_eq!(allowed.mesh.faces.len(), 1);
}

#[test]
fn decimate_mesh_honors_meshlib_critical_triangle_aspect_ratio_relaxation() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.090_871_211_463_571_5, 0.0],
        [2.0, 0.0, 0.0],
        [1.5, 0.866_025_403_784_438_6, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [1, 3, 4]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 1.1,
            max_triangle_aspect_ratio: 1.05,
            critical_tri_aspect_ratio: 1.0,
            max_deleted_vertices: 1,
            region_faces: Some(vec![0]),
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.vertices.len(), 3);
    assert_eq!(result.mesh.faces.len(), 1);
}

#[test]
fn decimate_mesh_honors_meshlib_tiny_edge_length_aspect_bypass() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.5, 1.090_871_211_463_571_5, 0.0],
        [2.0, 0.0, 0.0],
        [1.5, 0.866_025_403_784_438_6, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [1, 3, 4]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 1.1,
            max_triangle_aspect_ratio: 1.05,
            tiny_edge_length: 1.1,
            optimize_vertex_pos: false,
            max_deleted_vertices: 1,
            region_faces: Some(vec![0]),
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.vertices.len(), 3);
    assert_eq!(result.mesh.faces.len(), 1);
}

#[test]
fn decimate_mesh_honors_meshlib_max_angle_change_delone_flip() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.1,
            max_angle_change: 0.0,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 0);
    assert_eq!(result.faces_deleted, 0);
    assert_eq!(result.mesh.vertices, vertices);
    assert_eq!(result.mesh.faces, vec![[1, 3, 0], [3, 1, 2]]);
}

#[test]
fn decimate_mesh_flips_meshlib_twin_edge_with_max_angle_change() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 1.0, 0.0],
        [4.0, 0.0, 0.0],
        [6.0, 0.0, 0.0],
        [6.0, 2.0, 0.0],
        [4.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.1,
            max_angle_change: 0.0,
            max_deleted_vertices: 1,
            max_deleted_faces: 2,
            twin_map: vec![[[0, 2], [4, 6]], [[4, 6], [0, 2]]],
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 0);
    assert_eq!(result.faces_deleted, 0);
    assert_eq!(result.mesh.vertices, vertices);
    assert_eq!(
        result.mesh.faces,
        vec![[1, 3, 0], [3, 1, 2], [5, 7, 4], [7, 5, 6]]
    );
    assert_eq!(result.twin_map, vec![[[1, 3], [5, 7]], [[5, 7], [1, 3]]]);
}

#[test]
fn decimate_mesh_honors_meshlib_touch_near_boundary_edges_false() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let blocked = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 2.0,
            touch_near_bd_edges: false,
            max_deleted_vertices: 1,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let allowed = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 2.0,
            touch_near_bd_edges: true,
            max_deleted_vertices: 1,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(blocked.verts_deleted, 0);
    assert_eq!(blocked.faces_deleted, 0);
    assert_eq!(blocked.mesh.vertices, vertices);
    assert_eq!(blocked.mesh.faces, faces);
    assert_eq!(allowed.verts_deleted, 1);
    assert_eq!(allowed.faces_deleted, 1);
    assert_eq!(allowed.mesh.vertices.len(), 3);
    assert_eq!(allowed.mesh.faces.len(), 1);
}

#[test]
fn decimate_mesh_honors_meshlib_touch_boundary_vertices_false() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let preserve_boundary = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 1.0,
            touch_bd_verts: false,
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let move_boundary = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 1.0,
            touch_bd_verts: true,
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(preserve_boundary.verts_deleted, 1);
    assert_eq!(preserve_boundary.faces_deleted, 2);
    assert_eq!(preserve_boundary.mesh.vertices[0], vertices[0]);
    assert_ne!(move_boundary.mesh.vertices[0], vertices[0]);
    assert_eq!(move_boundary.mesh.vertices[0], [0.25, 0.25, 0.0]);
}

#[test]
fn decimate_mesh_honors_meshlib_max_boundary_shift_guard() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5, 0.0],
    ];
    let faces = vec![[0_i64, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];

    let blocked = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.8,
            max_bd_shift: 0.2,
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let allowed = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.8,
            max_bd_shift: 0.3,
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(blocked.verts_deleted, 0);
    assert_eq!(blocked.faces_deleted, 0);
    assert_eq!(blocked.mesh.vertices, vertices);
    assert_eq!(blocked.mesh.faces, faces);
    assert_eq!(allowed.verts_deleted, 1);
    assert_eq!(allowed.faces_deleted, 2);
    assert_eq!(allowed.mesh.vertices[0], [0.25, 0.25, 0.0]);
}

#[test]
fn decimate_mesh_honors_meshlib_not_flippable_adjacent_collapse_guard() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let blocked = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            not_flippable_edges: vec![[1, 3]],
            collapse_near_not_flippable: false,
            max_deleted_vertices: 1,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    let allowed = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            not_flippable_edges: vec![[1, 3]],
            collapse_near_not_flippable: true,
            max_deleted_vertices: 1,
            pack_mesh: true,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(blocked.verts_deleted, 0);
    assert_eq!(blocked.faces_deleted, 0);
    assert_eq!(allowed.verts_deleted, 1);
    assert_eq!(allowed.faces_deleted, 1);
    assert_eq!(allowed.mesh.vertices.len(), 3);
    assert_eq!(allowed.mesh.faces.len(), 1);
}

#[test]
fn decimate_mesh_reports_meshlib_remapped_not_flippable_edges_after_collapse() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            not_flippable_edges: vec![[1, 3]],
            collapse_near_not_flippable: true,
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.faces, vec![[0_i64, 2, 3]]);
    assert_eq!(result.not_flippable_edges, vec![[0, 3]]);
}

#[test]
fn decimate_mesh_honors_meshlib_edges_to_collapse_subset_and_remaps_it() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.05, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [2.0, 0.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3], [2, 4, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 2.0,
            edges_to_collapse: Some(vec![[1, 2]]),
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.faces, vec![[0_i64, 1, 3], [1, 4, 3]]);
    assert_eq!(result.mesh.vertices[1], [0.525, 0.0, 0.0]);
    assert!(result.edges_to_collapse.is_empty());
}

#[test]
fn decimate_mesh_honors_empty_meshlib_edges_to_collapse_subset() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.05, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 2.0,
            edges_to_collapse: Some(Vec::new()),
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 0);
    assert_eq!(result.faces_deleted, 0);
    assert_eq!(result.mesh.faces, faces);
    assert!(result.edges_to_collapse.is_empty());
}

#[test]
fn decimate_mesh_remaps_meshlib_twin_map_after_collapse() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            twin_map: vec![[[1, 3], [1, 2]], [[1, 2], [1, 3]]],
            max_deleted_vertices: 1,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.faces, vec![[0_i64, 2, 3]]);
    assert_eq!(result.twin_map, vec![[[0, 2], [0, 3]], [[0, 3], [0, 2]]]);
}

#[test]
fn decimate_mesh_collapses_meshlib_twin_edge_with_same_position() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [10.0, 0.0, 0.0],
        [10.15, 0.0, 0.0],
        [11.0, 0.0, 0.0],
        [10.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3], [4, 5, 7], [5, 6, 7]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            twin_map: vec![[[0, 1], [4, 5]], [[4, 5], [0, 1]]],
            max_deleted_vertices: 1,
            max_triangle_aspect_ratio: 1_000_000.0,
            pack_mesh: false,
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 2);
    assert_eq!(result.faces_deleted, 2);
    assert_eq!(result.mesh.faces, vec![[0_i64, 2, 3], [4, 6, 7]]);
    assert_eq!(result.mesh.vertices[0], [0.05, 0.0, 0.0]);
    assert_eq!(result.mesh.vertices[4], [0.05, 0.0, 0.0]);
    assert!(result.twin_map.is_empty());
}

#[test]
fn decimate_mesh_interpolates_vertex_uvs_with_meshlib_pre_collapse_callback() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            max_deleted_vertices: 1,
            pack_mesh: true,
            vertex_uvs: Some(vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [0.0, 1.0]]),
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.faces, vec![[0_i64, 1, 2]]);
    assert_eq!(
        result.vertex_uvs,
        Some(vec![[0.5, 0.0], [2.0, 0.0], [0.0, 1.0]])
    );
}

#[test]
fn decimate_mesh_interpolates_vertex_colors_with_meshlib_pre_collapse_truncation() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 3], [1, 2, 3]];

    let result = decimate_mesh(
        &vertices,
        &faces,
        DecimateMeshOptions {
            strategy: DecimateMeshStrategy::ShortestEdgeFirst,
            max_error: 0.2,
            max_deleted_vertices: 1,
            pack_mesh: true,
            vertex_colors: Some(vec![
                [1, 10, 100, 255],
                [3, 20, 200, 127],
                [5, 30, 210, 255],
                [7, 40, 220, 255],
            ]),
            ..DecimateMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(result.verts_deleted, 1);
    assert_eq!(result.faces_deleted, 1);
    assert_eq!(result.mesh.faces, vec![[0_i64, 1, 2]]);
    assert_eq!(
        result.vertex_colors,
        Some(vec![
            [1, 15, 150, 190],
            [5, 30, 210, 255],
            [7, 40, 220, 255]
        ])
    );
}

#[test]
fn subdivide_mesh_matches_meshlib_chained_square_region_split_counts() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let first = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.3,
            max_edge_splits: 1000,
            region_faces: Some(vec![0]),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();
    let second = subdivide_mesh(
        &first.mesh.vertices,
        &first.mesh.faces,
        SubdivideMeshOptions {
            max_edge_len: 0.1,
            max_edge_splits: 10,
            region_faces: Some(first.region_faces),
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(second.splits_done, 10);
    assert_eq!(second.mesh.vertices.len(), 36);
    assert_eq!(second.mesh.faces.len(), 57);
    assert_eq!(second.region_face_count, 49);
}

#[test]
fn subdivide_mesh_honors_meshlib_max_tri_aspect_ratio_stop() {
    let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let faces = vec![[0_i64, 1, 2]];

    let already_below_threshold = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 10,
            max_tri_aspect_ratio: 1.3,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(already_below_threshold.splits_done, 0);
    assert_eq!(already_below_threshold.mesh.vertices.len(), 3);
    assert_eq!(already_below_threshold.mesh.faces.len(), 1);

    let still_above_threshold = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 10,
            max_tri_aspect_ratio: 1.1,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(still_above_threshold.splits_done, 10);
    assert_eq!(still_above_threshold.mesh.vertices.len(), 13);
    assert_eq!(still_above_threshold.mesh.faces.len(), 14);
}

#[test]
fn subdivide_mesh_honors_meshlib_max_splittable_tri_aspect_ratio_gate() {
    let vertices = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let faces = vec![[0_i64, 1, 2]];

    let blocked = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 1.0,
            max_edge_splits: 10,
            max_splittable_tri_aspect_ratio: 5.0,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(blocked.splits_done, 0);
    assert_eq!(blocked.mesh.vertices.len(), 3);
    assert_eq!(blocked.mesh.faces.len(), 1);

    let allowed = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 1.0,
            max_edge_splits: 10,
            max_splittable_tri_aspect_ratio: 6.0,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(allowed.splits_done, 10);
    assert_eq!(allowed.mesh.vertices.len(), 13);
    assert_eq!(allowed.mesh.faces.len(), 12);
}

#[test]
fn subdivide_mesh_honors_meshlib_curvature_priority_edge_ranking() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 10.0, 0.0],
        [2.0, 10.0, 0.0],
        [0.0, 12.0, 0.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3], [4, 5, 6]];

    let flat_priority = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 1,
            curvature_priority: 0.0,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(flat_priority.splits_done, 1);
    assert_eq!(
        flat_priority.mesh.vertices.last().copied().unwrap(),
        [1.0, 11.0, 0.0]
    );

    let curvature_priority = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 1,
            curvature_priority: 5.0,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(curvature_priority.splits_done, 1);
    assert_eq!(
        curvature_priority.mesh.vertices.last().copied().unwrap(),
        [0.0, 0.5, 0.5]
    );
}

#[test]
fn subdivide_mesh_honors_meshlib_project_on_original_mesh() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let unprojected = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 3,
            project_on_original_mesh: false,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(unprojected.splits_done, 3);
    assert_eq!(
        unprojected.mesh.vertices.last().copied().unwrap(),
        [0.75, 0.5, 0.25]
    );

    let projected = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 3,
            project_on_original_mesh: true,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(projected.splits_done, 3);
    assert_eq!(
        projected.mesh.vertices.last().copied().unwrap(),
        [0.75, 0.5, 0.0]
    );
    assert_eq!(projected.mesh.faces, unprojected.mesh.faces);
}

#[test]
fn subdivide_mesh_honors_meshlib_smooth_mode_without_sharp_constraints() {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![[0_i64, 1, 2], [0, 2, 3]];

    let unsmoothed = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 3,
            smooth_mode: false,
            min_sharp_dihedral_angle: 999.0,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(unsmoothed.splits_done, 3);
    assert_eq!(
        unsmoothed.mesh.vertices.last().copied().unwrap(),
        [0.75, 0.5, 0.25]
    );

    let smoothed = subdivide_mesh(
        &vertices,
        &faces,
        SubdivideMeshOptions {
            max_edge_len: 0.0,
            max_edge_splits: 3,
            smooth_mode: true,
            min_sharp_dihedral_angle: 999.0,
            ..SubdivideMeshOptions::default()
        },
    )
    .unwrap();

    assert_eq!(smoothed.splits_done, 3);
    let last = smoothed.mesh.vertices.last().copied().unwrap();
    assert!((last[0] - 0.873372078).abs() < 1e-6, "{last:?}");
    assert!((last[1] - 0.47443521).abs() < 1e-6, "{last:?}");
    assert!((last[2] - 0.031970274).abs() < 1e-6, "{last:?}");
    assert_eq!(smoothed.mesh.faces, unsmoothed.mesh.faces);
}

#[test]
fn mesh_obj_import_triangulates_meshlib_negative_index_quad() {
    let object = mesh_from_obj(
        b"o relative_quad\n\
          v 0 0 0\n\
          v 1 0 0\n\
          v 1 1 0\n\
          v 0 1 0\n\
          f -4 -3 -2 -1\n",
    )
    .unwrap();

    assert_eq!(
        object.vertices,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]
    );
    assert_eq!(object.faces, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(object.object_names, vec!["relative_quad"]);
}

#[test]
fn mesh_obj_import_loads_meshlib_mtl_diffuse_texture_metadata() {
    let material_dir = std::env::temp_dir().join(format!(
        "zennah_obj_mtl_{}_{}",
        std::process::id(),
        "diffuse_texture"
    ));
    std::fs::create_dir_all(&material_dir).unwrap();
    std::fs::write(
        material_dir.join("jewel.mtl"),
        "newmtl polished_gold\nKd 0.2 0.4 0.6\nmap_Kd -clamp on albedo.png\n",
    )
    .unwrap();

    let object = mesh_from_obj_with_material_dir(
        b"mtllib jewel.mtl\n\
          usemtl polished_gold\n\
          v 0 0 0\n\
          v 1 0 0\n\
          v 1 1 0\n\
          v 0 1 0\n\
          f -4 -3 -2 -1\n",
        &material_dir,
    )
    .unwrap();

    assert_eq!(object.faces, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(object.diffuse_color, Some([51, 102, 153, 255]));
    assert_eq!(object.texture_files, vec!["albedo.png"]);
    assert_eq!(object.texture_per_face, vec![0, 0]);
    assert_eq!(object.material_names, vec!["polished_gold"]);

    let _ = std::fs::remove_dir_all(material_dir);
}

#[test]
fn mesh_obj_import_preserves_meshlib_vt_uvs_for_textured_faces() {
    let material_dir = std::env::temp_dir().join(format!(
        "zennah_obj_mtl_{}_{}",
        std::process::id(),
        "vt_uvs"
    ));
    std::fs::create_dir_all(&material_dir).unwrap();
    std::fs::write(
        material_dir.join("jewel.mtl"),
        "newmtl polished_gold\nmap_Kd albedo.png\n",
    )
    .unwrap();

    let object = mesh_from_obj_with_material_dir(
        b"mtllib jewel.mtl\n\
          usemtl polished_gold\n\
          v 0 0 0\n\
          v 1 0 0\n\
          v 1 1 0\n\
          v 0 1 0\n\
          vt 0.0 0.0\n\
          vt 1.0 0.0\n\
          vt 1.0 1.0\n\
          vt 0.0 1.0\n\
          f 1/1 2/2 3/3 4/4\n",
        &material_dir,
    )
    .unwrap();

    assert_eq!(object.faces, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(object.texture_files, vec!["albedo.png"]);
    assert_eq!(object.texture_per_face, vec![0, 0]);
    assert_eq!(
        object.tri_corner_uvs,
        vec![
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        ]
    );

    let _ = std::fs::remove_dir_all(material_dir);
}

#[test]
fn mesh_obj_import_loads_meshlib_map_kd_texture_image() {
    let material_dir = std::env::temp_dir().join(format!(
        "zennah_obj_mtl_{}_{}",
        std::process::id(),
        "texture_image"
    ));
    std::fs::create_dir_all(&material_dir).unwrap();
    let texture_path = material_dir.join("albedo.png");
    std::fs::write(&texture_path, opaque_white_png()).unwrap();
    std::fs::write(
        material_dir.join("jewel.mtl"),
        "newmtl polished_gold\nKd 0.2 0.4 0.6\nmap_Kd -clamp on albedo.png\n",
    )
    .unwrap();

    let object = mesh_from_obj_with_material_dir(
        b"mtllib jewel.mtl\n\
          usemtl polished_gold\n\
          v 0 0 0\n\
          v 1 0 0\n\
          v 1 1 0\n\
          v 0 1 0\n\
          f -4 -3 -2 -1\n",
        &material_dir,
    )
    .unwrap();

    assert_eq!(object.texture_files, vec!["albedo.png"]);
    assert_eq!(object.texture_per_face, vec![0, 0]);
    assert_eq!(object.texture_images.len(), 1);
    let texture = &object.texture_images[0];
    assert_eq!(texture.file, "albedo.png");
    assert_eq!(texture.resolved_path, texture_path.to_string_lossy());
    assert_eq!(texture.width, 1);
    assert_eq!(texture.height, 1);
    assert_eq!(texture.filter, "Linear");
    assert_eq!(texture.wrap, "Clamp");
    assert_eq!(texture.pixels_rgba, vec![[255, 255, 255, 255]]);

    let _ = std::fs::remove_dir_all(material_dir);
}

#[test]
fn point_cloud_ply_io_preserves_points_normals_and_colors_in_rust() {
    let source = b"ply\n\
format ascii 1.0\n\
comment MeshInspector.com\n\
element vertex 2\n\
property float x\n\
property float y\n\
property float z\n\
property float nx\n\
property float ny\n\
property float nz\n\
property uchar red\n\
property uchar green\n\
property uchar blue\n\
end_header\n\
0.0 1.0 2.0 0.0 0.0 1.0 255 128 0\n\
3.0 4.0 5.0 0.0 1.0 0.0 4 5 6\n";

    let document = point_cloud_from_ply(source).unwrap();

    assert_eq!(document.points, vec![[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]]);
    assert_eq!(document.normals, vec![[0.0, 0.0, 1.0], [0.0, 1.0, 0.0]]);
    assert_eq!(document.colors, vec![[255, 128, 0], [4, 5, 6]]);

    let exported = point_cloud_to_ply(
        &document.points,
        Some(document.normals.as_slice()),
        Some(document.colors.as_slice()),
    )
    .unwrap();
    let header_end = exported
        .windows(b"end_header\n".len())
        .position(|window| window == b"end_header\n")
        .unwrap()
        + b"end_header\n".len();
    assert!(std::str::from_utf8(&exported[..header_end])
        .unwrap()
        .contains("format binary_little_endian 1.0"));

    let round_tripped = point_cloud_from_ply(&exported).unwrap();
    assert_eq!(round_tripped.points, document.points);
    assert_eq!(round_tripped.normals, document.normals);
    assert_eq!(round_tripped.colors, document.colors);
}
