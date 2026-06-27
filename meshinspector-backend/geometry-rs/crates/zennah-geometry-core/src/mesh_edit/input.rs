use super::SubdivideMeshOptions;
use crate::GeometryError;

pub(super) fn validate_subdivide_options(
    options: &SubdivideMeshOptions,
) -> Result<(), GeometryError> {
    if !options.max_edge_len.is_finite() || options.max_edge_len < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_edge_len",
            value: options.max_edge_len,
        });
    }
    if !options.curvature_priority.is_finite() || options.curvature_priority < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "curvature_priority",
            value: options.curvature_priority,
        });
    }
    if !options.min_sharp_dihedral_angle.is_finite() || options.min_sharp_dihedral_angle <= 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "min_sharp_dihedral_angle",
            value: options.min_sharp_dihedral_angle,
        });
    }
    if !options.max_tri_aspect_ratio.is_finite() || options.max_tri_aspect_ratio < 0.0 {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_tri_aspect_ratio",
            value: options.max_tri_aspect_ratio,
        });
    }
    if !options.max_splittable_tri_aspect_ratio.is_finite()
        || options.max_splittable_tri_aspect_ratio < 0.0
    {
        return Err(GeometryError::InvalidMeshEditInput {
            field: "max_splittable_tri_aspect_ratio",
            value: options.max_splittable_tri_aspect_ratio,
        });
    }
    validate_optional_nonnegative_finite(
        "max_deviation_after_flip",
        options.max_deviation_after_flip,
    )?;
    validate_optional_nonnegative_finite(
        "max_angle_change_after_flip",
        options.max_angle_change_after_flip,
    )?;
    validate_optional_nonnegative_finite(
        "critical_tri_aspect_ratio_flip",
        options.critical_tri_aspect_ratio_flip,
    )?;
    Ok(())
}

fn validate_optional_nonnegative_finite(
    field: &'static str,
    value: Option<f64>,
) -> Result<(), GeometryError> {
    if let Some(value) = value {
        if !value.is_finite() || value < 0.0 {
            return Err(GeometryError::InvalidMeshEditInput { field, value });
        }
    }
    Ok(())
}

pub(super) fn validate_faces(
    faces: &[[i64; 3]],
    vertex_count: usize,
) -> Result<Vec<[usize; 3]>, GeometryError> {
    faces
        .iter()
        .enumerate()
        .map(|(face_index, face)| {
            let mut converted = [0_usize; 3];
            for (slot, vertex) in face.iter().copied().enumerate() {
                if vertex < 0 {
                    return Err(GeometryError::NegativeFaceIndex {
                        face: face_index,
                        vertex,
                    });
                }
                let vertex_index = vertex as usize;
                if vertex_index >= vertex_count {
                    return Err(GeometryError::FaceIndexOutOfBounds {
                        face: face_index,
                        vertex,
                        vertex_count,
                    });
                }
                converted[slot] = vertex_index;
            }
            Ok(converted)
        })
        .collect()
}

pub(super) fn initial_region(
    face_count: usize,
    region_faces: Option<&[usize]>,
) -> Result<Vec<bool>, GeometryError> {
    let mut region = vec![region_faces.is_none(); face_count];
    if let Some(indices) = region_faces {
        region.fill(false);
        for index in indices {
            if *index >= face_count {
                return Err(GeometryError::FaceRegionIndexOutOfBounds {
                    index: *index,
                    face_count,
                });
            }
            region[*index] = true;
        }
    }
    Ok(region)
}
