use super::bvh::build_flat_bvh;
use super::ray::ray_hits_with_bvh;
use crate::math::normalize_vector;
use crate::mesh::validate_faces;
use crate::GeometryError;

pub(crate) fn ray_triangle_hit_counts(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origins: &[[f64; 3]],
    directions: &[[f64; 3]],
    epsilon: f64,
    ignored_face_indices: &[i64],
) -> Result<Vec<usize>, GeometryError> {
    if origins.len() != directions.len() || origins.len() != ignored_face_indices.len() {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "ray_queries",
            value: format!(
                "origins={}, directions={}, ignored_faces={}",
                origins.len(),
                directions.len(),
                ignored_face_indices.len()
            ),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(vec![0; origins.len()]);
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let mut counts = Vec::with_capacity(origins.len());
    for ((origin, direction), ignored_face) in origins
        .iter()
        .copied()
        .zip(directions.iter().copied())
        .zip(ignored_face_indices.iter().copied())
    {
        let ray_direction = normalize_vector(direction)?;
        counts.push(
            ray_hits_with_bvh(
                origin,
                ray_direction,
                &bvh,
                &triangles,
                epsilon,
                &[ignored_face],
            )
            .len(),
        );
    }
    Ok(counts)
}
