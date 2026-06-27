use crate::distance::nearest_distances_to_validated_indices;
use crate::materials::{grams_to_mm3, mm3_to_grams};
use crate::math::{add, cross, dot, norm, scale, sub};
use crate::mesh::{bounds, mesh_surface_area, mesh_volume, validate_faces};
use crate::spatial::ray_thickness_at_vertices;
use crate::{
    default_boolean_origin_phase, voxel_boolean_mesh, AdaptiveHollowResult, GeometryError,
    MeshArrays, SdfBooleanOperation, VoxelBooleanMeshOptions, VoxelMeshOptions,
};
use std::collections::{BTreeMap, BTreeSet};

mod adaptive;
mod drain;
#[cfg(test)]
mod tests;

pub use adaptive::{adaptive_hollow_to_weight, adaptive_protected_hollow_to_weight};
pub(crate) use adaptive::{
    region_index_by_id, validate_region_offsets, validate_region_vertex_index,
};
pub use drain::{drain_hole_cutter_mesh, drain_hole_cutters_mesh, plan_drain_holes};

struct AdaptiveHollowContext<'a> {
    target_weight_g: f64,
    material: &'a str,
    tolerance_g: f64,
    min_thickness_mm: f64,
    max_thickness_mm: f64,
    max_iterations: usize,
    original_weight_g: f64,
}

struct AdaptiveHollowRequest<'a> {
    target_weight_g: f64,
    material: &'a str,
    tolerance_g: f64,
    min_thickness_mm: f64,
    max_thickness_mm: f64,
    max_iterations: usize,
}

pub fn protected_hollow_scale_field(
    vertices: &[[f64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    base_thickness_mm: f64,
) -> Result<Vec<f32>, GeometryError> {
    let falloff_mm = (base_thickness_mm * 3.5).max(1.5);
    protected_hollow_scale_field_with_falloff(
        vertices,
        region_ids,
        vertex_offsets,
        vertex_indices,
        protect_region_ids,
        base_thickness_mm,
        falloff_mm,
    )
}

fn protected_hollow_scale_field_with_falloff(
    vertices: &[[f64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    base_thickness_mm: f64,
    falloff_mm: f64,
) -> Result<Vec<f32>, GeometryError> {
    let mut scales = vec![1.0_f32; vertices.len()];
    if vertices.is_empty() || region_ids.is_empty() || protect_region_ids.is_empty() {
        return Ok(scales);
    }

    let ranges = validate_region_offsets(vertex_offsets, vertex_indices.len(), region_ids.len())?;
    let region_by_id = region_index_by_id(region_ids);
    let mut protected = BTreeSet::<usize>::new();
    for region_id in protect_region_ids {
        let Some(region_index) = region_by_id.get(region_id) else {
            continue;
        };
        let range = ranges[*region_index].clone();
        for index in &vertex_indices[range] {
            let vertex_index = validate_region_vertex_index(*index, vertices.len())?;
            protected.insert(vertex_index);
        }
    }

    if protected.is_empty() {
        return Ok(scales);
    }

    let protected_indices: Vec<usize> = protected.iter().copied().collect();
    let min_hollow_mm = (base_thickness_mm * 0.18).max(0.08);
    let min_scale = clamp(min_hollow_mm / base_thickness_mm.max(1e-6), 0.08, 0.45);
    let distances = nearest_distances_to_validated_indices(vertices, &protected_indices);
    for (index, distance) in distances.iter().enumerate() {
        let mut protection = (-0.5 * (distance / falloff_mm).powi(2)).exp();
        if *distance > falloff_mm * 2.75 {
            protection = 0.0;
        }
        scales[index] = clamp(1.0 - 0.92 * protection, min_scale, 1.0) as f32;
    }
    for index in protected_indices {
        scales[index] = min_scale as f32;
    }
    Ok(scales)
}

pub fn inward_directions_for_hollow(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
) -> Result<Vec<[f64; 3]>, GeometryError> {
    Ok(crate::deform::outward_directions(vertices, faces_i64)?
        .into_iter()
        .map(|direction| scale(direction, -1.0))
        .collect())
}

/// Both sides of a thin feature move toward each other during an inner offset,
/// so each side may only consume a fraction of the locally measured thickness
/// before the offset surface folds through itself.
const MAX_LOCAL_THICKNESS_FRACTION: f64 = 0.35;
const OFFSET_SMOOTHING_ITERATIONS: usize = 8;
/// Raw per-vertex normals diverge sharply across ornament creases; offsetting
/// along them folds crevice flanks into each other. Diffusing the direction
/// field first makes nearby vertices travel coherently.
const DIRECTION_SMOOTHING_ITERATIONS: usize = 30;
/// Any face whose orientation still inverts after the clamped offset gets its
/// vertex offsets halved until the fold disappears.
const FOLD_RELAXATION_PASSES: usize = 6;

pub fn weighted_inner_offset_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    wall_thickness_mm: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let scales = protected_hollow_scale_field(
        vertices,
        region_ids,
        vertex_offsets,
        vertex_indices,
        protect_region_ids,
        wall_thickness_mm,
    )?;
    weighted_inner_offset_vertices_with_scales(vertices, faces_i64, &scales, wall_thickness_mm)
}

/// Preview variant of [`weighted_inner_offset_vertices`]: protected regions are
/// frozen in place instead of receiving the shell floor offset. The voxel-shell
/// boolean path keeps the non-zero floor (it needs a closed inner surface
/// everywhere), but a deformation preview that drags 0.14 mm of travel through
/// sub-0.1 mm ornament detail shreds it.
pub fn weighted_inner_offset_preview_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    wall_thickness_mm: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    // The shell pipeline's wide Euclidean falloff saturates thin bands (the
    // outer surface of a ring sits within a wall thickness of the protected
    // inner band straight through the solid), so the preview uses a tighter
    // blend radius around protected detail.
    let falloff_mm = wall_thickness_mm.max(0.6);
    let scales = protected_hollow_scale_field_with_falloff(
        vertices,
        region_ids,
        vertex_offsets,
        vertex_indices,
        protect_region_ids,
        wall_thickness_mm,
        falloff_mm,
    )?;
    let min_hollow_mm = (wall_thickness_mm * 0.18).max(0.08);
    let floor_scale = clamp(min_hollow_mm / wall_thickness_mm.max(1e-6), 0.08, 0.45) as f32;
    let denominator = (1.0 - floor_scale).max(1e-6);
    let preview_scales: Vec<f32> = scales
        .iter()
        .map(|vertex_scale| ((vertex_scale - floor_scale) / denominator).clamp(0.0, 1.0))
        .collect();
    weighted_inner_offset_vertices_with_scales(
        vertices,
        faces_i64,
        &preview_scales,
        wall_thickness_mm,
    )
}

fn weighted_inner_offset_vertices_with_scales(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    scales: &[f32],
    wall_thickness_mm: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let inward = inward_directions_for_hollow(vertices, faces_i64)?;
    let faces = validate_faces(faces_i64, vertices.len())?;

    let (bbox_min, bbox_max) = bounds(vertices);
    let diagonal = norm(sub(bbox_max, bbox_min));
    let epsilon = (diagonal * 1e-5).clamp(1e-7, 1e-3);
    let thickness = ray_thickness_at_vertices(vertices, faces_i64, epsilon)?;

    let limits: Vec<f64> = scales
        .iter()
        .zip(thickness.iter())
        .map(|(vertex_scale, vertex_thickness)| {
            let requested = wall_thickness_mm * *vertex_scale as f64;
            if vertex_thickness.is_finite() {
                requested.min(MAX_LOCAL_THICKNESS_FRACTION * vertex_thickness)
            } else {
                requested
            }
        })
        .collect();

    let mut neighbors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); vertices.len()];
    for face in &faces {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            neighbors[first].insert(second);
            neighbors[second].insert(first);
        }
    }

    let mut directions = inward;
    for _ in 0..DIRECTION_SMOOTHING_ITERATIONS {
        let snapshot = directions.clone();
        for (index, neighbor_set) in neighbors.iter().enumerate() {
            if neighbor_set.is_empty() {
                continue;
            }
            let mut neighbor_sum = [0.0_f64; 3];
            for neighbor in neighbor_set {
                neighbor_sum = add(neighbor_sum, snapshot[*neighbor]);
            }
            let blended = add(
                scale(snapshot[index], 0.5),
                scale(neighbor_sum, 0.5 / neighbor_set.len() as f64),
            );
            let length = norm(blended);
            if length > 1e-12 {
                directions[index] = scale(blended, 1.0 / length);
            }
        }
    }

    let mut offsets = limits.clone();
    for _ in 0..OFFSET_SMOOTHING_ITERATIONS {
        let snapshot = offsets.clone();
        for (index, neighbor_set) in neighbors.iter().enumerate() {
            if neighbor_set.is_empty() {
                continue;
            }
            let neighbor_mean = neighbor_set
                .iter()
                .map(|neighbor| snapshot[*neighbor])
                .sum::<f64>()
                / neighbor_set.len() as f64;
            offsets[index] = (0.5 * snapshot[index] + 0.5 * neighbor_mean).min(limits[index]);
        }
    }

    let displace = |offsets: &[f64]| -> Vec<[f64; 3]> {
        vertices
            .iter()
            .zip(directions.iter())
            .zip(offsets.iter())
            .map(|((vertex, direction), offset_mm)| add(*vertex, scale(*direction, *offset_mm)))
            .collect()
    };
    let face_area_normal = |points: &[[f64; 3]], face: &[usize; 3]| -> [f64; 3] {
        cross(
            sub(points[face[1]], points[face[0]]),
            sub(points[face[2]], points[face[0]]),
        )
    };

    let mut displaced = displace(&offsets);
    for _ in 0..FOLD_RELAXATION_PASSES {
        let mut any_fold = false;
        for face in &faces {
            let original_normal = face_area_normal(vertices, face);
            let displaced_normal = face_area_normal(&displaced, face);
            if dot(original_normal, displaced_normal) <= 0.0 {
                any_fold = true;
                for vertex in face {
                    offsets[*vertex] *= 0.5;
                }
            }
        }
        if !any_fold {
            break;
        }
        displaced = displace(&offsets);
    }

    Ok(displaced)
}

/// Cavity surface for the voxel-shell difference. Unprotected zones sit
/// `wall_thickness_mm` below the outer surface (leaving a wall-thick shell).
/// Protected zones push the cavity past the local mid-thickness so the cavity
/// there collapses below voxel resolution and the material stays solid —
/// "protected" must keep material, not carve a thinner wall under the detail.
#[allow(clippy::too_many_arguments)]
pub fn weighted_inner_offset_shell_vertices(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    wall_thickness_mm: f64,
    voxel_size_mm: f64,
) -> Result<Vec<[f64; 3]>, GeometryError> {
    let falloff_mm = wall_thickness_mm.max(0.6);
    let scales = protected_hollow_scale_field_with_falloff(
        vertices,
        region_ids,
        vertex_offsets,
        vertex_indices,
        protect_region_ids,
        wall_thickness_mm,
        falloff_mm,
    )?;
    let min_hollow_mm = (wall_thickness_mm * 0.18).max(0.08);
    let floor_scale = clamp(min_hollow_mm / wall_thickness_mm.max(1e-6), 0.08, 0.45);
    let protection_denominator = (1.0 - floor_scale).max(1e-6);

    let faces = validate_faces(faces_i64, vertices.len())?;
    let (bbox_min, bbox_max) = bounds(vertices);
    let diagonal = norm(sub(bbox_max, bbox_min));
    let epsilon = (diagonal * 1e-5).clamp(1e-7, 1e-3);
    let thickness = ray_thickness_at_vertices(vertices, faces_i64, epsilon)?;

    let mut depths: Vec<f64> = scales
        .iter()
        .zip(thickness.iter())
        .map(|(vertex_scale, vertex_thickness)| {
            let protection =
                ((1.0 - *vertex_scale as f64) / protection_denominator).clamp(0.0, 1.0);
            let solid_depth = if vertex_thickness.is_finite() {
                (0.5 * vertex_thickness + 0.5 * voxel_size_mm).max(wall_thickness_mm)
            } else {
                wall_thickness_mm
            };
            wall_thickness_mm + (solid_depth - wall_thickness_mm) * protection
        })
        .collect();

    let mut neighbors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); vertices.len()];
    for face in &faces {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            neighbors[first].insert(second);
            neighbors[second].insert(first);
        }
    }

    let mut directions = inward_directions_for_hollow(vertices, faces_i64)?;
    for _ in 0..DIRECTION_SMOOTHING_ITERATIONS {
        let snapshot = directions.clone();
        for (index, neighbor_set) in neighbors.iter().enumerate() {
            if neighbor_set.is_empty() {
                continue;
            }
            let mut neighbor_sum = [0.0_f64; 3];
            for neighbor in neighbor_set {
                neighbor_sum = add(neighbor_sum, snapshot[*neighbor]);
            }
            let blended = add(
                scale(snapshot[index], 0.5),
                scale(neighbor_sum, 0.5 / neighbor_set.len() as f64),
            );
            let length = norm(blended);
            if length > 1e-12 {
                directions[index] = scale(blended, 1.0 / length);
            }
        }
    }

    for _ in 0..OFFSET_SMOOTHING_ITERATIONS {
        let snapshot = depths.clone();
        for (index, neighbor_set) in neighbors.iter().enumerate() {
            if neighbor_set.is_empty() {
                continue;
            }
            let neighbor_mean = neighbor_set
                .iter()
                .map(|neighbor| snapshot[*neighbor])
                .sum::<f64>()
                / neighbor_set.len() as f64;
            depths[index] = (0.5 * snapshot[index] + 0.5 * neighbor_mean).max(wall_thickness_mm);
        }
    }

    Ok(vertices
        .iter()
        .zip(directions.iter())
        .zip(depths.iter())
        .map(|((vertex, direction), depth_mm)| add(*vertex, scale(*direction, *depth_mm)))
        .collect())
}

#[allow(clippy::too_many_arguments)]
pub fn protected_hollow_mesh(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    wall_thickness_mm: f64,
    options: VoxelMeshOptions,
) -> Result<MeshArrays, GeometryError> {
    if !wall_thickness_mm.is_finite() || wall_thickness_mm <= 0.0 {
        return Err(GeometryError::InvalidWallThickness { wall_thickness_mm });
    }
    let inner_vertices = weighted_inner_offset_shell_vertices(
        vertices,
        faces_i64,
        region_ids,
        vertex_offsets,
        vertex_indices,
        protect_region_ids,
        wall_thickness_mm,
        options.voxel_size,
    )?;
    let voxel_size = options.voxel_size;
    let shell = voxel_boolean_mesh(
        vertices,
        faces_i64,
        &inner_vertices,
        faces_i64,
        SdfBooleanOperation::Difference,
        protected_boolean_options(options, wall_thickness_mm),
    )?;
    // The cavity pinches to nothing along the protected blend; marching cubes
    // emits voxel-scale bubble surfaces there. Drop anything that small — real
    // shell surfaces are orders of magnitude larger.
    let pruned = crate::repair_components::prune_small_components(
        &shell.vertices,
        &shell.faces,
        24.0 * voxel_size * voxel_size,
    )?;
    Ok(MeshArrays {
        vertices: pruned.vertices,
        faces: pruned.faces,
    })
}

#[allow(clippy::too_many_arguments)]
fn clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    value.max(minimum).min(maximum)
}

fn protected_boolean_options(
    options: VoxelMeshOptions,
    wall_thickness_mm: f64,
) -> VoxelBooleanMeshOptions {
    VoxelBooleanMeshOptions {
        voxel_size: options.voxel_size,
        padding_mm: Some(
            options.padding_mm.unwrap_or_else(|| {
                (wall_thickness_mm + options.voxel_size).max(options.voxel_size)
            }),
        ),
        origin_phase: default_boolean_origin_phase(),
        extractor: options.extractor,
        refine: options.refine,
    }
}

fn validate_positive_finite(field: &'static str, value: f64) -> Result<(), GeometryError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(GeometryError::InvalidAdaptiveHollowInput { field, value })
    }
}

fn no_hollow_result(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    context: &AdaptiveHollowContext<'_>,
) -> AdaptiveHollowResult {
    AdaptiveHollowResult {
        vertices: vertices.to_vec(),
        faces: faces.to_vec(),
        achieved_weight_g: round3(context.original_weight_g),
        wall_thickness_mm: None,
        iterations: 0,
        warning: Some(
            "Target weight is greater than or equal to original weight. No hollowing applied."
                .to_string(),
        ),
        original_weight_g: round3(context.original_weight_g),
        target_weight_g: round3(context.target_weight_g),
    }
}

fn adaptive_hollow_result(
    mesh: MeshArrays,
    achieved_weight_g: f64,
    wall_thickness_mm: Option<f64>,
    iterations: usize,
    warning: Option<String>,
    original_weight_g: f64,
    target_weight_g: f64,
) -> AdaptiveHollowResult {
    AdaptiveHollowResult {
        vertices: mesh.vertices,
        faces: mesh.faces,
        achieved_weight_g: round3(achieved_weight_g),
        wall_thickness_mm: wall_thickness_mm.map(round3),
        iterations,
        warning,
        original_weight_g: round3(original_weight_g),
        target_weight_g: round3(target_weight_g),
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round_ties_even() / 1000.0
}
