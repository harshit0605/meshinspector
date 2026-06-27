use super::base::validate_faces;
use crate::math::{add, cross, distance_sq, dot, norm, scale, sub};
use crate::GeometryError;

pub fn select_overlapping_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    max_dist_sq: f64,
    max_normal_dot: f64,
    min_area_fraction: f64,
) -> Result<Vec<i64>, GeometryError> {
    if !max_dist_sq.is_finite() || max_dist_sq < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_dist_sq",
            value: max_dist_sq.to_string(),
        });
    }
    if !max_normal_dot.is_finite() || !(-1.0..=1.0).contains(&max_normal_dot) {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "max_normal_dot",
            value: max_normal_dot.to_string(),
        });
    }
    if !min_area_fraction.is_finite() || min_area_fraction < 0.0 {
        return Err(GeometryError::InvalidSelectionParameter {
            field: "min_area_fraction",
            value: min_area_fraction.to_string(),
        });
    }

    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.len() < 2 {
        return Ok(Vec::new());
    }

    let triangles = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect::<Vec<_>>();
    let double_area_vectors = triangles
        .iter()
        .map(|triangle| cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0])))
        .collect::<Vec<_>>();
    let double_areas = double_area_vectors
        .iter()
        .map(|vector| norm(*vector))
        .collect::<Vec<_>>();
    let normals = double_area_vectors
        .iter()
        .zip(&double_areas)
        .map(|(vector, area)| {
            if *area <= 1e-12 {
                [0.0; 3]
            } else {
                scale(*vector, 1.0 / area)
            }
        })
        .collect::<Vec<_>>();

    let candidates =
        crate::spatial::aabb_overlapping_face_pairs(vertices, faces_i64, 16, max_dist_sq.sqrt())?;
    let mut selected = vec![false; faces.len()];
    for (left, right) in candidates {
        if triangle_triangle_distance_sq(triangles[left], triangles[right]) > max_dist_sq {
            continue;
        }
        if overlapping_neighbor_matches(
            left,
            right,
            &double_areas,
            &normals,
            max_normal_dot,
            min_area_fraction,
        ) {
            selected[left] = true;
        }
        if overlapping_neighbor_matches(
            right,
            left,
            &double_areas,
            &normals,
            max_normal_dot,
            min_area_fraction,
        ) {
            selected[right] = true;
        }
    }

    Ok(selected
        .into_iter()
        .enumerate()
        .filter_map(|(face_index, is_selected)| is_selected.then_some(face_index as i64))
        .collect())
}

pub fn select_inside_part_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<i64>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(Vec::new());
    }

    let triangles = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect::<Vec<_>>();
    let normalization = 4.0 * std::f64::consts::PI;
    let mut selected = Vec::new();
    for (face_index, triangle) in triangles.iter().enumerate() {
        let center = face_center(triangle);
        let winding = triangles
            .iter()
            .enumerate()
            .filter(|(candidate_index, _)| *candidate_index != face_index)
            .map(|(_, candidate)| triangle_solid_angle(center, *candidate))
            .sum::<f64>()
            / normalization;
        if winding < 0.0 || winding > 1.0 {
            selected.push(face_index as i64);
        }
    }
    Ok(selected)
}

pub(super) fn face_center(triangle: &[[f64; 3]; 3]) -> [f64; 3] {
    [
        (triangle[0][0] + triangle[1][0] + triangle[2][0]) / 3.0,
        (triangle[0][1] + triangle[1][1] + triangle[2][1]) / 3.0,
        (triangle[0][2] + triangle[1][2] + triangle[2][2]) / 3.0,
    ]
}

fn triangle_solid_angle(point: [f64; 3], triangle: [[f64; 3]; 3]) -> f64 {
    let a = sub(triangle[0], point);
    let b = sub(triangle[1], point);
    let c = sub(triangle[2], point);
    let la = norm(a);
    let lb = norm(b);
    let lc = norm(c);
    let numerator = dot(a, cross(b, c));
    let denominator = la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb;
    2.0 * numerator.atan2(denominator)
}

pub(super) fn ray_intersects_any_face_except(
    triangles: &[[[f64; 3]; 3]],
    origin: [f64; 3],
    direction: [f64; 3],
    ignored_face: usize,
    epsilon: f64,
) -> bool {
    triangles.iter().enumerate().any(|(face_index, triangle)| {
        face_index != ignored_face
            && ray_triangle_distance(origin, direction, *triangle, epsilon).is_some()
    })
}

fn ray_triangle_distance(
    origin: [f64; 3],
    direction: [f64; 3],
    triangle: [[f64; 3]; 3],
    epsilon: f64,
) -> Option<f64> {
    let [a, b, c] = triangle;
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let h = cross(direction, edge2);
    let det = dot(edge1, h);
    if det.abs() <= epsilon {
        return None;
    }

    let inv_det = 1.0 / det;
    let s = sub(origin, a);
    let u = inv_det * dot(s, h);
    if u < -epsilon || u > 1.0 + epsilon {
        return None;
    }

    let q = cross(s, edge1);
    let v = inv_det * dot(direction, q);
    if v < -epsilon || u + v > 1.0 + epsilon {
        return None;
    }

    let distance = inv_det * dot(edge2, q);
    if distance <= epsilon {
        return None;
    }
    Some(distance)
}

fn overlapping_neighbor_matches(
    face_index: usize,
    neighbor_index: usize,
    double_areas: &[f64],
    normals: &[[f64; 3]],
    max_normal_dot: f64,
    min_area_fraction: f64,
) -> bool {
    if double_areas[face_index] <= 1e-12 || double_areas[neighbor_index] <= 1e-12 {
        return false;
    }
    if double_areas[face_index] * min_area_fraction > double_areas[neighbor_index] {
        return false;
    }
    dot(normals[face_index], normals[neighbor_index]) <= max_normal_dot
}

fn triangle_triangle_distance_sq(first: [[f64; 3]; 3], second: [[f64; 3]; 3]) -> f64 {
    if crate::spatial::triangles_intersect(first, second, 1e-12) {
        return 0.0;
    }

    let mut best = f64::INFINITY;
    for point in first {
        let closest = crate::spatial::closest_point_on_triangle(point, second);
        best = best.min(distance_sq(point, closest));
    }
    for point in second {
        let closest = crate::spatial::closest_point_on_triangle(point, first);
        best = best.min(distance_sq(point, closest));
    }
    for left_edge in 0..3 {
        for right_edge in 0..3 {
            best = best.min(segment_segment_distance_sq(
                first[left_edge],
                first[(left_edge + 1) % 3],
                second[right_edge],
                second[(right_edge + 1) % 3],
            ));
        }
    }
    best
}

fn segment_segment_distance_sq(
    first_start: [f64; 3],
    first_end: [f64; 3],
    second_start: [f64; 3],
    second_end: [f64; 3],
) -> f64 {
    let first_direction = sub(first_end, first_start);
    let second_direction = sub(second_end, second_start);
    let start_delta = sub(first_start, second_start);
    let first_len_sq = dot(first_direction, first_direction);
    let second_len_sq = dot(second_direction, second_direction);
    let second_projection = dot(second_direction, start_delta);
    let epsilon = 1e-18;

    let (first_t, second_t) = if first_len_sq <= epsilon && second_len_sq <= epsilon {
        (0.0, 0.0)
    } else if first_len_sq <= epsilon {
        (0.0, clamp01(second_projection / second_len_sq))
    } else {
        let first_projection = dot(first_direction, start_delta);
        if second_len_sq <= epsilon {
            (clamp01(-first_projection / first_len_sq), 0.0)
        } else {
            let direction_dot = dot(first_direction, second_direction);
            let denominator = first_len_sq * second_len_sq - direction_dot * direction_dot;
            let mut first_t = if denominator.abs() > epsilon {
                clamp01(
                    (direction_dot * second_projection - first_projection * second_len_sq)
                        / denominator,
                )
            } else {
                0.0
            };
            let mut second_t = (direction_dot * first_t + second_projection) / second_len_sq;
            if second_t < 0.0 {
                second_t = 0.0;
                first_t = clamp01(-first_projection / first_len_sq);
            } else if second_t > 1.0 {
                second_t = 1.0;
                first_t = clamp01((direction_dot - first_projection) / first_len_sq);
            }
            (first_t, second_t)
        }
    };

    let first_closest = add(first_start, scale(first_direction, first_t));
    let second_closest = add(second_start, scale(second_direction, second_t));
    distance_sq(first_closest, second_closest)
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}
