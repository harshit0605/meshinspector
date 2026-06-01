use crate::distance::nearest_distances_to_validated_indices;
use crate::materials::mm3_to_grams;
use crate::math::{add, cross, dot, norm, normalize_vector, scale, sub};
use crate::mesh::{bounds, mesh_volume};
use crate::{
    default_boolean_origin_phase, voxel_boolean_mesh, voxel_shell_mesh, AdaptiveHollowResult,
    DrainHolePlan, GeometryError, MeshArrays, SdfBooleanOperation, VoxelBooleanMeshOptions,
    VoxelMeshOptions,
};
use std::collections::{BTreeMap, BTreeSet};

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
    let falloff_mm = (base_thickness_mm * 3.5).max(1.5);
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
    let inward = inward_directions_for_hollow(vertices, faces_i64)?;
    Ok(vertices
        .iter()
        .zip(inward.iter())
        .zip(scales.iter())
        .map(|((vertex, direction), vertex_scale)| {
            add(
                *vertex,
                scale(*direction, wall_thickness_mm * *vertex_scale as f64),
            )
        })
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
    let inner_vertices = weighted_inner_offset_vertices(
        vertices,
        faces_i64,
        region_ids,
        vertex_offsets,
        vertex_indices,
        protect_region_ids,
        wall_thickness_mm,
    )?;
    voxel_boolean_mesh(
        vertices,
        faces_i64,
        &inner_vertices,
        faces_i64,
        SdfBooleanOperation::Difference,
        protected_boolean_options(options, wall_thickness_mm),
    )
}

pub fn plan_drain_holes(
    vertices: &[[f64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    ring_axis: [f64; 3],
    wall_thickness_mm: f64,
    hole_diameter_mm: f64,
) -> Result<Vec<DrainHolePlan>, GeometryError> {
    let ranges = validate_region_offsets(vertex_offsets, vertex_indices.len(), region_ids.len())?;
    let region_by_id = region_index_by_id(region_ids);
    let inner_range = region_by_id
        .get("inner_band")
        .map(|region_index| ranges[*region_index].clone())
        .ok_or(GeometryError::MissingInnerBandRegion)?;
    if inner_range.is_empty() {
        return Err(GeometryError::MissingInnerBandRegion);
    }

    let center = centroid(vertices);
    let axis = normalize_vector(ring_axis)?;
    let mut inner_vertices = Vec::with_capacity(inner_range.len());
    for index in &vertex_indices[inner_range] {
        inner_vertices.push(vertices[validate_region_vertex_index(*index, vertices.len())?]);
    }

    let mut valid_dirs = Vec::new();
    let mut valid_vertices = Vec::new();
    for vertex in &inner_vertices {
        let centered = sub(*vertex, center);
        let radial = sub(centered, scale(axis, dot(centered, axis)));
        let radial_norm = norm(radial);
        if radial_norm > 1e-6 {
            valid_dirs.push(scale(radial, 1.0 / radial_norm));
            valid_vertices.push(*vertex);
        }
    }
    if valid_dirs.is_empty() {
        return Err(GeometryError::DrainHoleDirectionsUnavailable);
    }

    let mut radial_basis = scale(
        valid_dirs.iter().copied().fold([0.0_f64; 3], add),
        1.0 / valid_dirs.len() as f64,
    );
    if norm(radial_basis) < 1e-6 {
        radial_basis = valid_dirs[0];
    }
    radial_basis = normalize_vector(radial_basis)?;

    let (bbox_min, bbox_max) = bounds(vertices);
    let bbox_size = [
        bbox_max[0] - bbox_min[0],
        bbox_max[1] - bbox_min[1],
        bbox_max[2] - bbox_min[2],
    ];
    let max_bbox_size = bbox_size.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let length = clamp(
        max_bbox_size * 0.18,
        (wall_thickness_mm * 5.0).max(3.0),
        8.0,
    );

    let mut plans = Vec::with_capacity(2);
    for basis in [radial_basis, scale(radial_basis, -1.0)] {
        let (anchor, direction) =
            pick_drain_anchor(&valid_vertices, &valid_dirs, center, axis, basis)?;
        let center_point = add(anchor, scale(direction, wall_thickness_mm * 0.55));
        plans.push(DrainHolePlan {
            center_mm: center_point,
            direction,
            radius_mm: hole_diameter_mm / 2.0,
            length_mm: length,
        });
    }
    Ok(plans)
}

pub fn drain_hole_cutter_mesh(
    plan: DrainHolePlan,
    sections: usize,
) -> Result<MeshArrays, GeometryError> {
    if sections < 8 {
        return Err(GeometryError::InvalidDrainHoleSections { sections });
    }

    let direction = normalize_vector(plan.direction)?;
    let mut helper = [0.0, 1.0, 0.0];
    if dot(direction, helper).abs() > 0.92 {
        helper = [1.0, 0.0, 0.0];
    }
    let tangent_u = normalize_vector(cross(direction, helper))?;
    let tangent_v = normalize_vector(cross(direction, tangent_u))?;
    let half = scale(direction, plan.length_mm / 2.0);
    let start = sub(plan.center_mm, half);
    let end = add(plan.center_mm, half);

    let mut vertices = Vec::with_capacity(sections * 2 + 2);
    for base in [start, end] {
        for index in 0..sections {
            let theta = 2.0 * std::f64::consts::PI * index as f64 / sections as f64;
            vertices.push(add(
                base,
                add(
                    scale(tangent_u, plan.radius_mm * theta.cos()),
                    scale(tangent_v, plan.radius_mm * theta.sin()),
                ),
            ));
        }
    }
    let start_center = vertices.len() as i64;
    vertices.push(start);
    let end_center = vertices.len() as i64;
    vertices.push(end);

    let mut faces = Vec::with_capacity(sections * 4);
    for index in 0..sections {
        let next = (index + 1) % sections;
        let a = index as i64;
        let b = next as i64;
        let c = (sections + next) as i64;
        let d = (sections + index) as i64;
        faces.push([a, b, c]);
        faces.push([a, c, d]);
        faces.push([start_center, b, a]);
        faces.push([end_center, d, c]);
    }

    Ok(MeshArrays { vertices, faces })
}

pub fn drain_hole_cutters_mesh(
    plans: &[DrainHolePlan],
    sections: usize,
) -> Result<MeshArrays, GeometryError> {
    if plans.is_empty() {
        return Ok(MeshArrays {
            vertices: Vec::new(),
            faces: Vec::new(),
        });
    }

    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut offset = 0_i64;
    for plan in plans {
        let cutter = drain_hole_cutter_mesh(plan.clone(), sections)?;
        vertices.extend(cutter.vertices);
        faces.extend(
            cutter
                .faces
                .into_iter()
                .map(|face| [face[0] + offset, face[1] + offset, face[2] + offset]),
        );
        offset = vertices.len() as i64;
    }
    Ok(MeshArrays { vertices, faces })
}

#[allow(clippy::too_many_arguments)]
pub fn adaptive_hollow_to_weight(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    target_weight_g: f64,
    material: &str,
    tolerance_g: f64,
    min_thickness_mm: f64,
    max_thickness_mm: f64,
    max_iterations: usize,
    options: VoxelMeshOptions,
) -> Result<AdaptiveHollowResult, GeometryError> {
    let context = adaptive_hollow_context(
        vertices,
        faces,
        AdaptiveHollowRequest {
            target_weight_g,
            material,
            tolerance_g,
            min_thickness_mm,
            max_thickness_mm,
            max_iterations,
        },
    )?;
    if target_weight_g >= context.original_weight_g {
        return Ok(no_hollow_result(vertices, faces, &context));
    }

    adaptive_hollow_search(context, |current_thickness| {
        voxel_shell_mesh(vertices, faces, current_thickness, options)
    })
}

#[allow(clippy::too_many_arguments)]
pub fn adaptive_protected_hollow_to_weight(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    target_weight_g: f64,
    material: &str,
    tolerance_g: f64,
    min_thickness_mm: f64,
    max_thickness_mm: f64,
    max_iterations: usize,
    options: VoxelMeshOptions,
) -> Result<AdaptiveHollowResult, GeometryError> {
    let context = adaptive_hollow_context(
        vertices,
        faces,
        AdaptiveHollowRequest {
            target_weight_g,
            material,
            tolerance_g,
            min_thickness_mm,
            max_thickness_mm,
            max_iterations,
        },
    )?;
    if target_weight_g >= context.original_weight_g {
        return Ok(no_hollow_result(vertices, faces, &context));
    }

    adaptive_hollow_search(context, |current_thickness| {
        protected_hollow_mesh(
            vertices,
            faces,
            region_ids,
            vertex_offsets,
            vertex_indices,
            protect_region_ids,
            current_thickness,
            options,
        )
    })
}

fn adaptive_hollow_context<'a>(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    request: AdaptiveHollowRequest<'a>,
) -> Result<AdaptiveHollowContext<'a>, GeometryError> {
    validate_positive_finite("target_weight_g", request.target_weight_g)?;
    validate_positive_finite("tolerance_g", request.tolerance_g)?;
    validate_positive_finite("min_thickness_mm", request.min_thickness_mm)?;
    validate_positive_finite("max_thickness_mm", request.max_thickness_mm)?;
    if request.max_thickness_mm < request.min_thickness_mm {
        return Err(GeometryError::InvalidAdaptiveHollowInput {
            field: "max_thickness_mm",
            value: request.max_thickness_mm,
        });
    }
    if request.max_iterations == 0 {
        return Err(GeometryError::InvalidAdaptiveHollowInput {
            field: "max_iterations",
            value: 0.0,
        });
    }
    Ok(AdaptiveHollowContext {
        target_weight_g: request.target_weight_g,
        material: request.material,
        tolerance_g: request.tolerance_g,
        min_thickness_mm: request.min_thickness_mm,
        max_thickness_mm: request.max_thickness_mm,
        max_iterations: request.max_iterations,
        original_weight_g: mm3_to_grams(mesh_volume(vertices, faces)?, request.material),
    })
}

fn adaptive_hollow_search(
    context: AdaptiveHollowContext<'_>,
    mut build_shell: impl FnMut(f64) -> Result<MeshArrays, GeometryError>,
) -> Result<AdaptiveHollowResult, GeometryError> {
    let target_weight_g = context.target_weight_g;

    let mut min_t = context.min_thickness_mm;
    let mut max_t = context.max_thickness_mm;
    let mut best_mesh: Option<MeshArrays> = None;
    let mut best_weight: Option<f64> = None;
    let mut best_thickness: Option<f64> = None;
    let mut iterations = 0_usize;

    for iteration in 0..context.max_iterations {
        iterations = iteration + 1;
        let current_thickness = (min_t + max_t) * 0.5;
        let shell = match build_shell(current_thickness) {
            Ok(shell) => shell,
            Err(_) => {
                if (current_thickness - min_t).abs() < f64::EPSILON {
                    min_t = current_thickness + 0.1;
                } else {
                    max_t = (current_thickness - 0.1).max(min_t);
                }
                continue;
            }
        };
        let current_weight = mm3_to_grams(
            mesh_volume(&shell.vertices, &shell.faces)?,
            context.material,
        );

        if best_weight
            .map(|weight| {
                (current_weight - target_weight_g).abs() < (weight - target_weight_g).abs()
            })
            .unwrap_or(true)
        {
            best_weight = Some(current_weight);
            best_thickness = Some(current_thickness);
            best_mesh = Some(shell.clone());
        }

        if (current_weight - target_weight_g).abs() < context.tolerance_g {
            return Ok(adaptive_hollow_result(
                shell,
                current_weight,
                Some(current_thickness),
                iterations,
                None,
                context.original_weight_g,
                target_weight_g,
            ));
        }

        if current_weight > target_weight_g {
            max_t = current_thickness;
        } else {
            min_t = current_thickness;
        }
        if max_t - min_t < 0.01 {
            break;
        }
    }

    let (Some(mesh), Some(weight), Some(thickness)) = (best_mesh, best_weight, best_thickness)
    else {
        return Err(GeometryError::AdaptiveHollowFailed);
    };
    let warning = if (weight - target_weight_g).abs() > context.tolerance_g {
        if weight > target_weight_g {
            Some(format!(
                "Target weight not achievable. Minimum achievable: {:.2}g",
                weight
            ))
        } else {
            Some(format!(
                "Close to target but outside tolerance. Achieved: {:.2}g",
                weight
            ))
        }
    } else {
        Some("Max iterations reached".to_string())
    };
    Ok(adaptive_hollow_result(
        mesh,
        weight,
        Some(thickness),
        iterations,
        warning,
        context.original_weight_g,
        target_weight_g,
    ))
}

fn validate_region_offsets(
    offsets: &[i64],
    index_count: usize,
    region_count: usize,
) -> Result<Vec<std::ops::Range<usize>>, GeometryError> {
    if offsets.len() != region_count + 1 {
        return Err(GeometryError::InvalidRegionOffsets {
            offsets: offsets.len(),
            regions: region_count,
        });
    }
    let mut ranges = Vec::with_capacity(region_count);
    for offset in offsets {
        if *offset < 0 || *offset as usize > index_count {
            return Err(GeometryError::InvalidRegionOffset {
                offset: *offset,
                index_count,
            });
        }
    }
    for pair in offsets.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        if next < previous {
            return Err(GeometryError::RegionOffsetsNotSorted { previous, next });
        }
        ranges.push(previous as usize..next as usize);
    }
    Ok(ranges)
}

fn validate_region_vertex_index(index: i64, vertex_count: usize) -> Result<usize, GeometryError> {
    if index < 0 || index as usize >= vertex_count {
        return Err(GeometryError::RegionVertexOutOfBounds {
            index,
            vertex_count,
        });
    }
    Ok(index as usize)
}

fn region_index_by_id(region_ids: &[String]) -> BTreeMap<String, usize> {
    region_ids
        .iter()
        .enumerate()
        .map(|(index, region_id)| (region_id.clone(), index))
        .collect()
}

fn centroid(vertices: &[[f64; 3]]) -> [f64; 3] {
    if vertices.is_empty() {
        return [0.0; 3];
    }
    scale(
        vertices.iter().copied().fold([0.0_f64; 3], add),
        1.0 / vertices.len() as f64,
    )
}

fn pick_drain_anchor(
    valid_vertices: &[[f64; 3]],
    valid_dirs: &[[f64; 3]],
    center: [f64; 3],
    axis: [f64; 3],
    direction: [f64; 3],
) -> Result<([f64; 3], [f64; 3]), GeometryError> {
    let mut best_index = 0;
    let mut best_score = f64::NEG_INFINITY;
    for (index, valid_dir) in valid_dirs.iter().enumerate() {
        let score = dot(*valid_dir, direction);
        if score > best_score {
            best_score = score;
            best_index = index;
        }
    }
    let anchor = valid_vertices[best_index];
    let radial = sub(
        sub(anchor, center),
        scale(axis, dot(sub(anchor, center), axis)),
    );
    Ok((anchor, normalize_vector(radial)?))
}

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
