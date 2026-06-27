use crate::math::{add, dot, norm, scale, sub};
use crate::mesh::{
    edge_face_map, mesh_bounds, mesh_stats, validate_faces, vertex_face_adjacency,
    vertex_neighbor_list, vertex_normals,
};
use crate::spatial::{
    build_flat_bvh, closest_point_excluding_incident, point_mesh_distances,
    ray_thickness_at_vertices, self_intersecting_faces, signed_point_mesh_distances, FlatBvh,
};
use crate::{DistanceSummary, GeometryError, ThicknessSummary, VersionCompareSummary};
use rayon::prelude::*;

mod section;
pub use section::{section_contour, SectionContour, SectionContourSegment};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignedCompareOptions {
    pub winding_threshold: f64,
    pub reject_self_intersections: bool,
    pub max_self_intersection_faces: Option<usize>,
    pub epsilon: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InSphereThicknessOptions {
    pub max_radius: f64,
    pub max_iters: usize,
    pub min_shrinkage: f64,
    pub min_angle_cos: f64,
    pub epsilon: f64,
}

impl Default for InSphereThicknessOptions {
    fn default() -> Self {
        Self {
            max_radius: 1.0,
            max_iters: 16,
            min_shrinkage: 0.99999,
            min_angle_cos: -1.0,
            epsilon: 1e-5,
        }
    }
}

pub fn summarize_thickness(values: &[f32], threshold_mm: f64) -> ThicknessSummary {
    let mut valid_vertex_count = 0_usize;
    let mut violation_count = 0_usize;
    let mut min_mm = f64::INFINITY;
    let mut max_mm = f64::NEG_INFINITY;
    let mut sum = 0.0_f64;

    for value in values {
        let thickness = f64::from(*value);
        if !thickness.is_finite() || thickness <= 0.0 {
            continue;
        }
        valid_vertex_count += 1;
        if thickness < threshold_mm {
            violation_count += 1;
        }
        min_mm = min_mm.min(thickness);
        max_mm = max_mm.max(thickness);
        sum += thickness;
    }

    if valid_vertex_count == 0 {
        return ThicknessSummary {
            min_mm: None,
            avg_mm: None,
            max_mm: None,
            valid_vertex_count: 0,
            violation_count: 0,
        };
    }

    ThicknessSummary {
        min_mm: Some(min_mm),
        avg_mm: Some(sum / valid_vertex_count as f64),
        max_mm: Some(max_mm),
        valid_vertex_count,
        violation_count,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScalarOverlayPayload {
    pub overlay_type: String,
    pub values: Vec<f64>,
    pub min_value: f64,
    pub max_value: f64,
    pub center_value: f64,
    pub threshold_mm: Option<f64>,
    pub max_abs_value: f64,
    pub mean_value: f64,
    pub valid_count: usize,
}

pub fn scalar_overlay_payload(
    values: &[f32],
    overlay_type: &str,
    center_value: f64,
    threshold_mm: Option<f64>,
    max_abs_value: f64,
) -> ScalarOverlayPayload {
    let mut display_values = Vec::with_capacity(values.len());
    let mut valid_count = 0_usize;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    let mut max_abs = 0.0_f64;
    let mut sum = 0.0_f64;

    for value in values {
        let scalar = f64::from(*value);
        let is_valid = scalar.is_finite() && scalar.abs() <= max_abs_value;
        if is_valid {
            valid_count += 1;
            min_value = min_value.min(scalar);
            max_value = max_value.max(scalar);
            max_abs = max_abs.max(scalar.abs());
            sum += scalar;
            display_values.push(round_five_places(scalar));
        } else {
            display_values.push(0.0);
        }
    }

    ScalarOverlayPayload {
        overlay_type: overlay_type.to_string(),
        values: display_values,
        min_value: if valid_count == 0 {
            0.0
        } else {
            round_five_places(min_value)
        },
        max_value: if valid_count == 0 {
            0.0
        } else {
            round_five_places(max_value)
        },
        center_value: round_five_places(center_value),
        threshold_mm: threshold_mm.map(round_five_places),
        max_abs_value: round_five_places(max_abs),
        mean_value: if valid_count == 0 {
            0.0
        } else {
            round_five_places(sum / valid_count as f64)
        },
        valid_count,
    }
}

fn round_five_places(value: f64) -> f64 {
    (value * 100_000.0).round() / 100_000.0
}

pub fn insphere_thickness_at_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    options: InSphereThicknessOptions,
) -> Result<Vec<f32>, GeometryError> {
    validate_insphere_options(options)?;
    let faces = validate_faces(faces_i64, vertices.len())?;
    if vertices.is_empty() || faces.is_empty() {
        return Ok(vec![0.0; vertices.len()]);
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let normals = vertex_normals(vertices.len(), &triangles, &faces);
    let incident_faces = vertex_face_adjacency(vertices.len(), &faces);
    let neighbors = vertex_neighbor_list(vertices.len(), &faces);

    let thickness = vertices
        .par_iter()
        .enumerate()
        .map(|(vertex_id, vertex)| {
            let normal = normals[vertex_id];
            if norm(normal) < 1e-8 {
                return f32::NAN;
            }

            let mut best = f64::INFINITY;
            for direction in [scale(normal, -1.0), normal] {
                if let Some(radius) = find_insphere_radius_for_direction(
                    *vertex,
                    direction,
                    vertices,
                    &triangles,
                    &bvh,
                    &neighbors[vertex_id],
                    &incident_faces[vertex_id],
                    options,
                ) {
                    best = best.min(radius * 2.0);
                }
            }

            if best.is_finite() && best > 0.0 {
                best as f32
            } else {
                f32::NAN
            }
        })
        .collect();
    Ok(thickness)
}

pub fn service_thickness_at_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    options: InSphereThicknessOptions,
) -> Result<Vec<f32>, GeometryError> {
    let insphere = insphere_thickness_at_vertices(vertices, faces_i64, options)?;
    let ray = ray_thickness_at_vertices(vertices, faces_i64, options.epsilon)?;
    Ok(insphere
        .into_iter()
        .zip(ray)
        .map(|(insphere_value, ray_value)| {
            let ray_is_valid = ray_value.is_finite() && ray_value > 0.0 && ray_value <= 1e10;
            let combined = if ray_is_valid {
                f64::from(insphere_value).min(ray_value)
            } else {
                f64::from(insphere_value)
            };
            if combined.is_finite() && combined > 0.0 {
                combined as f32
            } else {
                f32::NAN
            }
        })
        .collect())
}

pub fn nearest_surface_distances(
    source_vertices: &[[f64; 3]],
    target_vertices: &[[f64; 3]],
    target_faces: &[[i64; 3]],
) -> Result<Vec<f32>, GeometryError> {
    point_mesh_distances(source_vertices, target_vertices, target_faces).map(|distances| {
        distances
            .into_iter()
            .map(|distance| distance as f32)
            .collect()
    })
}

fn validate_insphere_options(options: InSphereThicknessOptions) -> Result<(), GeometryError> {
    if !options.max_radius.is_finite() || options.max_radius <= 0.0 {
        return Err(GeometryError::InvalidThicknessInput {
            field: "max_radius",
            value: options.max_radius,
        });
    }
    if options.max_iters == 0 {
        return Err(GeometryError::InvalidThicknessInput {
            field: "max_iters",
            value: options.max_iters as f64,
        });
    }
    if !options.min_shrinkage.is_finite()
        || options.min_shrinkage <= 0.0
        || options.min_shrinkage >= 1.0
    {
        return Err(GeometryError::InvalidThicknessInput {
            field: "min_shrinkage",
            value: options.min_shrinkage,
        });
    }
    if !options.epsilon.is_finite() || options.epsilon <= 0.0 {
        return Err(GeometryError::InvalidThicknessInput {
            field: "epsilon",
            value: options.epsilon,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn find_insphere_radius_for_direction(
    anchor: [f64; 3],
    direction: [f64; 3],
    vertices: &[[f64; 3]],
    triangles: &[[[f64; 3]; 3]],
    bvh: &FlatBvh,
    neighbors: &[usize],
    incident_faces: &[i64],
    options: InSphereThicknessOptions,
) -> Option<f64> {
    if norm(direction) < 1e-8 {
        return None;
    }
    let mut radius = options.max_radius;
    for neighbor in neighbors {
        if let Some(candidate_radius) =
            candidate_insphere_radius(anchor, direction, vertices[*neighbor], radius, options)
        {
            radius = candidate_radius;
        }
    }

    for _ in 0..options.max_iters {
        let previous = radius;
        let center = add(anchor, scale(direction, radius));
        let candidate = closest_point_excluding_incident(center, bvh, triangles, incident_faces)?;
        let Some(candidate_radius) =
            candidate_insphere_radius(anchor, direction, candidate, radius, options)
        else {
            break;
        };
        radius = candidate_radius;
        if radius <= options.epsilon || radius > previous * options.min_shrinkage {
            break;
        }
    }
    Some(radius)
}

fn candidate_insphere_radius(
    anchor: [f64; 3],
    direction: [f64; 3],
    candidate: [f64; 3],
    current_radius: f64,
    options: InSphereThicknessOptions,
) -> Option<f64> {
    let delta = sub(candidate, anchor);
    let along = dot(direction, delta);
    if along <= options.epsilon {
        return None;
    }
    let radius = dot(delta, delta) / (2.0 * along);
    if !radius.is_finite() || radius <= options.epsilon || radius >= current_radius {
        return None;
    }
    if options.min_angle_cos > -1.0 {
        let center = add(anchor, scale(direction, radius));
        let touch_direction = sub(candidate, center);
        let touch_norm = norm(touch_direction);
        if touch_norm <= options.epsilon {
            return None;
        }
        if dot(direction, scale(touch_direction, 1.0 / touch_norm)) < options.min_angle_cos {
            return None;
        }
    }
    Some(radius)
}

pub fn nearest_vertex_distances(
    source_vertices: &[[f64; 3]],
    target_vertices: &[[f64; 3]],
) -> Vec<f32> {
    if source_vertices.is_empty() || target_vertices.is_empty() {
        return vec![0.0; source_vertices.len()];
    }
    source_vertices
        .par_iter()
        .map(|source| {
            target_vertices
                .iter()
                .map(|target| norm(sub(*source, *target)))
                .fold(f64::INFINITY, f64::min) as f32
        })
        .collect()
}

pub fn supports_winding_sign(
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
    if should_reject && !self_intersecting_faces(vertices, faces_i64, epsilon)?.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub fn signed_surface_distances(
    source_vertices: &[[f64; 3]],
    target_vertices: &[[f64; 3]],
    target_faces: &[[i64; 3]],
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> Result<Vec<f32>, GeometryError> {
    if supports_winding_sign(
        target_vertices,
        target_faces,
        reject_self_intersections,
        max_self_intersection_faces,
        epsilon,
    )? {
        signed_point_mesh_distances(
            source_vertices,
            target_vertices,
            target_faces,
            winding_threshold,
        )
        .map(|distances| {
            distances
                .into_iter()
                .map(|distance| distance as f32)
                .collect()
        })
    } else {
        nearest_surface_distances(source_vertices, target_vertices, target_faces)
    }
}

pub fn version_compare_distances(
    source_vertices: &[[f64; 3]],
    target_vertices: &[[f64; 3]],
    target_faces: &[[i64; 3]],
    options: SignedCompareOptions,
) -> Result<Vec<f32>, GeometryError> {
    let distances = signed_surface_distances(
        source_vertices,
        target_vertices,
        target_faces,
        options.winding_threshold,
        options.reject_self_intersections,
        options.max_self_intersection_faces,
        options.epsilon,
    )?;
    let (source_min, source_max) = mesh_bounds(source_vertices);
    let (target_min, target_max) = mesh_bounds(target_vertices);
    let source_bbox = sub(source_max, source_min);
    let target_bbox = sub(target_max, target_min);
    let max_expected_distance = norm(source_bbox).max(norm(target_bbox)).max(1.0) * 4.0;

    Ok(distances
        .into_iter()
        .map(|distance| {
            let value = f64::from(distance);
            if value.is_finite() && value.abs() <= max_expected_distance {
                distance
            } else {
                f32::NAN
            }
        })
        .collect())
}

pub fn service_compare_distances(
    source_vertices: &[[f64; 3]],
    source_faces: &[[i64; 3]],
    other_vertices: &[[f64; 3]],
    options: SignedCompareOptions,
) -> Result<Vec<f32>, GeometryError> {
    version_compare_distances(other_vertices, source_vertices, source_faces, options)
}

pub fn summarize_distances(values: &[f32], finite_only: bool) -> DistanceSummary {
    let mut count = 0_usize;
    let mut min_mm = f64::INFINITY;
    let mut max_mm = f64::NEG_INFINITY;
    let mut sum = 0.0_f64;

    for value in values {
        let distance = f64::from(*value);
        if finite_only && !distance.is_finite() {
            continue;
        }
        count += 1;
        min_mm = min_mm.min(distance);
        max_mm = max_mm.max(distance);
        sum += distance;
    }

    if count == 0 {
        return DistanceSummary {
            min_mm: None,
            max_mm: None,
            mean_mm: None,
        };
    }

    DistanceSummary {
        min_mm: Some(min_mm),
        max_mm: Some(max_mm),
        mean_mm: Some(sum / count as f64),
    }
}

pub fn compare_summary(
    source_vertices: &[[f64; 3]],
    target_vertices: &[[f64; 3]],
    target_faces: &[[i64; 3]],
) -> Result<DistanceSummary, GeometryError> {
    let distances = nearest_surface_distances(source_vertices, target_vertices, target_faces)?;
    Ok(summarize_distances(&distances, false))
}

pub fn signed_compare_summary(
    source_vertices: &[[f64; 3]],
    target_vertices: &[[f64; 3]],
    target_faces: &[[i64; 3]],
    winding_threshold: f64,
    reject_self_intersections: bool,
    max_self_intersection_faces: Option<usize>,
    epsilon: f64,
) -> Result<DistanceSummary, GeometryError> {
    let distances = signed_surface_distances(
        source_vertices,
        target_vertices,
        target_faces,
        winding_threshold,
        reject_self_intersections,
        max_self_intersection_faces,
        epsilon,
    )?;
    Ok(summarize_distances(&distances, true))
}

pub fn version_compare_summary(
    source_vertices: &[[f64; 3]],
    source_faces: &[[i64; 3]],
    target_vertices: &[[f64; 3]],
    target_faces: &[[i64; 3]],
    options: SignedCompareOptions,
) -> Result<VersionCompareSummary, GeometryError> {
    let source_stats = mesh_stats(source_vertices, source_faces)?;
    let target_stats = mesh_stats(target_vertices, target_faces)?;
    let signed_distances =
        version_compare_distances(source_vertices, target_vertices, target_faces, options)?;
    let signed_summary = summarize_distances(&signed_distances, true);
    Ok(VersionCompareSummary {
        volume_delta_mm3: source_stats.volume_mm3 - target_stats.volume_mm3,
        bbox_delta_mm: [
            source_stats.bbox_size[0] - target_stats.bbox_size[0],
            source_stats.bbox_size[1] - target_stats.bbox_size[1],
            source_stats.bbox_size[2] - target_stats.bbox_size[2],
        ],
        min_signed_distance_mm: signed_summary.min_mm,
        max_signed_distance_mm: signed_summary.max_mm,
        mean_signed_distance_mm: signed_summary.mean_mm,
    })
}

pub fn service_compare_summary(
    source_vertices: &[[f64; 3]],
    source_faces: &[[i64; 3]],
    other_vertices: &[[f64; 3]],
    other_faces: &[[i64; 3]],
    options: SignedCompareOptions,
) -> Result<VersionCompareSummary, GeometryError> {
    let source_stats = mesh_stats(source_vertices, source_faces)?;
    let other_stats = mesh_stats(other_vertices, other_faces)?;
    let signed_distances =
        service_compare_distances(source_vertices, source_faces, other_vertices, options)?;
    let signed_summary = summarize_distances(&signed_distances, true);
    Ok(VersionCompareSummary {
        volume_delta_mm3: source_stats.volume_mm3 - other_stats.volume_mm3,
        bbox_delta_mm: [
            source_stats.bbox_size[0] - other_stats.bbox_size[0],
            source_stats.bbox_size[1] - other_stats.bbox_size[1],
            source_stats.bbox_size[2] - other_stats.bbox_size[2],
        ],
        min_signed_distance_mm: signed_summary.min_mm,
        max_signed_distance_mm: signed_summary.max_mm,
        mean_signed_distance_mm: signed_summary.mean_mm,
    })
}
