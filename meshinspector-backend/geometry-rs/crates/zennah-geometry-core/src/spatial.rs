use crate::grid::grid_value_count;
use crate::math::{add, norm, normalize_vector, scale};
use crate::mesh::{validate_faces, vertex_face_adjacency, vertex_normals};
use crate::{ClosestPointsResult, GeometryError, RayHit, RayHitsResult};
use rayon::prelude::*;
use std::collections::BTreeSet;
mod aabb_query;
mod bvh;
mod closest;
mod exact_boolean;
mod exact_boolean_assembly;
mod exact_boolean_candidate;
mod exact_boolean_diagnostics;
mod exact_boolean_paths;
mod exact_boolean_topology;
mod exact_classify;
mod exact_contours;
mod exact_coplanar;
mod exact_cut;
mod exact_cut_apply;
mod exact_cut_pair;
mod exact_face_split;
mod exact_fill_apply;
mod exact_fill_plan;
mod exact_halfedge;
mod exact_intersections;
mod exact_kernel;
mod exact_lone_retry;
mod exact_meshlib_near_stitch;
mod exact_meshlib_rewrite_apply;
mod exact_one_mesh;
mod exact_splice;
mod exact_splice_apply;
mod exact_splice_path;
mod exact_stitch;
mod predicates;
mod ray;
mod sign;
mod winding;
pub use aabb_query::{
    aabb_closest_candidate_faces, aabb_overlapping_face_pairs, aabb_ray_candidate_faces,
    point_aabb_distance_sq, ray_intersects_aabb, AabbQueryTree,
};
use bvh::{build_flat_bvh, overlapping_face_pairs};
use closest::{
    closest_point_on_triangle as closest_point_on_triangle_impl, closest_point_with_bvh,
    ClosestPointHit,
};
pub use exact_boolean::{
    assemble_classified_boolean, exact_assemble_boolean_from_cut_meshes, exact_boolean_from_meshes,
    ExactBooleanAssemblyResult, ExactBooleanOperand, ExactBooleanOperation,
    ExactBooleanOutputFaceSource, ExactBooleanPipelineDiagnostics, ExactBooleanPipelineResult,
    ExactBooleanStitchedEdgeSource, MeshlibFaceExportFailureDiagnostic,
    MeshlibNearStitchFailureDiagnostic, MeshlibNearStitchLinkedEdgeDiagnostic,
    MeshlibNearStitchRingDiagnostic, MeshlibNearStitchSourceLookupDiagnostic,
    MeshlibNearStitchTargetSnapshotDiagnostic, MeshlibPreparedSourceRecordReplayDiagnostic,
};
pub use exact_classify::{
    exact_classify_components, ExactComponentClassification, ExactMeshPartClassification,
};
pub use exact_contours::{
    exact_intersection_contours, ExactContourIntersection, ExactIntersectionContour,
};
pub use exact_cut::{
    exact_cut_preplan, exact_mesh_pair_cut_preplans, ExactCutPathSegment, ExactCutPoint,
    ExactCutPreplan, ExactCutPrimitive, ExactEdgeSplitEvent, ExactMeshPairCutPreplans,
};
pub use exact_cut_apply::{exact_cut_mesh_by_contours, ExactCutMeshResult};
pub use exact_cut_pair::{exact_mesh_pair_cut_meshes, ExactMeshPairCutMeshes};
pub use exact_fill_apply::{
    exact_cut_hole_fill_plans, exact_fill_cut_holes, ExactCutHoleFillPlan, ExactCutHoleFillResult,
};
pub use exact_fill_plan::{
    exact_planar_hole_fill_plan, execute_exact_planar_hole_fill_plan, ExactPlanarHoleFillExecution,
    ExactPlanarHoleFillPlan,
};
pub use exact_intersections::{exact_mesh_intersections, ExactMeshIntersection};
pub use exact_kernel::{
    orient3d_precise_sos_positive, orient3d_sign, orient3d_sos_positive, orient3d_volume,
    segment_intersection_order_rational, triangle_segment_intersection_point,
    triangle_segment_intersection_sos, triangle_triangle_intersections_sos,
    ExactCoordinateConverter, ExactPoint3, ExactSign, ExactTriangleOwner, ExactVertCoords,
    TriangleEdgeIntersection, TriangleSegmentIntersection,
};
pub use exact_one_mesh::{
    exact_one_mesh_intersection_contours, ExactOneMeshContour, ExactOneMeshContours,
    ExactOneMeshIntersection, ExactOneMeshPrimitive,
};
pub use exact_splice::{
    exact_topology_splice_plan, ExactTopologySpliceEntry, ExactTopologySplicePlan,
    ExactTopologySpliceStatus,
};
pub use exact_splice_apply::{
    exact_topology_splice_apply_plan, ExactTopologySpliceApplyEntry, ExactTopologySpliceApplyPlan,
    ExactTopologySpliceApplyStatus,
};
pub use exact_stitch::{
    exact_stitch_plan_by_edges, exact_stitch_plan_from_cut_meshes, ExactStitchEdgePair,
    ExactStitchPath, ExactStitchPlan,
};
use predicates::{faces_share_vertex, triangles_intersect as triangles_intersect_predicate};
use ray::{first_ray_hit_with_bvh, ray_hits_with_bvh};
pub use sign::{
    point_inside_mesh, point_inside_mesh_winding, signed_point_mesh_distances_with_method,
    supports_winding_sign_for_mesh,
};
use winding::triangle_solid_angle;
pub fn self_intersecting_faces(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    epsilon: f64,
) -> Result<Vec<usize>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.len() < 2 {
        return Ok(Vec::new());
    }
    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let candidate_pairs = overlapping_face_pairs(&bvh, epsilon);
    let mut intersecting = BTreeSet::new();
    for (face_a, face_b) in candidate_pairs {
        if faces_share_vertex(faces[face_a], faces[face_b]) {
            continue;
        }
        if triangles_intersect_predicate(triangles[face_a], triangles[face_b], epsilon) {
            intersecting.insert(face_a);
            intersecting.insert(face_b);
        }
    }

    Ok(intersecting.into_iter().collect())
}

pub fn triangles_intersect(
    triangle_a: [[f64; 3]; 3],
    triangle_b: [[f64; 3]; 3],
    epsilon: f64,
) -> bool {
    triangles_intersect_predicate(triangle_a, triangle_b, epsilon)
}

pub fn closest_point_on_triangle(point: [f64; 3], triangle: [[f64; 3]; 3]) -> [f64; 3] {
    closest_point_on_triangle_impl(point, triangle)
}

pub fn closest_points_on_mesh(
    points: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<ClosestPointsResult, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(ClosestPointsResult {
            closest_points: vec![[0.0; 3]; points.len()],
            distances: vec![f64::INFINITY; points.len()],
            face_indices: vec![-1; points.len()],
        });
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);

    let hits: Vec<ClosestPointHit> = points
        .par_iter()
        .map(|point| closest_point_with_bvh(*point, &bvh, &triangles))
        .collect();

    let mut closest_points = Vec::with_capacity(hits.len());
    let mut distances = Vec::with_capacity(hits.len());
    let mut output_faces = Vec::with_capacity(hits.len());
    for closest in hits {
        closest_points.push(closest.point);
        distances.push(closest.distance_sq.sqrt());
        output_faces.push(closest.face_index);
    }

    Ok(ClosestPointsResult {
        closest_points,
        distances,
        face_indices: output_faces,
    })
}

pub fn point_mesh_distances(
    points: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<f64>, GeometryError> {
    Ok(closest_points_on_mesh(points, vertices, faces_i64)?.distances)
}

pub fn winding_numbers(
    points: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<f64>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(vec![0.0; points.len()]);
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let normalization = 4.0 * std::f64::consts::PI;
    let output = points
        .par_iter()
        .map(|point| {
            let solid_angle: f64 = triangles
                .iter()
                .map(|triangle| triangle_solid_angle(*point, *triangle))
                .sum();
            solid_angle / normalization
        })
        .collect();
    Ok(output)
}

pub fn signed_point_mesh_distances(
    points: &[[f64; 3]],
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    winding_threshold: f64,
) -> Result<Vec<f64>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(vec![f64::INFINITY; points.len()]);
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let normalization = 4.0 * std::f64::consts::PI;

    let output = points
        .par_iter()
        .map(|point| {
            let closest = closest_point_with_bvh(*point, &bvh, &triangles);
            let winding = triangles
                .iter()
                .map(|triangle| triangle_solid_angle(*point, *triangle))
                .sum::<f64>()
                / normalization;
            let sign = if winding.abs() >= winding_threshold {
                -1.0
            } else {
                1.0
            };
            closest.distance_sq.sqrt() * sign
        })
        .collect();
    Ok(output)
}

pub fn ray_thickness_at_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    epsilon: f64,
) -> Result<Vec<f64>, GeometryError> {
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
    let adjacent_faces = vertex_face_adjacency(vertices.len(), &faces);

    let thickness = vertices
        .par_iter()
        .enumerate()
        .map(|(vertex_id, vertex)| {
            let normal = normals[vertex_id];
            if norm(normal) < 1e-8 {
                return f64::NAN;
            }

            let mut best = f64::INFINITY;
            for direction in [normal, scale(normal, -1.0)] {
                let origin = add(*vertex, scale(direction, epsilon));
                let hit = first_ray_hit_with_bvh(
                    origin,
                    direction,
                    &bvh,
                    &triangles,
                    epsilon * 0.1,
                    &adjacent_faces[vertex_id],
                );
                if let Some(ray_hit) = hit {
                    if ray_hit.distance.is_finite() {
                        best = best.min(ray_hit.distance + epsilon);
                    }
                }
            }
            if best.is_finite() {
                best
            } else {
                f64::NAN
            }
        })
        .collect();

    Ok(thickness)
}

pub fn sdf_grid_values(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origin: [f64; 3],
    shape: [usize; 3],
    voxel_size: f64,
    winding_threshold: f64,
) -> Result<Vec<f32>, GeometryError> {
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(GeometryError::InvalidVoxelSize { voxel_size });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    let total = grid_value_count(shape)?;
    if faces.is_empty() {
        return Ok(vec![f32::INFINITY; total]);
    }

    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    let normalization = 4.0 * std::f64::consts::PI;
    let yz_plane = shape[1] * shape[2];
    let output = (0..total)
        .into_par_iter()
        .map(|index| {
            let ix = index / yz_plane;
            let remainder = index % yz_plane;
            let iy = remainder / shape[2];
            let iz = remainder % shape[2];
            let point = [
                origin[0] + ix as f64 * voxel_size,
                origin[1] + iy as f64 * voxel_size,
                origin[2] + iz as f64 * voxel_size,
            ];
            let closest = closest_point_with_bvh(point, &bvh, &triangles);
            let winding = triangles
                .iter()
                .map(|triangle| triangle_solid_angle(point, *triangle))
                .sum::<f64>()
                / normalization;
            let sign = if winding.abs() >= winding_threshold {
                -1.0
            } else {
                1.0
            };
            (closest.distance_sq.sqrt() * sign) as f32
        })
        .collect();

    Ok(output)
}

pub fn first_ray_hit(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origin: [f64; 3],
    direction: [f64; 3],
    epsilon: f64,
    ignored_faces: &[i64],
) -> Result<Option<RayHit>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(None);
    }

    let ray_direction = normalize_vector(direction)?;
    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);

    let hit = first_ray_hit_with_bvh(
        origin,
        ray_direction,
        &bvh,
        &triangles,
        epsilon,
        ignored_faces,
    );
    Ok(hit)
}

pub fn ray_triangle_hits(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origin: [f64; 3],
    direction: [f64; 3],
    epsilon: f64,
    ignored_faces: &[i64],
) -> Result<Vec<RayHit>, GeometryError> {
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(Vec::new());
    }

    let ray_direction = normalize_vector(direction)?;
    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);
    Ok(ray_hits_with_bvh(
        origin,
        ray_direction,
        &bvh,
        &triangles,
        epsilon,
        ignored_faces,
    ))
}

pub fn first_ray_hits(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    origins: &[[f64; 3]],
    directions: &[[f64; 3]],
    epsilon: f64,
    ignored_faces: &[i64],
) -> Result<RayHitsResult, GeometryError> {
    if origins.len() != directions.len() {
        return Err(GeometryError::RayCountMismatch {
            origins: origins.len(),
            directions: directions.len(),
        });
    }
    let faces = validate_faces(faces_i64, vertices.len())?;
    if faces.is_empty() {
        return Ok(RayHitsResult {
            face_indices: vec![-1; origins.len()],
            distances: vec![f64::INFINITY; origins.len()],
            points: vec![[f64::NAN; 3]; origins.len()],
        });
    }

    let ray_directions = directions
        .iter()
        .map(|direction| normalize_vector(*direction))
        .collect::<Result<Vec<_>, _>>()?;
    let triangles: Vec<[[f64; 3]; 3]> = faces
        .iter()
        .map(|face| [vertices[face[0]], vertices[face[1]], vertices[face[2]]])
        .collect();
    let bvh = build_flat_bvh(&triangles, 16);

    let hits: Vec<Option<RayHit>> = origins
        .par_iter()
        .zip(ray_directions.par_iter())
        .map(|(origin, direction)| {
            first_ray_hit_with_bvh(
                *origin,
                *direction,
                &bvh,
                &triangles,
                epsilon,
                ignored_faces,
            )
        })
        .collect();
    let mut face_indices = Vec::with_capacity(hits.len());
    let mut distances = Vec::with_capacity(hits.len());
    let mut points = Vec::with_capacity(hits.len());
    for hit in hits {
        if let Some(ray_hit) = hit {
            face_indices.push(ray_hit.face_index as i64);
            distances.push(ray_hit.distance);
            points.push(ray_hit.point);
        } else {
            face_indices.push(-1);
            distances.push(f64::INFINITY);
            points.push([f64::NAN; 3]);
        }
    }

    Ok(RayHitsResult {
        face_indices,
        distances,
        points,
    })
}
