use super::decode::*;
use super::*;

pub(super) fn meshlib_validate_scene_line_object(
    object: &MeshlibSceneObjectLines,
) -> Result<(), String> {
    for (index, line) in object.lines.iter().enumerate() {
        if line[0] >= object.points.len() || line[1] >= object.points.len() {
            return Err(format!(
                "MRU ObjectLines {} line {} references a missing point",
                object.object_key, index
            ));
        }
        if line[0] == line[1] {
            return Err(format!(
                "MRU ObjectLines {} line {} references the same point twice",
                object.object_key, index
            ));
        }
    }
    if object.line_width <= 0.0 {
        return Err(format!(
            "MRU ObjectLines {} LineWidth must be positive",
            object.object_key
        ));
    }
    Ok(())
}

pub(super) fn meshlib_validate_scene_point_object(
    object: &MeshlibSceneObjectPoints,
) -> Result<(), String> {
    if object
        .points
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(format!(
            "MRU ObjectPoints {} contains non-finite point coordinates",
            object.object_key
        ));
    }
    if !object.normals.is_empty() && object.normals.len() != object.points.len() {
        return Err(format!(
            "MRU ObjectPoints {} normals must match point count",
            object.object_key
        ));
    }
    if object
        .normals
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(format!(
            "MRU ObjectPoints {} contains non-finite normals",
            object.object_key
        ));
    }
    if !object.vert_colors.is_empty() && object.vert_colors.len() != object.points.len() {
        return Err(format!(
            "MRU ObjectPoints {} vertex colors must match point count",
            object.object_key
        ));
    }
    if !object.point_size.is_finite() || object.point_size <= 0.0 {
        return Err(format!(
            "MRU ObjectPoints {} PointSize must be positive",
            object.object_key
        ));
    }
    Ok(())
}

pub(super) fn meshlib_validate_scene_distance_map_object(
    object: &MeshlibSceneObjectDistanceMap,
) -> Result<(), String> {
    if object.width == 0 || object.height == 0 {
        return Err(format!(
            "MRU ObjectDistanceMap {} dimensions must be positive",
            object.object_key
        ));
    }
    let expected_values = object.width.checked_mul(object.height).ok_or_else(|| {
        format!(
            "MRU ObjectDistanceMap {} dimensions overflow",
            object.object_key
        )
    })?;
    if object.values.len() != expected_values {
        return Err(format!(
            "MRU ObjectDistanceMap {} values must match width * height",
            object.object_key
        ));
    }
    for (index, value) in object.values.iter().enumerate() {
        if !value.is_finite() {
            return Err(format!(
                "MRU ObjectDistanceMap {} value {} is not finite",
                object.object_key, index
            ));
        }
    }
    for (field_name, vector) in [
        ("OriginWorld", object.origin_world),
        ("PixelXVec", object.pixel_x_vec),
        ("PixelYVec", object.pixel_y_vec),
        ("DepthVec", object.depth_vec),
    ] {
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "MRU ObjectDistanceMap {} {field_name} contains a non-finite coordinate",
                object.object_key
            ));
        }
    }
    Ok(())
}

pub(super) fn meshlib_validate_scene_voxel_object(
    object: &MeshlibSceneObjectVoxels,
) -> Result<(), String> {
    if object.dimensions.iter().any(|dimension| *dimension == 0) {
        return Err(format!(
            "MRU ObjectVoxels {} dimensions must be positive",
            object.object_key
        ));
    }
    let expected_values = meshlib_scene_voxel_value_count(object)?;
    let opaque_vdb = meshlib_scene_voxel_has_opaque_vdb_payload(object);
    if opaque_vdb && object.model_bytes.is_empty() {
        return Err(format!(
            "MRU ObjectVoxels {} .vdb payload is empty",
            object.object_key
        ));
    }
    if object.values.len() != expected_values && !(opaque_vdb && object.values.is_empty()) {
        return Err(format!(
            "MRU ObjectVoxels {} values must match dimensions",
            object.object_key
        ));
    }
    for (index, value) in object.values.iter().enumerate() {
        if !value.is_finite() {
            return Err(format!(
                "MRU ObjectVoxels {} value {} is not finite",
                object.object_key, index
            ));
        }
    }
    if object
        .voxel_size
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(format!(
            "MRU ObjectVoxels {} voxel size must be positive",
            object.object_key
        ));
    }
    if object
        .min_corner
        .iter()
        .zip(object.max_corner.iter())
        .any(|(min_corner, max_corner)| min_corner > max_corner)
        || object
            .max_corner
            .iter()
            .zip(object.dimensions.iter())
            .any(|(max_corner, dimension)| max_corner > dimension)
    {
        return Err(format!(
            "MRU ObjectVoxels {} corners must fit dimensions",
            object.object_key
        ));
    }
    if !object.iso_value.is_finite() {
        return Err(format!(
            "MRU ObjectVoxels {} IsoValue must be finite",
            object.object_key
        ));
    }
    if object
        .selected_voxels
        .iter()
        .any(|voxel_id| *voxel_id >= expected_values)
    {
        return Err(format!(
            "MRU ObjectVoxels {} selected voxels reference missing values",
            object.object_key
        ));
    }
    Ok(())
}

pub(super) fn meshlib_scene_voxel_value_count(
    object: &MeshlibSceneObjectVoxels,
) -> Result<usize, String> {
    object
        .dimensions
        .iter()
        .try_fold(1usize, |product, dimension| product.checked_mul(*dimension))
        .ok_or_else(|| format!("MRU ObjectVoxels {} dimensions overflow", object.object_key))
}

pub(super) fn meshlib_scene_voxel_has_opaque_vdb_payload(
    object: &MeshlibSceneObjectVoxels,
) -> bool {
    object
        .model_extension
        .trim_start_matches('.')
        .eq_ignore_ascii_case("vdb")
}

pub(super) fn meshlib_validate_scene_feature_object(
    object: &MeshlibSceneFeatureObject,
) -> Result<(), String> {
    if !meshlib_is_supported_feature_type(&object.feature_type) {
        return Err(format!(
            "MRU FeatureObject {} has unsupported feature type {}",
            object.object_key, object.feature_type
        ));
    }
    for (field_name, vector) in [
        (
            "DecorationsColorUnselected",
            object.decorations_color_unselected,
        ),
        (
            "DecorationsColorSelected",
            object.decorations_color_selected,
        ),
    ] {
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "MRU FeatureObject {} {field_name} contains a non-finite coordinate",
                object.object_key
            ));
        }
    }
    for (field_name, value) in [
        ("PointSize", object.point_size),
        ("LineWidth", object.line_width),
        ("SubPointSize", object.sub_point_size),
        ("SubLineWidth", object.sub_line_width),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!(
                "MRU FeatureObject {} {field_name} must be positive",
                object.object_key
            ));
        }
    }
    for (field_name, value) in [
        ("MainAlpha", object.main_alpha),
        ("SubAlphaPoints", object.sub_alpha_points),
        ("SubAlphaLines", object.sub_alpha_lines),
        ("SubAlphaMesh", object.sub_alpha_mesh),
    ] {
        if !value.is_finite() {
            return Err(format!(
                "MRU FeatureObject {} {field_name} must be finite",
                object.object_key
            ));
        }
    }
    Ok(())
}

pub(super) fn meshlib_distance_map_stats(values: &[f32]) -> (usize, f32, f32) {
    let mut valid_count = 0usize;
    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;
    for value in values {
        if *value == crate::distance::DISTANCE_MAP_NOT_VALID_VALUE {
            continue;
        }
        valid_count += 1;
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    if valid_count == 0 {
        (0, 0.0, 0.0)
    } else {
        (valid_count, min_value, max_value)
    }
}

pub(super) fn meshlib_voxel_stats(values: &[f32]) -> (f32, f32) {
    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;
    for value in values {
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    if values.is_empty() {
        (0.0, 0.0)
    } else {
        (min_value, max_value)
    }
}
