use super::export_validation::*;
use super::export_write::*;
use super::*;

pub fn meshlib_object_mesh_mru_scene_bytes(
    input: &MeshlibObjectMeshSceneInput,
    model_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let root_key = meshlib_scene_key("Root", 0);
    let object_key = meshlib_scene_key(&input.object_name, input.child_index);
    let extension = normalized_extension(&input.model_extension);
    let model_path = format!("{root_key}/{object_key}{extension}");
    let root_payload = meshlib_object_mesh_mru_scene_value(input);
    let root_json = serde_json::to_vec(&root_payload).map_err(|error| error.to_string())?;

    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("Root.json", options)
        .map_err(|error| error.to_string())?;
    zip.write_all(&root_json)
        .map_err(|error| error.to_string())?;
    zip.start_file(model_path, options)
        .map_err(|error| error.to_string())?;
    zip.write_all(model_bytes)
        .map_err(|error| error.to_string())?;
    let cursor = zip.finish().map_err(|error| error.to_string())?;
    Ok(cursor.into_inner())
}

pub(super) fn meshlib_multi_object_mru_scene_value(
    input: &MeshlibSceneExportInput,
    root_key: &str,
) -> Result<Value, String> {
    Ok(json!({
        "FormatVersion": 1.0,
        "Name": if input.root_name.is_empty() { "Root" } else { input.root_name.as_str() },
        "Visibility": VIEWPORT_MASK_ALL,
        "Selected": false,
        "Locked": false,
        "ParentLocked": false,
        "XF": meshlib_scene_xf_value(MeshlibSceneXf::identity()),
        "Type": ["Object", "RootObject"],
        "Tags": [],
        "Key": root_key,
        "Children": {},
        "meshlib_reference": "MR::serializeObjectTree",
        "meshlib_source": "MeshLib/source/MRMesh/MRObjectSave.cpp;MeshLib/source/MRMesh/MRObject.cpp",
        "meshlib_source_language": "rust",
    }))
}

pub(super) fn meshlib_export_object_scene_value(object: &MeshlibSceneExportObject) -> Value {
    let mut value = json!({
        "meshlib_reference": "MR::serializeObjectTree/ObjectMeshHolder::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectMeshHolder.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
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
        "ShowTexture": 0,
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
        "TextureCount": 0,
        "Textures": {},
        "TexturePerFace": {},
        "UVCoordinates": {},
        "SelectionFaceBitSet": {},
        "SelectionEdgeBitSet": {},
        "MeshCreasesUndirEdgeBitSet": {},
        "PointSize": 5.0,
    });
    if let Some(link) = object.link.as_ref() {
        value["Link"] = json!(link);
    }
    value
}

pub(super) fn meshlib_export_group_object_scene_value(object: &MeshlibSceneGroupObject) -> Value {
    json!({
        "meshlib_reference": "MR::serializeObjectTree/Object::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectSave.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
        "Type": ["Object"],
        "Tags": [],
    })
}

pub(super) fn meshlib_export_line_object_scene_value(object: &MeshlibSceneObjectLines) -> Value {
    json!({
        "meshlib_reference": "MR::serializeObjectTree/ObjectLinesHolder::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectLinesHolder.cpp;MeshLib/source/MRMesh/MRObjectLines.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
        "Type": ["Object", "VisualObject", "LinesHolder", "ObjectLines"],
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
        "ShowPoints": object.show_points,
        "SmoothConnections": object.smooth_connections,
        "ColoringType": object.coloring_type,
        "LineColors": meshlib_rgba_rows_value(&object.line_colors),
        "VertColors": meshlib_rgba_rows_value(&object.vert_colors),
        "LineWidth": object.line_width,
        "Polyline": {
            "Points": object.points.iter().map(|point| meshlib_vec3_value(*point)).collect::<Vec<_>>(),
            "Lines": object.lines.iter().flat_map(|line| [json!(line[0]), json!(line[1])]).collect::<Vec<_>>(),
        },
    })
}

pub(super) fn meshlib_export_point_object_scene_value(object: &MeshlibSceneObjectPoints) -> Value {
    let mut value = json!({
        "meshlib_reference": "MR::serializeObjectTree/ObjectPointsHolder::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectPointsHolder.cpp;MeshLib/source/MRMesh/MRObjectPoints.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
        "Type": ["Object", "VisualObject", "PointsHolder", "ObjectPoints"],
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
            "Selection": {
                "Diffuse": {"x": 1.0, "y": 0.7, "z": 0.0, "w": 1.0},
                "Points": {"x": 1.0, "y": 0.7, "z": 0.0, "w": 1.0},
            },
        },
        "ShowName": 0,
        "UseDefaultSceneProperties": false,
        "ShowPoints": VIEWPORT_MASK_ALL,
        "ColoringType": "Solid",
        "SelectionVertBitSet": {},
        "ValidVertBitSet": {},
        "PointSize": object.point_size,
        "MaxRenderingPoints": object.max_rendering_points,
    });
    if let Some(link) = object.link.as_ref() {
        value["Link"] = json!(link);
    }
    value
}

pub(super) fn meshlib_export_distance_map_object_scene_value(
    object: &MeshlibSceneObjectDistanceMap,
) -> Value {
    let mut value = json!({
        "meshlib_reference": "MR::serializeObjectTree/ObjectDistanceMap::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRObjectDistanceMap.cpp;MeshLib/source/MRMesh/MRDistanceMapSave.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
        "Type": ["Object", "VisualObject", "ObjectDistanceMap"],
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
        "PixelXVec": meshlib_vec3_value(object.pixel_x_vec),
        "PixelYVec": meshlib_vec3_value(object.pixel_y_vec),
        "DepthVec": meshlib_vec3_value(object.depth_vec),
        "OriginWorld": meshlib_vec3_value(object.origin_world),
    });
    if let Some(link) = object.link.as_ref() {
        value["Link"] = json!(link);
    }
    value
}

pub(super) fn meshlib_export_voxel_object_scene_value(object: &MeshlibSceneObjectVoxels) -> Value {
    let selection_size = meshlib_scene_voxel_value_count(object).unwrap_or(object.values.len());
    let mut value = json!({
        "meshlib_reference": "MR::serializeObjectTree/ObjectVoxels::serializeFields_",
        "meshlib_source": "MeshLib/source/MRVoxels/MRObjectVoxels.cpp;MeshLib/source/MRVoxels/MRVoxelsSave.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
        "Type": ["Object", "VisualObject", "ObjectVoxels"],
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
        "VoxelSize": meshlib_vec3f32_value(object.voxel_size),
        "Dimensions": meshlib_vec3usize_value(object.dimensions),
        "MinCorner": meshlib_vec3usize_value(object.min_corner),
        "MaxCorner": meshlib_vec3usize_value(object.max_corner),
        "SelectionVoxels": meshlib_compact_bitset_value(&object.selected_voxels, selection_size),
        "IsoValue": object.iso_value,
        "DualMarchingCubes": object.dual_marching_cubes,
    });
    if let Some(link) = object.link.as_ref() {
        value["Link"] = json!(link);
    }
    value
}

pub(super) fn meshlib_export_feature_object_scene_value(
    object: &MeshlibSceneFeatureObject,
) -> Value {
    let mut dimension_visibility = Map::new();
    for (name, mask) in &object.dimension_visibility {
        dimension_visibility.insert(name.clone(), json!(mask));
    }

    json!({
        "meshlib_reference": "MR::serializeObjectTree/FeatureObject::serializeFields_",
        "meshlib_source": "MeshLib/source/MRMesh/MRObject.cpp;MeshLib/source/MRMesh/MRFeatureObject.cpp;MeshLib/source/MRMesh/MRPointObject.cpp;MeshLib/source/MRMesh/MRLineObject.cpp;MeshLib/source/MRMesh/MRPlaneObject.cpp;MeshLib/source/MRMesh/MRSphereObject.cpp;MeshLib/source/MRMesh/MRCircleObject.cpp;MeshLib/source/MRMesh/MRCylinderObject.cpp;MeshLib/source/MRMesh/MRConeObject.cpp",
        "meshlib_source_language": "rust",
        "Key": object.object_key,
        "Name": object.object_name,
        "Visibility": object.visibility_mask,
        "Selected": object.selected,
        "Locked": object.locked,
        "ParentLocked": object.parent_locked,
        "XF": meshlib_scene_xf_value(object.xf),
        "Type": ["Object", "VisualObject", "FeatureObject", object.feature_type.as_str()],
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
        "SubfeatureVisibility": object.subfeature_visibility,
        "DetailsOnNameTag": object.details_on_name_tag,
        "DecorationsColorUnselected": meshlib_vec4_value(object.decorations_color_unselected),
        "DecorationsColorSelected": meshlib_vec4_value(object.decorations_color_selected),
        "PointSize": object.point_size,
        "LineWidth": object.line_width,
        "SubPointSize": object.sub_point_size,
        "SubLineWidth": object.sub_line_width,
        "MainAlpha": object.main_alpha,
        "SubAlphaPoints": object.sub_alpha_points,
        "SubAlphaLines": object.sub_alpha_lines,
        "SubAlphaMesh": object.sub_alpha_mesh,
        "DimensionVisibility": Value::Object(dimension_visibility),
    })
}
