use super::*;
use crate::voxel_mesh_ops::{
    sample_sdf_field_for_mesh, voxel_boolean_mesh_from_left_sdf_field,
    voxel_shell_mesh_from_sdf_field, SdfField,
};

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

    let voxel_size = options.voxel_size;
    let padding = options
        .padding_mm
        .unwrap_or_else(|| (context.max_thickness_mm + voxel_size).max(voxel_size));
    let field = sample_sdf_field_for_mesh(vertices, faces, voxel_size, padding, [0.0; 3])?;
    let first_thickness = if context.max_iterations == 1 {
        Some((context.min_thickness_mm + context.max_thickness_mm) * 0.5)
    } else {
        estimate_shell_thickness_from_surface_area(vertices, faces, &context)
            .or_else(|| estimate_shell_thickness_for_target(&field, voxel_size, &context))
    };
    adaptive_hollow_search_with_initial(context, first_thickness, |current_thickness| {
        voxel_shell_mesh_from_sdf_field(&field, current_thickness, options)
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

    let boolean_options = protected_boolean_options(options, context.max_thickness_mm);
    let source_padding = boolean_options.padding_mm.unwrap_or(options.voxel_size);
    let source_field = sample_sdf_field_for_mesh(
        vertices,
        faces,
        options.voxel_size,
        source_padding,
        boolean_options.origin_phase,
    )?;
    adaptive_hollow_search_with_initial(context, None, |current_thickness| {
        protected_hollow_mesh_with_source_sdf_field(
            vertices,
            faces,
            region_ids,
            vertex_offsets,
            vertex_indices,
            protect_region_ids,
            current_thickness,
            options,
            &source_field,
            boolean_options,
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

fn adaptive_hollow_search_with_initial(
    context: AdaptiveHollowContext<'_>,
    first_thickness: Option<f64>,
    mut build_shell: impl FnMut(f64) -> Result<MeshArrays, GeometryError>,
) -> Result<AdaptiveHollowResult, GeometryError> {
    let target_weight_g = context.target_weight_g;

    let mut min_t = context.min_thickness_mm;
    let mut max_t = context.max_thickness_mm;
    let mut best_mesh: Option<MeshArrays> = None;
    let mut best_weight: Option<f64> = None;
    let mut best_thickness: Option<f64> = None;
    let mut iterations = 0_usize;

    let mut visit_candidate = |current_thickness: f64,
                               iterations: usize,
                               min_t: &mut f64,
                               max_t: &mut f64,
                               best_mesh: &mut Option<MeshArrays>,
                               best_weight: &mut Option<f64>,
                               best_thickness: &mut Option<f64>|
     -> Result<Option<AdaptiveHollowResult>, GeometryError> {
        let shell = match build_shell(current_thickness) {
            Ok(shell) => shell,
            Err(_) => {
                if (current_thickness - *min_t).abs() < f64::EPSILON {
                    *min_t = current_thickness + 0.1;
                } else {
                    *max_t = (current_thickness - 0.1).max(*min_t);
                }
                return Ok(None);
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
            *best_weight = Some(current_weight);
            *best_thickness = Some(current_thickness);
            *best_mesh = Some(shell.clone());
        }

        if (current_weight - target_weight_g).abs() < context.tolerance_g {
            return Ok(Some(adaptive_hollow_result(
                shell,
                current_weight,
                Some(current_thickness),
                iterations,
                None,
                context.original_weight_g,
                target_weight_g,
            )));
        }

        if current_weight > target_weight_g {
            *max_t = current_thickness;
        } else {
            *min_t = current_thickness;
        }
        Ok(None)
    };

    if let Some(first_thickness) = first_thickness {
        let first_thickness = first_thickness.clamp(min_t, max_t);
        iterations += 1;
        if let Some(result) = visit_candidate(
            first_thickness,
            iterations,
            &mut min_t,
            &mut max_t,
            &mut best_mesh,
            &mut best_weight,
            &mut best_thickness,
        )? {
            return Ok(result);
        }
        if iterations < context.max_iterations {
            if let Some(corrected_thickness) = linearized_shell_thickness_correction(
                best_thickness,
                best_weight,
                target_weight_g,
                min_t,
                max_t,
            ) {
                iterations += 1;
                if let Some(result) = visit_candidate(
                    corrected_thickness,
                    iterations,
                    &mut min_t,
                    &mut max_t,
                    &mut best_mesh,
                    &mut best_weight,
                    &mut best_thickness,
                )? {
                    return Ok(result);
                }
            }
        }
    }

    while iterations < context.max_iterations {
        iterations += 1;
        let current_thickness = (min_t + max_t) * 0.5;
        if let Some(result) = visit_candidate(
            current_thickness,
            iterations,
            &mut min_t,
            &mut max_t,
            &mut best_mesh,
            &mut best_weight,
            &mut best_thickness,
        )? {
            return Ok(result);
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

fn linearized_shell_thickness_correction(
    best_thickness: Option<f64>,
    best_weight: Option<f64>,
    target_weight_g: f64,
    min_t: f64,
    max_t: f64,
) -> Option<f64> {
    let thickness = best_thickness?;
    let weight = best_weight?;
    if !thickness.is_finite()
        || !weight.is_finite()
        || !target_weight_g.is_finite()
        || thickness <= 0.0
        || weight <= 0.0
        || max_t <= min_t
    {
        return None;
    }
    let corrected = (thickness * target_weight_g / weight).clamp(min_t, max_t);
    ((corrected - thickness).abs() >= 0.01).then_some(corrected)
}

#[allow(clippy::too_many_arguments)]
fn protected_hollow_mesh_with_source_sdf_field(
    vertices: &[[f64; 3]],
    faces_i64: &[[i64; 3]],
    region_ids: &[String],
    vertex_offsets: &[i64],
    vertex_indices: &[i64],
    protect_region_ids: &[String],
    wall_thickness_mm: f64,
    options: VoxelMeshOptions,
    source_field: &SdfField,
    boolean_options: VoxelBooleanMeshOptions,
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
    let shell = voxel_boolean_mesh_from_left_sdf_field(
        source_field,
        &inner_vertices,
        faces_i64,
        SdfBooleanOperation::Difference,
        boolean_options,
    )?;
    let pruned = crate::repair_components::prune_small_components(
        &shell.vertices,
        &shell.faces,
        24.0 * options.voxel_size * options.voxel_size,
    )?;
    Ok(MeshArrays {
        vertices: pruned.vertices,
        faces: pruned.faces,
    })
}

fn estimate_shell_thickness_for_target(
    field: &SdfField,
    voxel_size: f64,
    context: &AdaptiveHollowContext<'_>,
) -> Option<f64> {
    let min_weight = estimate_cached_shell_weight_g(
        field,
        voxel_size,
        context.min_thickness_mm,
        context.material,
    );
    let max_weight = estimate_cached_shell_weight_g(
        field,
        voxel_size,
        context.max_thickness_mm,
        context.material,
    );
    if !min_weight.is_finite() || !max_weight.is_finite() || max_weight <= min_weight {
        return None;
    }
    if context.target_weight_g <= min_weight {
        return Some(context.min_thickness_mm);
    }
    if context.target_weight_g >= max_weight {
        return Some(context.max_thickness_mm);
    }

    let mut min_t = context.min_thickness_mm;
    let mut max_t = context.max_thickness_mm;
    for _ in 0..24 {
        let current_t = (min_t + max_t) * 0.5;
        let current_weight =
            estimate_cached_shell_weight_g(field, voxel_size, current_t, context.material);
        if current_weight > context.target_weight_g {
            max_t = current_t;
        } else {
            min_t = current_t;
        }
    }
    Some((min_t + max_t) * 0.5)
}

fn estimate_shell_thickness_from_surface_area(
    vertices: &[[f64; 3]],
    faces: &[[i64; 3]],
    context: &AdaptiveHollowContext<'_>,
) -> Option<f64> {
    let surface_area = mesh_surface_area(vertices, faces).ok()?;
    if !surface_area.is_finite() || surface_area <= 0.0 {
        return None;
    }
    let target_volume_mm3 = grams_to_mm3(context.target_weight_g, context.material);
    if !target_volume_mm3.is_finite() || target_volume_mm3 <= 0.0 {
        return None;
    }
    Some(
        (target_volume_mm3 / surface_area)
            .clamp(context.min_thickness_mm, context.max_thickness_mm),
    )
}

fn estimate_cached_shell_weight_g(
    field: &SdfField,
    voxel_size: f64,
    wall_thickness_mm: f64,
    material: &str,
) -> f64 {
    let lower = -(wall_thickness_mm as f32);
    let occupied = field
        .values
        .iter()
        .filter(|value| value.is_finite() && **value <= 0.0 && **value >= lower)
        .count();
    let voxel_volume_mm3 = voxel_size * voxel_size * voxel_size;
    mm3_to_grams(occupied as f64 * voxel_volume_mm3, material)
}

pub(crate) fn validate_region_offsets(
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

pub(crate) fn validate_region_vertex_index(
    index: i64,
    vertex_count: usize,
) -> Result<usize, GeometryError> {
    if index < 0 || index as usize >= vertex_count {
        return Err(GeometryError::RegionVertexOutOfBounds {
            index,
            vertex_count,
        });
    }
    Ok(index as usize)
}

pub(crate) fn region_index_by_id(region_ids: &[String]) -> BTreeMap<String, usize> {
    region_ids
        .iter()
        .enumerate()
        .map(|(index, region_id)| (region_id.clone(), index))
        .collect()
}
