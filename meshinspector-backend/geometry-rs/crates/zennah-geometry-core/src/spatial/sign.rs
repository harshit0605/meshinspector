use crate::mesh::{edge_face_map, validate_faces};
use crate::{GeometryError, RayHit};

const DEFAULT_WINDING_SIGN_SELF_INTERSECTION_FACE_LIMIT: usize = 50_000;
const DEFAULT_RAY_SIGN_DIRECTION: [f64; 3] = [1.0, 0.371, 0.219];

pub fn supports_winding_sign_for_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> Result<bool, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(false);
    }
    if !edge_face_map(&faces)
        .values()
        .all(|face_ids| face_ids.len() == 2)
    {
        return Ok(false);
    }

    let should_reject = reject_self_intersections
        && max_self_intersection_faces.is_none_or(|limit| faces.len() <= limit);
    if should_reject && !super::self_intersecting_faces(vertices, faces_i64, epsilon)?.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub fn point_inside_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    point: [f64; 3],
    direction: [f64; 3],
    epsilon: f64,
) -> Result<bool, GeometryError> {
    let hits = super::ray_triangle_hits(vertices, faces_i64, point, direction, epsilon, &[])?;
    let unique_distances = unique_hit_distances(&hits, epsilon * 100.0);
    Ok(unique_distances.len() % 2 == 1)
}

pub fn point_inside_mesh_winding(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    point: [f64; 3],
    threshold: f64,
    require_closed: bool,
    epsilon: f64,
) -> Result<bool, GeometryError> {
    if require_closed
        && !supports_winding_sign_for_mesh(
            vertices,
            faces_i64,
            true,
            Some(DEFAULT_WINDING_SIGN_SELF_INTERSECTION_FACE_LIMIT),
            epsilon,
        )?
    {
        return Ok(false);
    }
    let winding = super::winding_numbers(&[point], vertices, faces_i64)?
        .first()
        .copied()
        .unwrap_or(0.0);
    Ok(winding.abs() >= threshold)
}

pub fn signed_point_mesh_distances_with_method(
    points: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    sign_method: &str,
    winding_threshold: f64,
    topology_epsilon: f64,
    ray_epsilon: f64,
) -> Result<Vec<f64>, GeometryError> {
    let method = if sign_method == "auto" {
        if supports_winding_sign_for_mesh(
            vertices,
            faces_i64,
            true,
            Some(DEFAULT_WINDING_SIGN_SELF_INTERSECTION_FACE_LIMIT),
            topology_epsilon,
        )? {
            "winding"
        } else {
            "unsigned"
        }
    } else {
        sign_method
    };

    match method {
        "winding" => {
            super::signed_point_mesh_distances(points, vertices, faces_i64, winding_threshold)
        }
        "unsigned" => super::point_mesh_distances(points, vertices, faces_i64),
        "ray" => signed_distances_by_ray(points, vertices, faces_i64, ray_epsilon),
        _ => Err(GeometryError::InvalidSignMethod {
            method: sign_method.to_string(),
        }),
    }
}

fn signed_distances_by_ray(
    points: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    ray_epsilon: f64,
) -> Result<Vec<f64>, GeometryError> {
    let distances = super::point_mesh_distances(points, vertices, faces_i64)?;
    let mut signed = Vec::with_capacity(points.len());
    for (point, distance) in points.iter().zip(distances) {
        let sign = if point_inside_mesh(
            vertices,
            faces_i64,
            *point,
            DEFAULT_RAY_SIGN_DIRECTION,
            ray_epsilon,
        )? {
            -1.0
        } else {
            1.0
        };
        signed.push(distance * sign);
    }
    Ok(signed)
}

fn unique_hit_distances(hits: &[RayHit], epsilon: f64) -> Vec<f64> {
    let mut unique = Vec::new();
    for hit in hits.iter().filter(|hit| hit.distance > epsilon) {
        if unique
            .last()
            .map(|last| f64::abs(hit.distance - last) > epsilon)
            .unwrap_or(true)
        {
            unique.push(hit.distance);
        }
    }
    unique
}
